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
mod declobj;
mod hdrarray;
mod reqauth;
mod tok;
mod vmodalias;

use std::io::Write;

use declobj::DeclarativeObject;
use hdrarray::HeaderArray;
use reqauth::RequestAuthority;
use tok::Flow;
use tok::Lexeme::*;
use tok::Tokenizer;
use vmodalias::VmodAlias;

fn main() {
    let (src, mut out) = match cli::parse_args() {
        Ok((s, o)) => (s, o),
        Err(e) => cli::fail(e),
    };

    let input = Tokenizer::new(src.chars());
    let pass1 = DeclarativeObject::new(input);
    let pass2 = RequestAuthority::new(pass1);
    let pass3 = VmodAlias::new(pass2);
    let pass4 = HeaderArray::new(pass3);
    let vcl = Flow::new(pass4);

    for rctok in vcl {
        let tok = rctok.borrow();
        match tok.lexeme {
            Bad => {
                cli::fail(format!("{}, Line {}, Pos {}",
                    tok.as_str(), tok.start.line, tok.start.column));
            }
            _ => match write!(out, "{}", tok.as_str()) {
                Err(e) => cli::fail(e),
                Ok(_) => (),
            }
        }
    }
}
