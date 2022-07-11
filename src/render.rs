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

//! Rendering code.

use std::io;
use std::io::Read as _;

use palette::IntoColor;
use palette::Srgb;

use crate::color;
use crate::color::TermColor;

const ALPHABET: &[u8] =
  b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ+/";

const ALPHABET_UPPER: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

pub struct AsciiOpts {
  pub unprintable: Srgb<u8>,
  pub upper: Srgb<u8>,
  pub lower: Srgb<u8>,
  pub number: Srgb<u8>,
  pub punct: Srgb<u8>,
}

pub struct RenderOpts<'a> {
  pub log2_base: u32,
  pub bytes_per_word: u32,
  pub words_per_line: u32,
  pub little_endian: bool,
  pub display_offset_start: u64,
  pub limit: u64,

  pub gradient: Vec<Srgb<u8>>,
  pub use_truecolor: bool,
  pub color_single_glyphs: bool,
  pub ascii: Option<AsciiOpts>,
  pub uppercase: bool,

  pub row_label_style: RowLabelStyle,
  pub calc: crate::calc::Calc,

  pub r: &'a mut dyn io::Read,
  pub w: &'a mut dyn io::Write,
}

pub enum RowLabelStyle {
  None,
  Byte,
  Word,
  Line,
}

impl argh::FromArgValue for RowLabelStyle {
  fn from_arg_value(value: &str) -> Result<Self, String> {
    match value.to_lowercase().as_str() {
      "none" | "hide" | "false" => Ok(Self::None),
      "byte" | "bytes" | "true" => Ok(Self::Byte),
      "word" | "words" => Ok(Self::Word),
      "line" | "lines" => Ok(Self::Line),
      _ => Err("expected `none`, `byte`, `word`, or `line`".into()),
    }
  }
}

