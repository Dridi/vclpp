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

mod bktchk;
mod cli;
mod declobj;
mod tok;

use std::io::Write;

use bktchk::BracketCheck;
use declobj::DeclarativeObject;
use tok::Lexeme::*;
use tok::Tokenizer;

fn main() {
    let (src, mut out) = match cli::parse_args() {
        Ok((s, o)) => (s, o),
        Err(e) => cli::fail(e),
    };

    let input = Tokenizer::new(src.chars());
    let pass0 = BracketCheck::new(input);
    let pass1 = DeclarativeObject::new(pass0);
    let pass2 = BracketCheck::new(pass1);

    for tok in pass2 {
        match tok.lexeme {
            Bad(msg) => {
                cli::fail(format!("{}, Line {}, Pos {}",
                    msg, tok.start.line, tok.start.column));
            }
            _ => match write!(out, "{}", tok.as_str()) {
                Err(e) => cli::fail(e),
                Ok(_) => (),
            }
        }
    }
}
