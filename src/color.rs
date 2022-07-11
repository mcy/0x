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

//! Color utilities.

#![allow(unused)]

use std::io;

use palette::gradient::Gradient;
use palette::ColorDifference;
use palette::Hsv;
use palette::IntoColor;
use palette::Lab;
use palette::Srgb;

pub fn make_gradient(colors: &[Srgb<u8>], len: usize) -> Vec<Srgb<u8>> {
  let domain = colors
    .iter()
    .enumerate()
    .map(|(i, c)| {
      let color: Hsv = c.into_format::<f32>().into_color();
      (i as f32 / (len as f32 - 1.0), color)
    })
    .collect::<Vec<_>>();
  Gradient::with_domain(domain)
    .take(len)
    .map(|color| {
      let rgb: Srgb = color.into_color();
      rgb.into_format::<u8>()
    })
    .collect()
}

pub fn make_quantized_gradient(
  colors: &[Srgb<u8>],
  len: usize,
  palette: &[Lab],
) -> Vec<usize> {
  let domain = colors
    .iter()
    .enumerate()
    .map(|(i, c)| {
      let color: Hsv = c.into_format::<f32>().into_color();
      (i as f32 / (len as f32 - 1.0), color)
    })
    .collect::<Vec<_>>();

  quantize(Gradient::with_domain(domain).take(len), &palette)
}

pub fn quantize_rgb<I>(iter: I, palette: &[Lab]) -> Vec<usize>
where
  I: IntoIterator<Item = Srgb<u8>>,
{
  quantize(
    iter
      .into_iter()
      .map::<Hsv, _>(|c| c.into_format::<f32>().into_color()),
    palette,
  )
}

pub fn quantize<I>(iter: I, palette: &[Lab]) -> Vec<usize>
where
  I: IntoIterator,
  I::Item: IntoColor<Lab> + Copy,
{
  let mut quanta = Vec::new();
  for color in iter {
    let mut score = f32::INFINITY;
    let mut winner = 0;
    for (i, c) in palette.iter().enumerate() {
      let diff = c.get_color_difference(&color.into_color());
      if diff < score {
        score = diff;
        winner = i;
      }
    }
    quanta.push(winner);
  }
  quanta
}

/// A color that can be on a terminal.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TermColor {
  Dark(u8),
  Bright(u8),
  Index(usize),
  Rgb(Srgb<u8>),
  Reset,
}

impl TermColor {
  /// Sets the foreground on `out` to this color.
  pub fn fg(self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
    match self {
      Self::Dark(c) => write!(out, "\x1b[{}m", (c & 7) + 30),
      Self::Bright(c) => write!(out, "\x1b[{}m", (c & 7) + 90),
      Self::Index(i) => write!(out, "\x1b[38;5;{i}m"),
      Self::Rgb(c) => {
        write!(out, "\x1b[38;2;{};{};{}m", c.red, c.green, c.blue)
      }
      Self::Reset => write!(out, "\x1b[39m"),
    }
  }

  /// Sets the background on `out` to this color.
  pub fn bg(self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
    match self {
      Self::Dark(c) => write!(out, "\x1b[{}m", (c & 7) + 40),
      Self::Bright(c) => write!(out, "\x1b[{}m", (c & 7) + 100),
      Self::Index(i) => write!(out, "\x1b[48;5;{i}m"),
      Self::Rgb(c) => {
        write!(out, "\x1b[48;2;{};{};{}m", c.red, c.green, c.blue)
      }
      Self::Reset => write!(out, "\x1b[49m"),
    }
  }
}

