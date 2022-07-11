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

//! An extremely basic RPN calculator, for determining how to color-code bytes.

#[rustfmt::skip]
pub enum Op {
  Add, Sub, Mul, Div, Rem,
  And, Or, Xor,
  Sll, Srl, Sra,
  Not, Neg,

  X, Imm(u64),
}

#[derive(Default)]
pub struct Calc(Vec<Op>);

impl Calc {
  pub fn execute(&self, x: u64, bits: u32, stack: &mut Vec<u64>) -> u64 {
    stack.clear();
    stack.push(x);
    let mask = (1 << bits) - 1;
    for op in &self.0 {
      let val = match op {
        Op::Add => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.wrapping_add(a)
        }
        Op::Sub => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.wrapping_sub(a)
        }
        Op::Mul => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.wrapping_mul(a)
        }
        Op::Div => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.checked_div(a).unwrap_or(0xff)
        }
        Op::Rem => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.checked_rem(a).unwrap_or(b)
        }

        Op::And => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b & a
        }
        Op::Or => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b | a
        }
        Op::Xor => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b ^ a
        }

        Op::Sll => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.wrapping_shl(a as u32)
        }
        Op::Srl => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          b.wrapping_shr(a as u32)
        }
        Op::Sra => {
          let a = stack.pop().unwrap_or(0);
          let b = stack.pop().unwrap_or(0);
          // Because where the sign bit is is dynamic, we need to shift in a
          // bunch of sign bits first.
          (b as i64)
            .wrapping_shl(64 - bits)
            .wrapping_shr(a as u32 + 64 - bits) as u64
        }

        Op::Neg => {
          let a = stack.pop().unwrap_or(0);
          a.wrapping_neg()
        }
        Op::Not => {
          let a = stack.pop().unwrap_or(0);
          !a
        }

        Op::X => x,
        Op::Imm(y) => *y as u64,
      };
      stack.push(val & mask);
    }
    stack.pop().unwrap_or(0)
  }
}

impl argh::FromArgValue for Calc {
  fn from_arg_value(mut value: &str) -> Result<Calc, String> {
    let mut ops = Vec::new();
    while let Some(first) = value.chars().next() {
      let op = match first {
        ' ' | '\t' | '\r' | '\n' => continue,
        '0'..='9' => {
          let mut base = 10;
          if let Some(trimmed) = value.strip_prefix("0x") {
            value = trimmed;
            base = 16;
          }

          let digits_end =
            match value.find(|c: char| !c.is_ascii_alphanumeric()) {
              Some(idx) => idx,
              None => value.len(),
            };
          let digits = &value[..digits_end];
          value = &value[digits_end..];

          let value =
            u64::from_str_radix(digits, base).map_err(|e| e.to_string())?;
          ops.push(Op::Imm(value));
          continue;
        }

        '>' | '<' => {
          let op = if let Some(rest) = value.strip_prefix("<<") {
            value = rest;
            Op::Sll
          } else if let Some(rest) = value.strip_prefix(">>>") {
            value = rest;
            Op::Sra
          } else if let Some(rest) = value.strip_prefix(">>") {
            value = rest;
            Op::Srl
          } else {
            return Err("unrecognized shift operator".into());
          };
          ops.push(op);
          continue;
        }

        '+' => Op::Add,
        '-' => Op::Sub,
        '*' => Op::Mul,
        '/' => Op::Div,
        '%' => Op::Rem,
        '~' => Op::Neg,
        '&' => Op::And,
        '|' => Op::Or,
        '^' => Op::Xor,
        '!' => Op::Not,
        'x' => Op::X,
        _ => return Err(format!("unrecognized character: {first}")),
      };

      ops.push(op);
      value = &value[1..];
    }
    Ok(Calc(ops))
  }
}
