// Copyright 2022 Miguel Young de la Sota
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! ohx (`0x`) -- like xxd, but colorful!
//! 
//! We try to match as much of the xxd CLI as is reasonable, but we don't
//! promise exact compatibility.

use std::env;
use std::fs::File;
use std::io;
use std::io::Seek;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

use argh::FromArgs;

use palette::Srgb;

mod calc;
mod color;
mod render;

/// Parses an RGB hex value, or one of the named CSS colors in `palette`.
fn parse_rgb(s: &str) -> Result<Srgb<u8>, String> {
  if let Some(rgb) = palette::named::from_str(&s.to_lowercase()) {
    return Ok(rgb);
  }
  s.parse::<Srgb<u8>>().map_err(|e| e.to_string())
}

struct Gradient(Vec<Srgb<u8>>);
impl argh::FromArgValue for Gradient {
  fn from_arg_value(s: &str) -> Result<Gradient, String> {
    let well_known = match s.to_uppercase().as_str() {
      "BLUES" => Some(colorous::BLUES),
      "BLUE_GREEN" => Some(colorous::BLUE_GREEN),
      "BLUE_PURPLE" => Some(colorous::BLUE_PURPLE),
      "BROWN_GREEN" => Some(colorous::BROWN_GREEN),
      "CIVIDIS" => Some(colorous::CIVIDIS),
      "COOL" => Some(colorous::COOL),
      "CUBEHELIX" => Some(colorous::CUBEHELIX),
      "GREENS" => Some(colorous::GREENS),
      "GREEN_BLUE" => Some(colorous::GREEN_BLUE),
      "GREYS" => Some(colorous::GREYS),
      "INFERNO" => Some(colorous::INFERNO),
      "MAGMA" => Some(colorous::MAGMA),
      "ORANGES" => Some(colorous::ORANGES),
      "ORANGE_RED" => Some(colorous::ORANGE_RED),
      "PINK-GREEN" => Some(colorous::PINK_GREEN),
      "PLASMA" => Some(colorous::PLASMA),
      "PURPLES" => Some(colorous::PURPLES),
      "PURPLE-BLUE" => Some(colorous::PURPLE_BLUE),
      "PURPLE-BLUE_GREEN" => Some(colorous::PURPLE_BLUE_GREEN),
      "PURPLE-GREEN" => Some(colorous::PURPLE_GREEN),
      "PURPLE-ORANGE" => Some(colorous::PURPLE_ORANGE),
      "PURPLE-RED" => Some(colorous::PURPLE_RED),
      "RAINBOW" => Some(colorous::RAINBOW),
      "REDS" => Some(colorous::REDS),
      "RED-BLUE" => Some(colorous::RED_BLUE),
      "RED-GREY" => Some(colorous::RED_GREY),
      "RED-PURPLE" => Some(colorous::RED_PURPLE),
      "RED-YELLOW-BLUE" => Some(colorous::RED_YELLOW_BLUE),
      "RED-YELLOW-GREEN" => Some(colorous::RED_YELLOW_GREEN),
      "SINEBOW" => Some(colorous::SINEBOW),
      "SPECTRAL" => Some(colorous::SPECTRAL),
      "TURBO" => Some(colorous::TURBO),
      "VIRIDIS" => Some(colorous::VIRIDIS),
      "WARM" => Some(colorous::WARM),
      "YELLOW-GREEN" => Some(colorous::YELLOW_GREEN),
      "YELLOW-GREEN-BLUE" => Some(colorous::YELLOW_GREEN_BLUE),
      "YELLOW-ORANGE-BROWN" => Some(colorous::YELLOW_ORANGE_BROWN),
      "YELLOW-ORANGE-RED" => Some(colorous::YELLOW_ORANGE_RED),
      _ => None,
    };

    // Sample sixteen points.
    if let Some(gradient) = well_known {
      return Ok(Self(
        (0..16)
          .map(|i| {
            let colorous::Color { r, g, b } = gradient.eval_rational(i, 16);
            Srgb::new(r, g, b)
          })
          .collect(),
      ));
    }

    let mut gradient = Vec::new();
    for color in s.split(",") {
      gradient.push(parse_rgb(color)?);
    }
    Ok(Self(gradient))
  }
}

struct MaybeAscii(Option<render::AsciiOpts>);
impl argh::FromArgValue for MaybeAscii {
  fn from_arg_value(mut s: &str) -> Result<MaybeAscii, String> {
    match s.to_lowercase().as_str() {
      "none" | "false" => return Ok(MaybeAscii(None)),
      "mariana" => {
        s = "#c594c5,#5fb3b3,#FAB763,#EE6A6F,slategray";
      }
      "monokai" => {
        s = "#AE81FF,#66D9EF,#A6E22E,#F92672,slategray";
      }
      _ => {}
    }

    let split = s.split(",").collect::<Vec<_>>();
    if split.len() != 5 {
      return Err(format!("expected 5 colors, got {}", split.len()));
    }

    Ok(MaybeAscii(Some(render::AsciiOpts {
      upper: parse_rgb(split[0])?,
      lower: parse_rgb(split[1])?,
      number: parse_rgb(split[2])?,
      punct: parse_rgb(split[3])?,
      unprintable: parse_rgb(split[4])?,
    })))
  }
}

/// convert binary input to color-coded hex (or other power-of-2 bases)
#[derive(FromArgs)]
struct Eks {
  /// the base to print bytes in: 2, 4, 8, 16, 32, or 64. for bases that not
  /// powers of powers of two, a "byte" is lcm(8, log2(base)) bits wide
  #[argh(option, short = 'b', default = "16")]
  base: u32,

