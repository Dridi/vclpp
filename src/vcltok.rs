/*-
 * vclpp
 * Copyright (C) 2017  Dridi Boukelmoune <dridi.boukelmoune@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

mod cli;
mod tok;

use std::io::Result;
use std::io::Write;
use std::vec::Vec;

use tok::Lexeme::*;
use tok::Tokenizer;

fn write_escaped<W: Write>(out: &mut W, s: &str) {
    s.bytes().map(|b| -> Vec<u8> {
        match b as char {
            '\\' => vec!('\\' as u8, '\\' as u8),
            '\n' => vec!('\\' as u8, 'n' as u8),
            '\r' => vec!('\\' as u8, 'r' as u8),
            '\t' => vec!('\\' as u8, 't' as u8),
            _ => vec!(b)
        }
    }).flat_map(|v| out.write(v.as_ref()).err())
      .take(1)
      .inspect(|e| cli::fail(e))
      .count();
}

fn decompose() -> Result<()> {
    let (src, mut out) = cli::parse_args()?;

    for tok in Tokenizer::new(src.chars()) {
        write!(out, "[{}...{}] ", tok.start, tok.end)?;
        match tok.lexeme {
            Bad(s) => {
                write!(out, "bad token: {}\n", s)?;
            }
            _ => {
                write!(out, "token: {:?} '", tok.lexeme)?;
                write_escaped(&mut out, &src[&tok]);
                write!(out, "'\n")?;
            }
        }
    }

    Ok(())
}

fn main() {
    match decompose() {
        Err(e) => cli::fail(e),
        _ => (),
    }
}