impl RenderOpts<'_> {
  pub fn render(&mut self) -> io::Result<()> {
    let base = 1u64 << self.log2_base;

    // lcm(base, 8) / 8
    let chunk_len = match self.log2_base {
      3 => 3,
      5 => 5,
      6 => 3,
      _ => 1,
    };
    let glyphs_per_byte = (8 * chunk_len) / self.log2_base;

    enum Colors {
      Quantized(Vec<usize>),
      True(Vec<Srgb<u8>>),
    }
    impl Colors {
      fn term_color(&self, idx: usize) -> TermColor {
        match self {
          Colors::Quantized(cs) => TermColor::Index(cs[idx] + 16),
          Colors::True(cs) => TermColor::Rgb(cs[idx]),
        }
      }
    }

    let (colors, ascii_colors) = if self.use_truecolor {
      (
        Colors::True(color::make_gradient(&self.gradient, 256)),
        self.ascii.as_ref().map(|ac| {
          Colors::True(vec![
            ac.unprintable,
            ac.upper,
            ac.lower,
            ac.number,
            ac.punct,
          ])
        }),
      )
    } else {
      let quanta = color::XTERM256_PALETTE
        .iter()
        .map(|&x| Srgb::from_components(x).into_format::<f32>().into_color())
        .collect::<Vec<_>>();
      (
        Colors::Quantized(color::make_quantized_gradient(
          &self.gradient,
          256,
          &quanta,
        )),
        self.ascii.as_ref().map(|ac| {
          Colors::Quantized(color::quantize_rgb(
            [ac.unprintable, ac.upper, ac.lower, ac.number, ac.punct],
            &quanta,
          ))
        }),
      )
    };

    let bytes_per_line = self.words_per_line * self.bytes_per_word;
    let render_ascii =
      |w: &mut dyn io::Write, ascii_buf: &mut Vec<u8>| -> io::Result<()> {
        let mut last_color = None;

        while ascii_buf.len() < bytes_per_line as usize {
          ascii_buf.push(0);
        }

        if let Some(ascii_colors) = &ascii_colors {
          TermColor::Reset.fg(w)?;
          write!(w, "  |")?;
          for &b in &*ascii_buf {
            let color = if b.is_ascii_uppercase() {
              1
            } else if b.is_ascii_lowercase() {
              2
            } else if b.is_ascii_digit() {
              3
            } else if b.is_ascii_punctuation() {
              4
            } else {
              0
            };

            if last_color != Some(color) {
              last_color = Some(color);
              ascii_colors.term_color(color).fg(w)?;
            }

            if b > 0x1f && b < 0x7f {
              write!(w, "{}", b as char)?;
            } else {
              write!(w, "·")?;
            }
          }
          TermColor::Reset.fg(w)?;
          write!(w, "|")?;
          ascii_buf.clear();
        }
        Ok(())
      };

    let file_offset = std::cell::Cell::new(self.display_offset_start);
    let mut byte_idx = 0;
    let mut word_idx = 0;
    let mut last_byte = None;
    let mut glyphs_in_line = 0;
    let mut ascii_buf = Vec::<u8>::new();
    let mut calc_stack = Vec::<u64>::new();

    let mut draw = |buf: [u8; 8],
                    buf_len: usize,
                    w: &mut dyn io::Write,
                    ascii_buf: &mut Vec<u8>|
     -> io::Result<()> {
      let mut bits = u64::from_le_bytes(buf);

      if byte_idx % self.bytes_per_word == 0 {
        if word_idx % self.words_per_line == 0 {
          if byte_idx != 0 {
            render_ascii(w, ascii_buf)?;
            write!(w, "\n")?;
            glyphs_in_line = 0;
          }
          TermColor::Reset.fg(w)?;
          last_byte = None;
          match self.row_label_style {
            RowLabelStyle::None => {}
            RowLabelStyle::Byte => write!(w, "0x{:08x}:  ", file_offset.get())?,
            RowLabelStyle::Word => write!(
              w,
              "0x{:08x}:  ",
              file_offset.get() / ((chunk_len * self.bytes_per_word) as u64)
            )?,
            RowLabelStyle::Line => write!(
              w,
              "0x{:08x}:  ",
              file_offset.get()
                / ((chunk_len * self.bytes_per_word * self.words_per_line)
                  as u64)
            )?,
          }
        } else if byte_idx != 0 {
          write!(w, " ")?;
          glyphs_in_line += 1;
        }
        word_idx += 1;
      }

      if !self.color_single_glyphs {
        let color_byte =
          self.calc.execute(bits, chunk_len * 8, &mut calc_stack);
        if last_byte != Some(color_byte) {
          last_byte = Some(color_byte);
          let max_byte = ((1u64 << (chunk_len * 8)) - 1) as f64;
          let idx = 255.0 * (color_byte as f64 / max_byte);
          colors.term_color(idx as usize).fg(w)?;
        }
      }

      for _ in 0..glyphs_per_byte {
        let glyph = bits >> (chunk_len * 8 - self.log2_base) & (base - 1);
        bits <<= self.log2_base;

        if self.color_single_glyphs {
          let color_byte =
            self.calc.execute(glyph, self.log2_base, &mut calc_stack);
          if last_byte != Some(color_byte) {
            last_byte = Some(color_byte);
            let idx = 255.0 * (color_byte as f64 / (base as f64 - 1.0));
            colors.term_color(idx as usize).fg(w)?;
          }
        }

        // Code in main() stops -u from being mixed with base64.
        let alphabet = if self.uppercase {
          ALPHABET_UPPER
        } else {
          ALPHABET
        };

        write!(w, "{}", alphabet[glyph as usize] as char)?;
        glyphs_in_line += 1;
      }

      if self.ascii.is_some() {
        ascii_buf.extend(&buf[..buf_len]);
      }

      byte_idx += 1;
      Ok(())
    };

    let mut buf = [0; 8];
    let mut octets_count = 0;
    for octet in self.r.bytes() {
      let octet = octet?;
      buf[octets_count] = octet;
      octets_count += 1;
      file_offset.set(file_offset.get() + 1);
      if octets_count != chunk_len as usize {
        continue;
      }

      if self.limit == 0 {
        break;
      }
      self.limit -= 1;

      draw(buf, octets_count, &mut self.w, &mut ascii_buf)?;
      octets_count = 0;
      buf.fill(0);
    }

    if octets_count != 0 {
      draw(buf, octets_count, &mut self.w, &mut ascii_buf)?;
    }

    if !ascii_buf.is_empty() {
      let line_len =
        self.words_per_line * self.bytes_per_word * glyphs_per_byte
          + (self.words_per_line - 1);
      for _ in glyphs_in_line..line_len as usize {
        write!(self.w, " ")?;
      }
      render_ascii(&mut self.w, &mut ascii_buf)?;
    }

    TermColor::Reset.fg(self.w)?;
    write!(self.w, "\n")
  }
}
