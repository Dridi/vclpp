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

use std::io::Write;

use tok::Lexeme::*;
use tok::Token;
use tok::Tokenizer;

use Expected::*;

/* ------------------------------------------------------------------- */

#[derive(Clone, Copy, Debug)]
enum Expected {
    Code,
    Ident,
    Block,
    Dot,
    Member,
    FieldOrMethod,
    Value,
    EndOfField,
    Arguments,
    EndOfMethod,
    SemiColon,
}

struct Preprocessor {
    expect: Expected,
    groups: isize,
    blocks: isize,
    ident: Option<Token>,
    object: Option<Token>,
    symbol: Option<Token>,
    field: Option<Token>,
    method: Option<Token>,
}

impl Preprocessor {
    fn new() -> Preprocessor {
        Preprocessor {
            expect: Code,
            groups: 0,
            blocks: 0,
            ident: None,
            object: None,
            symbol: None,
            field: None,
            method: None,
        }
    }

    fn balance(&mut self, tok: Token) -> Result<(), Token> {
        match (tok.lexeme, self.groups) {
            (OpeningBlock, 0) => Ok(()),
            (OpeningBlock, _) => Err(tok),
            (_, _) => Ok(()),
        }?;
        match tok.lexeme {
            OpeningGroup => self.groups += 1,
            ClosingGroup => self.groups -= 1,
            OpeningBlock => self.blocks += 1,
            ClosingBlock => self.blocks -= 1,
            _ => (),
        }
        assert!(self.groups >= -1);
        assert!(self.blocks >= -1);
        match (self.groups, self.blocks) {
            (-1, _) |
            (_, -1) => Err(tok),
            (0, 0) => Ok(()),
            (_, 0) => Err(tok),
            (_, _) => Ok(()),
        }
    }

    fn exec<'a, W: Write>(src: &'a String, mut out: W)
    -> Result<(), Token> {
        let mut pp = Preprocessor::new();
        for tok in Tokenizer::new(src.chars()) {
            pp.balance(tok)?;
            match (pp.expect, pp.blocks, pp.groups, tok.lexeme) {
                (Code, 0, _, Name(0)) => (),
                (Code, 0, _, Name(1)) => {
                    pp.object = Some(tok);
                    pp.expect = Ident;
                }
                (Code, 0, _, Name(_)) => unimplemented!(),
                (Code, _, _, _) => (),

                // NB. Abandon comments inside preprocessed code
                (_, _, _, Comment) |
                (_, _, _, CComment) |
                (_, _, _, CxxComment) => continue,

                (Ident, _, _, Name(0)) => pp.expect = Block,
                (Ident, _, _, Blank) => continue,
                (Ident, _, _, _) => unimplemented!(),

                (Block, _, _, OpeningBlock) => pp.expect = Dot,
                (Block, _, _, Blank) => continue,
                (Block, _, _, _) => unimplemented!(),

                (Dot, _, _, ClosingBlock) => {
                    if pp.field.is_none() && pp.method.is_none() {
                        write!(out, ");\n");
                    }
                    assert!(pp.groups == 0);
                    assert!(pp.blocks == 0);
                    pp = Preprocessor::new();
                }
                (Dot, _, _, Prop) => pp.expect = Member,
                (Dot, _, _, Blank) => continue,
                (Dot, _, _, _) => unimplemented!(),

                (Member, _, _, Name(0)) => {
                    pp.symbol = Some(tok);
                    pp.expect = FieldOrMethod;
                }
                (Member, _, _, Name(_)) => unimplemented!(),
                (Member, _, _, Blank) => continue,
                (Member, _, _, _) => unimplemented!(),

                (FieldOrMethod, _, _, Delim('=')) => {
                    if pp.method.is_some() {
                        return Err(tok);
                    }
                    if pp.field.is_some() {
                        write!(out, ",");
                    }
                    write!(out, "\n");
                    pp.field = pp.symbol;
                    pp.expect = Value;
                }
                (FieldOrMethod, _, _, OpeningGroup) => {
                    assert!(pp.groups == 1);
                    if pp.method.is_none() {
                        write!(out, ");\n");
                    }
                    pp.method = pp.symbol;
                    pp.expect = Arguments;
                }
                (FieldOrMethod, _, _, Blank) => continue,
                (FieldOrMethod, _, _, _) => unimplemented!(),

                (Value, _, 0, Delim(';')) => pp.expect = EndOfField,
                (Value, _, _, _) => (),

                (Arguments, _, 0, ClosingGroup) => pp.expect = EndOfMethod,
                (Arguments, _, _, _) => (),

                (SemiColon, _, 0, Delim(';')) => pp.expect = Dot,
                (SemiColon, _, _, _) => (),

                (_, _, _, _) => unreachable!(),
            }
            match pp.expect {
                Code => write!(out, "{}", &src[&tok]),
                Block => {
                    assert!(pp.object.is_some());
                    pp.ident = Some(tok);
                    write!(out, "sub vcl_init {{\n\tnew {} = {}(",
                        &src[&tok], &src[&pp.object.unwrap()])
                }
                Value => {
                    assert!(pp.field.is_some());
                    match pp.symbol {
                        Some(_) => {
                            assert_eq!(&src[&tok], "=");
                            pp.symbol = None;
                            write!(out, "\t\t{} =", &src[&pp.field.unwrap()])
                        }
                        None => write!(out, "{}", &src[&tok])
                    }
                }
                EndOfField => {
                    pp.expect = Dot;
                    Ok(())
                }
                Arguments => {
                    assert!(pp.ident.is_some());
                    assert!(pp.method.is_some());
                    match pp.symbol {
                        Some(_) => {
                            pp.symbol = None;
                            write!(out, "\t{}.{}(", &src[&pp.ident.unwrap()],
                                &src[&pp.method.unwrap()])
                        }
                        None => write!(out, "{}", &src[&tok])
                    }
                }
                EndOfMethod => {
                    pp.expect = SemiColon;
                    write!(out, ");\n")
                }
                _ => Ok(()),
            };
        }
        if pp.groups != 0 || pp.blocks != 0 {
            unimplemented!();
        }
        Ok(())
    }
}

/* ------------------------------------------------------------------- */

fn main() {
    let (src, out) = match cli::parse_args() {
        Ok((s, o)) => (s, o),
        Err(e) => cli::fail(e),
    };

    match Preprocessor::exec(&src, out) {
        Err(tok) => panic!("{:?}", tok.lexeme),
        _ => ()
    }
}
