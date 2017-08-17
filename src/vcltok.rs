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

use tok::Lexeme::*;
use tok::Tokenizer;

fn write_escaped<W: Write>(out: &mut W, s: &str) -> Result<usize> {
    s.chars()
     .flat_map(|c| c.escape_default())
     .map(|c| out.write(&[c as u8]))
     .filter(|r| r.is_err())
     .nth(0)
     .unwrap_or(Ok(0))
}

fn decompose() -> Result<()> {
    let (src, mut out) = cli::parse_args()?;

    for rctok in Tokenizer::new(src.chars()) {
        let tok = rctok.borrow();
        write!(out, "[{}...{}] ", tok.start, tok.end)?;
        match tok.lexeme {
            Bad => write!(out, "bad token: {}\n", tok.as_str())?,
            _ => {
                write!(out, "token: {:?} '", tok.lexeme)?;
                write_escaped(&mut out, tok.as_str())?;
                write!(out, "'\n")?;
            }
        }
    }

    out.flush()
}

fn main() {
    match decompose() {
        Err(e) => cli::fail(e),
        _ => (),
    }
}