/// The `xterm-256color` colors, as a palette.
pub const XTERM256_PALETTE: &[(u8, u8, u8)] = &[
  /*(0, 0, 0),
  (128, 0, 0),
  (0, 128, 0),
  (128, 128, 0),
  (0, 0, 128),
  (128, 0, 128),
  (0, 128, 128),
  (192, 192, 192),
  (128, 128, 128),
  (255, 0, 0),
  (0, 255, 0),
  (255, 255, 0),
  (0, 0, 255),
  (255, 0, 255),
  (0, 255, 255),
  (255, 255, 255),*/
  (0, 0, 0),
  (0, 0, 95),
  (0, 0, 135),
  (0, 0, 175),
  (0, 0, 215),
  (0, 0, 255),
  (0, 95, 0),
  (0, 95, 95),
  (0, 95, 135),
  (0, 95, 175),
  (0, 95, 215),
  (0, 95, 255),
  (0, 135, 0),
  (0, 135, 95),
  (0, 135, 135),
  (0, 135, 175),
  (0, 135, 215),
  (0, 135, 255),
  (0, 175, 0),
  (0, 175, 95),
  (0, 175, 135),
  (0, 175, 175),
  (0, 175, 215),
  (0, 175, 255),
  (0, 215, 0),
  (0, 215, 95),
  (0, 215, 135),
  (0, 215, 175),
  (0, 215, 215),
  (0, 215, 255),
  (0, 255, 0),
  (0, 255, 95),
  (0, 255, 135),
  (0, 255, 175),
  (0, 255, 215),
  (0, 255, 255),
  (95, 0, 0),
  (95, 0, 95),
  (95, 0, 135),
  (95, 0, 175),
  (95, 0, 215),
  (95, 0, 255),
  (95, 95, 0),
  (95, 95, 95),
  (95, 95, 135),
  (95, 95, 175),
  (95, 95, 215),
  (95, 95, 255),
  (95, 135, 0),
  (95, 135, 95),
  (95, 135, 135),
  (95, 135, 175),
  (95, 135, 215),
  (95, 135, 255),
  (95, 175, 0),
  (95, 175, 95),
  (95, 175, 135),
  (95, 175, 175),
  (95, 175, 215),
  (95, 175, 255),
  (95, 215, 0),
  (95, 215, 95),
  (95, 215, 135),
  (95, 215, 175),
  (95, 215, 215),
  (95, 215, 255),
  (95, 255, 0),
  (95, 255, 95),
  (95, 255, 135),
  (95, 255, 175),
  (95, 255, 215),
  (95, 255, 255),
  (135, 0, 0),
  (135, 0, 95),
  (135, 0, 135),
  (135, 0, 175),
  (135, 0, 215),
  (135, 0, 255),
  (135, 95, 0),
  (135, 95, 95),
  (135, 95, 135),
  (135, 95, 175),
  (135, 95, 215),
  (135, 95, 255),
  (135, 135, 0),
  (135, 135, 95),
  (135, 135, 135),
  (135, 135, 175),
  (135, 135, 215),
  (135, 135, 255),
  (135, 175, 0),
  (135, 175, 95),
  (135, 175, 135),
  (135, 175, 175),
  (135, 175, 215),
  (135, 175, 255),
  (135, 215, 0),
  (135, 215, 95),
  (135, 215, 135),
  (135, 215, 175),
  (135, 215, 215),
  (135, 215, 255),
  (135, 255, 0),
  (135, 255, 95),
  (135, 255, 135),
  (135, 255, 175),
  (135, 255, 215),
  (135, 255, 255),
  (175, 0, 0),
  (175, 0, 95),
  (175, 0, 135),
  (175, 0, 175),
  (175, 0, 215),
  (175, 0, 255),
  (175, 95, 0),
  (175, 95, 95),
  (175, 95, 135),
  (175, 95, 175),
  (175, 95, 215),
  (175, 95, 255),
  (175, 135, 0),
  (175, 135, 95),
  (175, 135, 135),
  (175, 135, 175),
  (175, 135, 215),
  (175, 135, 255),
  (175, 175, 0),
  (175, 175, 95),
  (175, 175, 135),
  (175, 175, 175),
  (175, 175, 215),
  (175, 175, 255),
  (175, 215, 0),
  (175, 215, 95),
  (175, 215, 135),
  (175, 215, 175),
  (175, 215, 215),
  (175, 215, 255),
  (175, 255, 0),
  (175, 255, 95),
  (175, 255, 135),
  (175, 255, 175),
  (175, 255, 215),
  (175, 255, 255),
  (215, 0, 0),
  (215, 0, 95),
  (215, 0, 135),
  (215, 0, 175),
  (215, 0, 215),
  (215, 0, 255),
  (215, 95, 0),
  (215, 95, 95),
  (215, 95, 135),
  (215, 95, 175),
  (215, 95, 215),
  (215, 95, 255),
  (215, 135, 0),
  (215, 135, 95),
  (215, 135, 135),
  (215, 135, 175),
  (215, 135, 215),
  (215, 135, 255),
  (215, 175, 0),
  (215, 175, 95),
  (215, 175, 135),
  (215, 175, 175),
  (215, 175, 215),
  (215, 175, 255),
  (215, 215, 0),
  (215, 215, 95),
  (215, 215, 135),
  (215, 215, 175),
  (215, 215, 215),
  (215, 215, 255),
  (215, 255, 0),
  (215, 255, 95),
  (215, 255, 135),
  (215, 255, 175),
  (215, 255, 215),
  (215, 255, 255),
  (255, 0, 0),
  (255, 0, 95),
  (255, 0, 135),
  (255, 0, 175),
  (255, 0, 215),
  (255, 0, 255),
  (255, 95, 0),
  (255, 95, 95),
  (255, 95, 135),
  (255, 95, 175),
  (255, 95, 215),
  (255, 95, 255),
  (255, 135, 0),
  (255, 135, 95),
  (255, 135, 135),
  (255, 135, 175),
  (255, 135, 215),
  (255, 135, 255),
  (255, 175, 0),
  (255, 175, 95),
  (255, 175, 135),
  (255, 175, 175),
  (255, 175, 215),
  (255, 175, 255),
  (255, 215, 0),
  (255, 215, 95),
  (255, 215, 135),
  (255, 215, 175),
  (255, 215, 215),
  (255, 215, 255),
  (255, 255, 0),
  (255, 255, 95),
  (255, 255, 135),
  (255, 255, 175),
  (255, 255, 215),
  (255, 255, 255),
  (8, 8, 8),
  (18, 18, 18),
  (28, 28, 28),
  (38, 38, 38),
  (48, 48, 48),
  (58, 58, 58),
  (68, 68, 68),
  (78, 78, 78),
  (88, 88, 88),
  (98, 98, 98),
  (108, 108, 108),
  (118, 118, 118),
  (128, 128, 128),
  (138, 138, 138),
  (148, 148, 148),
  (158, 158, 158),
  (168, 168, 168),
  (178, 178, 178),
  (188, 188, 188),
  (198, 198, 198),
  (208, 208, 208),
  (218, 218, 218),
  (228, 228, 228),
  (238, 238, 238),
];
