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

struct Preprocessor<'pp> {
    source: &'pp String,
    expect: Expected,
    groups: isize,
    blocks: isize,
    ident: Option<Token>,
    object: Option<Token>,
    symbol: Option<Token>,
    field: Option<Token>,
    method: Option<Token>,
    token: Option<Token>,
}

impl<'pp> Preprocessor<'pp> {
    fn new(source: &'pp String) -> Preprocessor<'pp> {
        Preprocessor {
            source: source,
            expect: Code,
            groups: 0,
            blocks: 0,
            ident: None,
            object: None,
            symbol: None,
            field: None,
            method: None,
            token: None,
        }
    }

    fn reset(&mut self) {
        self.expect = Code;
        self.groups = 0;
        self.blocks = 0;
        self.ident = None;
        self.object = None;
        self.symbol = None;
        self.field = None;
        self.method = None;
        // NB: don't reset self.token
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

    fn exec<'a, W: Write>(&mut self, mut out: W)
    -> PvclResult {
        for t in Tokenizer::new(self.source.chars()) {
            let tok = match self.balance(t) {
                Some(msg) => t.turn_bad(msg),
                None => t,
            };
            self.token = Some(tok);
            match (self.expect, self.blocks, self.groups, tok.lexeme) {
                (_, _, _, Bad(s)) => Err(SyntaxError(tok.turn_bad(s)))?,

                (Code, 0, _, Name(0)) => (),
                (Code, 0, _, Name(1)) => {
                    self.object = Some(tok);
                    self.expect = Ident;
                }
                (Code, 0, _, Name(_)) => {
                    Err(SyntaxError(tok.turn_bad("invalid identifier")))?
                }
                (Code, _, _, _) => (),

                // NB. Abandon comments inside preprocessed code
                (_, _, _, Comment) |
                (_, _, _, CComment) |
                (_, _, _, CxxComment) => continue,

                (Ident, _, _, Name(0)) => self.expect = Block,
                (Ident, _, _, Blank) => continue,
                (Ident, _, _, _) => self.error(tok)?,

                (Block, _, _, OpeningBlock) => self.expect = Dot,
                (Block, _, _, Blank) => continue,
                (Block, _, _, _) => self.error(tok)?,

                (Dot, _, _, ClosingBlock) => {
                    if self.field.is_none() && self.method.is_none() {
                        write!(out, ");\n")?;
                    }
                    assert!(self.groups == 0);
                    assert!(self.blocks == 0);
                    self.reset();
                }
                (Dot, _, _, Prop) => self.expect = Member,
                (Dot, _, _, Blank) => continue,
                (Dot, _, _, _) => self.error(tok)?,

                (Member, _, _, Name(0)) => {
                    self.symbol = Some(tok);
                    self.expect = FieldOrMethod;
                }
                (Member, _, _, Name(_)) => self.error(tok)?,
                (Member, _, _, Blank) => continue,
                (Member, _, _, _) => self.error(tok)?,

                (FieldOrMethod, _, _, Delim('=')) => {
                    if self.method.is_some() {
                        return Err(SyntaxError(
                            tok.turn_bad("field after methods")));
                    }
                    if self.field.is_some() {
                        write!(out, ",")?;
                    }
                    write!(out, "\n")?;
                    self.field = self.symbol;
                    self.expect = Value;
                }
                (FieldOrMethod, _, _, OpeningGroup) => {
                    assert!(self.groups == 1);
                    if self.method.is_none() {
                        write!(out, ");\n")?;
                    }
                    self.method = self.symbol;
                    self.expect = Arguments;
                }
                (FieldOrMethod, _, _, Blank) => continue,
                (FieldOrMethod, _, _, _) => self.error(tok)?,

                (Value, _, 0, Delim(';')) => self.error(tok)?,
                (Value, _, _, Blank) => continue,
                (Value, _, _, _) => self.expect = EndOfField,

                (EndOfField, _, 0, Delim(';')) => self.expect = Dot,
                (EndOfField, _, _, _) => (),

                (Arguments, _, 0, ClosingGroup) => self.expect = EndOfMethod,
                // XXX: insufficient arguments parsing
                (Arguments, _, _, _) => (),

                (SemiColon, _, 0, Delim(';')) => self.expect = Dot,
                (SemiColon, _, _, _) => self.error(tok)?,

                (_, _, _, _) => unreachable!(),
            }
            match self.expect {
                Code => write!(out, "{}", &self.source[&tok])?,
                Block => {
                    assert!(self.object.is_some());
                    self.ident = Some(tok);
                    write!(out, "sub vcl_init {{\n\tnew {} = {}(",
                        &self.source[&tok],
                        &self.source[&self.object.unwrap()])?;
                }
                Value => {
                    assert!(self.field.is_some());
                    assert!(self.symbol.is_some());
                    assert_eq!(&self.source[&tok], "=");
                    write!(out, "\t\t{} = ",
                        &self.source[&self.field.unwrap()])?;
                    self.symbol = None;
                }
                EndOfField => write!(out, "{}", &self.source[&tok])?,
                Arguments => {
                    assert!(self.ident.is_some());
                    assert!(self.method.is_some());
                    match self.symbol {
                        Some(_) => {
                            self.symbol = None;
                            write!(out, "\t{}.{}(",
                                &self.source[&self.ident.unwrap()],
                                &self.source[&self.method.unwrap()])?;
                        }
                        None => write!(out, "{}", &self.source[&tok])?,
                    }
                }
                EndOfMethod => {
                    self.expect = SemiColon;
                    write!(out, ");\n")?;
                }
                _ => (),
            };
        }
        if self.expect.pvcl() || self.groups != 0 || self.blocks != 0 {
            assert!(self.token.is_some());
            Err(SyntaxError(self.token.unwrap().turn_bad("incomplete VCL")))?;
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

    match Preprocessor::new(&src).exec(out) {
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