  /// number of "words" in a line
  #[argh(option, short = 'c')]
  cols: Option<u32>,

  /// print words as little-endian rather than big-endian. bytes are always
  /// little-endian.
  #[argh(switch, short = 'e')]
  little_endian: bool,

  /// number of bytes in a space-delimited "word"
  #[argh(option, short = 'g')]
  groups: Option<u32>,

  /// stop after a number of bytes
  #[argh(option, short = 'l', default = "u64::MAX")]
  limit: u64,

  /// add a fixed offset to the displayed file positions
  #[argh(option, short = 'o')]
  offset: Option<u64>,

  /// seek ahead of the input before decoding
  #[argh(option, short = 's', default = "0")]
  seek: i64,

  /// use uppercase letters for printing
  #[argh(switch, short = 'u')]
  uppercase: bool,

  /// print out the binary's version
  #[argh(switch, short = 'v')]
  version: bool,

  /// formula for picking which of the 256 colors to give each byte, in RPN.
  /// valid operands are x (for the byte itself), literal decimal or hex bytes,
  /// and the operators +, -, *, /, %, &, |, ^, >>, <<, >>> (arithmetic shift),
  /// ! (one's complement) and ~ (two's complement). the stack starts with x at
  /// at the top followed by infinite zeros.
  #[argh(option, short = 'x', default = "Default::default()")]
  calc: calc::Calc,

  /// colors for the ASCII render of each line of bytes. must be five
  /// comma-separated colors for uppercase, lowercase, digits, punctuation, and
  /// unprintable characters; disable with "none"
  #[argh(
    option,
    short = 'y',
    default = "argh::FromArgValue::from_arg_value(\"mariana\").unwrap()"
  )]
  ascii: MaybeAscii,

  /// comma-separated colors for the byte-coloring gradient
  #[argh(
    option,
    short = 'z',
    default = "argh::FromArgValue::from_arg_value(\"red,orangered,orange,gold,yellow,lightyellow\").unwrap()"
  )]
  gradient: Gradient,

  /// whether to color single glyphs rather than the bytes they're part of
  #[argh(switch)]
  color_single_glyphs: bool,

  /// what counter to print before each row: "bytes", "words", "lines", or
  /// "none"
  #[argh(option, default = "render::RowLabelStyle::Byte")]
  row_label_style: render::RowLabelStyle,

  /// force enable or disable truecolor, instead of detecting it
  #[argh(option)]
  force_truecolor: Option<bool>,

  /// input path to read from, and output path to write to;
  /// - (the default) means stdin/stdout
  #[argh(positional)]
  files: Vec<PathBuf>,
}

fn real_main() -> io::Result<()> {
  let eks: Eks = argh::from_env();

  if eks.version {
    eprintln!(
      "{} v{} by {}",
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_VERSION"),
      env!("CARGO_PKG_AUTHORS")
    );
    return Ok(());
  }

  let stdio = Path::new("-");
  let (mut input, mut output) = match &eks.files[..] {
    [] => (None, None),
    [inp] => ((inp != stdio).then(|| File::open(inp)).transpose()?, None),
    [inp, out] => (
      (inp != stdio).then(|| File::open(inp)).transpose()?,
      (out != stdio).then(|| File::create(out)).transpose()?,
    ),
    _files => {
      eprintln!("eks: only allow up to two file arguments");
      exit(1);
    }
  };

  let start_offset = if let Some(file) = &mut input {
    if eks.seek > 0 {
      file.seek(io::SeekFrom::Start(eks.seek as u64))?;
    } else if eks.seek < 0 {
      file.seek(io::SeekFrom::End(eks.seek))?;
    }
    file.stream_position()?
  } else {
    0
  };

  let (log2_base, bytes_per_word) = match eks.base {
    2 => (1, 1),
    4 => (2, 2),
    8 => (3, 1),
    16 => (4, 4),
    32 => (5, 1),
    64 => (6, 2),
    _ => {
      eprintln!("eks: base must be 2, 4, or 16");
      exit(1);
    }
  };

  if eks.uppercase && eks.base == 16 {
    eprintln!("eks: -u cannot be used with base 64");
    exit(1);
  }

  let bytes_per_word = eks.groups.unwrap_or(bytes_per_word);
  let words_per_line = eks.cols.unwrap_or(16) / bytes_per_word;

  let mut gradient = eks.gradient.0;
  if gradient.is_empty() {
    gradient = vec![palette::named::BEIGE];
  }

  let use_truecolor = eks
    .force_truecolor
    .unwrap_or_else(|| env::var_os("COLORTERM") == Some("truecolor".into()));

  render::RenderOpts {
    log2_base,
    bytes_per_word,
    words_per_line,
    display_offset_start: eks.offset.unwrap_or(start_offset),
    limit: eks.limit,
    little_endian: eks.little_endian,

    gradient,
    use_truecolor,
    ascii: eks.ascii.0,
    color_single_glyphs: eks.color_single_glyphs,
    uppercase: eks.uppercase,

    row_label_style: eks.row_label_style,
    calc: eks.calc,

    r: input
      .as_mut()
      .map(|f| f as &mut dyn io::Read)
      .unwrap_or(&mut io::stdin()),
    w: output
      .as_mut()
      .map(|f| f as &mut dyn io::Write)
      .unwrap_or(&mut io::stdout()),
  }
  .render()
}

fn main() {
  if let Err(e) = real_main() {
    eprintln!("eks: {}", e);
    exit(1);
  }
}
