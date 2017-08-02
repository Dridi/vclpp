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

use std::io::BufWriter;
use std::io::Write;
use std::io::stderr;
use std::io::stdin;
use std::io::stdout;
use std::process::exit;
use std::string::String;
use std::vec::Vec;

mod tok;

use tok::Lexeme::*;
use tok::Tokenizer;

fn write_escaped<W: Write>(out: &mut BufWriter<W>, s: &str) {
    s.bytes().map(|b| -> Vec<u8> {
        match b as char {
            '\\' => vec!('\\' as u8, '\\' as u8),
            '\n' => vec!('\\' as u8, 'n' as u8),
            '\r' => vec!('\\' as u8, 'r' as u8),
            '\t' => vec!('\\' as u8, 't' as u8),
            _ => vec!(b)
        }
    }).map(|v| out.write(v.as_ref()))
        .inspect(|r| if r.is_err() { exit(1); })
        .count();
}

fn main() {
    let mut buf = String::new();

    loop {
        match stdin().read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) => panic!("error: {}", e)
        }
    }

    let mut out = BufWriter::new(stdout());

    for tok in Tokenizer::new(buf.chars()) {
        write!(out, "[{}...{}] ", tok.start, tok.end);
        match tok.lexeme {
            Bad(s) => {
                write!(out, "bad token: {}\n", s);
            }
            _ => {
                write!(out, "token: {:?} '", tok.lexeme);
                write_escaped(&mut out, &buf[&tok]);
                write!(out, "'\n");
            }
        }
    }
}
