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

use std::io::Error;
use std::io::Write;

use tok::Lexeme::*;
use tok::Token;
use tok::Tokenizer;

use Expected::*;
use PvclError::*;

/* ------------------------------------------------------------------- */

type PvclResult = Result<(), PvclError>;

enum PvclError {
    SyntaxError(Token),
    IoError(Error),
}

impl From<Error> for PvclError {
    fn from(e: Error) -> PvclError { IoError(e) }
}

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

impl Expected {
    fn pvcl(&self) -> bool {
        match self {
            &Code => false,
            _ => true,
        }
    }
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

    fn balance(&mut self, tok: Token) -> Option<&'static str> {
        assert!(self.groups >= 0);
        assert!(self.blocks >= 0);
        if tok.lexeme == OpeningBlock && self.groups > 0 {
            return Some("opening a block inside an expression");
        }

        match tok.lexeme {
            OpeningGroup => self.groups += 1,
            ClosingGroup => self.groups -= 1,
            OpeningBlock => self.blocks += 1,
            ClosingBlock => self.blocks -= 1,
            _ => (),
        }

        if self.groups < 0 || self.blocks < 0 {
            Some("unbalanced brackets")
        }
        else {
            None
        }
    }

    fn error(&self, tok: Token) -> PvclResult {
        let msg = match self.expect {
            Code |
            Arguments |
            EndOfField |
            EndOfMethod => unreachable!(),
            Ident => "expected identifier",
            Block => "expected '{'",
            Dot => "expected '.' or '}'",
            Member => "expected field or method",
            FieldOrMethod => "expected '=' or '('",
            Value => "expected value",
            SemiColon => "expected ';'",
        };
        Err(SyntaxError(tok.turn_bad(msg)))
    }

    fn exec<'a, W: Write>(src: &'a String, mut out: W) -> PvclResult {
        let mut pp = Preprocessor::new();
        let mut last_tok: Option<Token> = None;
        for t in Tokenizer::new(src.chars()) {
            let tok = match pp.balance(t) {
                Some(msg) => t.turn_bad(msg),
                None => t,
            };
            last_tok = Some(tok);
            match (pp.expect, pp.blocks, pp.groups, tok.lexeme) {
                (_, _, _, Bad(s)) => Err(SyntaxError(tok.turn_bad(s)))?,

                (Code, 0, _, Name(0)) => (),
                (Code, 0, _, Name(1)) => {
                    pp.object = Some(tok);
                    pp.expect = Ident;
                }
                (Code, 0, _, Name(_)) => {
                    Err(SyntaxError(tok.turn_bad("invalid identifier")))?
                }
                (Code, _, _, _) => (),

                // NB. Abandon comments inside preprocessed code
                (_, _, _, Comment) |
                (_, _, _, CComment) |
                (_, _, _, CxxComment) => continue,

                (Ident, _, _, Name(0)) => pp.expect = Block,
                (Ident, _, _, Blank) => continue,
                (Ident, _, _, _) => pp.error(tok)?,

                (Block, _, _, OpeningBlock) => pp.expect = Dot,
                (Block, _, _, Blank) => continue,
                (Block, _, _, _) => pp.error(tok)?,

                (Dot, _, _, ClosingBlock) => {
                    if pp.field.is_none() && pp.method.is_none() {
                        write!(out, ");\n")?;
                    }
                    assert!(pp.groups == 0);
                    assert!(pp.blocks == 0);
                    pp = Preprocessor::new();
                }
                (Dot, _, _, Prop) => pp.expect = Member,
                (Dot, _, _, Blank) => continue,
                (Dot, _, _, _) => pp.error(tok)?,

                (Member, _, _, Name(0)) => {
                    pp.symbol = Some(tok);
                    pp.expect = FieldOrMethod;
                }
                (Member, _, _, Name(_)) => pp.error(tok)?,
                (Member, _, _, Blank) => continue,
                (Member, _, _, _) => pp.error(tok)?,

                (FieldOrMethod, _, _, Delim('=')) => {
                    if pp.method.is_some() {
                        return Err(SyntaxError(
                            tok.turn_bad("field after methods")));
                    }
                    if pp.field.is_some() {
                        write!(out, ",")?;
                    }
                    write!(out, "\n")?;
                    pp.field = pp.symbol;
                    pp.expect = Value;
                }
                (FieldOrMethod, _, _, OpeningGroup) => {
                    assert!(pp.groups == 1);
                    if pp.method.is_none() {
                        write!(out, ");\n")?;
                    }
                    pp.method = pp.symbol;
                    pp.expect = Arguments;
                }
                (FieldOrMethod, _, _, Blank) => continue,
                (FieldOrMethod, _, _, _) => pp.error(tok)?,

                (Value, _, 0, Delim(';')) => pp.error(tok)?,
                (Value, _, _, Blank) => continue,
                (Value, _, _, _) => pp.expect = EndOfField,

                (EndOfField, _, 0, Delim(';')) => pp.expect = Dot,
                (EndOfField, _, _, _) => (),

                (Arguments, _, 0, ClosingGroup) => pp.expect = EndOfMethod,
                // XXX: insufficient arguments parsing
                (Arguments, _, _, _) => (),

                (SemiColon, _, 0, Delim(';')) => pp.expect = Dot,
                (SemiColon, _, _, _) => pp.error(tok)?,

                (_, _, _, _) => unreachable!(),
            }
            match pp.expect {
                Code => write!(out, "{}", &src[&tok])?,
                Block => {
                    assert!(pp.object.is_some());
                    pp.ident = Some(tok);
                    write!(out, "sub vcl_init {{\n\tnew {} = {}(",
                        &src[&tok], &src[&pp.object.unwrap()])?;
                }
                Value => {
                    assert!(pp.field.is_some());
                    assert!(pp.symbol.is_some());
                    assert_eq!(&src[&tok], "=");
                    write!(out, "\t\t{} = ", &src[&pp.field.unwrap()])?;
                    pp.symbol = None;
                }
                EndOfField => write!(out, "{}", &src[&tok])?,
                Arguments => {
                    assert!(pp.ident.is_some());
                    assert!(pp.method.is_some());
                    match pp.symbol {
                        Some(_) => {
                            pp.symbol = None;
                            write!(out, "\t{}.{}(", &src[&pp.ident.unwrap()],
                                &src[&pp.method.unwrap()])?;
                        }
                        None => write!(out, "{}", &src[&tok])?,
                    }
                }
                EndOfMethod => {
                    pp.expect = SemiColon;
                    write!(out, ");\n")?;
                }
                _ => (),
            };
        }
        if pp.expect.pvcl() || pp.groups != 0 || pp.blocks != 0 {
            assert!(last_tok.is_some());
            Err(SyntaxError(last_tok.unwrap().turn_bad("incomplete VCL")))?;
        }
        out.flush()?;
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
        Err(SyntaxError(tok)) => {
            match tok.lexeme {
                Bad(msg) => {
                    cli::fail(format!("{}, Line {}, Pos {}",
                        msg, tok.start.line, tok.start.column));
                }
                _ => unreachable!(),
            }
        }
        Err(IoError(e)) => cli::fail(e),
        Ok(_) => ()
    }
}
