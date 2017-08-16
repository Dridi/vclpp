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

    fn balance(&mut self, tok: &Token) -> Option<&'static str> {
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

    fn error(&self, tok: Token) -> Vec<Token> {
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
        vec!(tok.turn_bad(msg))
    }

    fn process(&mut self, tok: Token) -> Vec<Token> {
        let mut synth = vec!();
        match (self.expect, self.blocks, self.groups, tok.lexeme) {
            (_, _, _, Bad(_)) => return vec!(tok),

            (Code, 0, _, Name(0)) => (),
            (Code, 0, _, Name(1)) => {
                self.object = Some(tok.clone());
                self.expect = Ident;
            }
            (Code, 0, _, Name(_)) => {
                return vec!(tok.turn_bad("invalid identifier"));
            }
            (Code, _, _, _) => (),

            // NB. Abandon comments inside preprocessed code
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) => return synth,

            (Ident, _, _, Name(0)) => self.expect = Block,
            (Ident, _, _, Blank) => return synth,
            (Ident, _, _, _) => return self.error(tok),

            (Block, _, _, OpeningBlock) => self.expect = Dot,
            (Block, _, _, Blank) => return synth,
            (Block, _, _, _) => return self.error(tok),

            (Dot, _, _, ClosingBlock) => {
                if self.field.is_none() && self.method.is_none() {
                    synth.push(Token::raw(ClosingGroup, ")"));
                    synth.push(Token::raw(Delim(';'), ";"));
                    synth.push(Token::raw(Blank, "\n"));
                }
                assert!(self.groups == 0);
                assert!(self.blocks == 0);
                self.reset();
            }
            (Dot, _, _, Prop) => self.expect = Member,
            (Dot, _, _, Blank) => return synth,
            (Dot, _, _, _) => return self.error(tok),

            (Member, _, _, Name(0)) => {
                self.symbol = Some(tok.clone());
                self.expect = FieldOrMethod;
            }
            (Member, _, _, Name(_)) => return self.error(tok),
            (Member, _, _, Blank) => return synth,
            (Member, _, _, _) => return self.error(tok),

            (FieldOrMethod, _, _, Delim('=')) => {
                if self.method.is_some() {
                    return vec!(tok.turn_bad("field after methods"));
                }
                if self.field.is_some() {
                    synth.push(Token::raw(Delim(','), ","));
                }
                synth.push(Token::raw(Blank, "\n"));
                self.field = self.symbol.clone();
                self.expect = Value;
            }
            (FieldOrMethod, _, _, OpeningGroup) => {
                assert!(self.groups == 1);
                if self.method.is_none() {
                    synth.push(Token::raw(ClosingGroup, ")"));
                    synth.push(Token::raw(Delim(';'), ";"));
                    synth.push(Token::raw(Blank, "\n"));
                }
                self.method = self.symbol.clone();
                self.expect = Arguments;
            }
            (FieldOrMethod, _, _, Blank) => return synth,
            (FieldOrMethod, _, _, _) => return self.error(tok),

            (Value, _, 0, Delim(';')) => return self.error(tok),
            (Value, _, _, Blank) => return synth,
            (Value, _, _, _) => self.expect = EndOfField,

            (EndOfField, _, 0, Delim(';')) => self.expect = Dot,
            (EndOfField, _, _, _) => (),

            (Arguments, _, 0, ClosingGroup) => self.expect = EndOfMethod,
            // XXX: insufficient arguments parsing
            (Arguments, _, _, _) => (),

            (SemiColon, _, 0, Delim(';')) => self.expect = Dot,
            (SemiColon, _, _, _) => return self.error(tok),

            (_, _, _, _) => unreachable!(),
        }
        match self.expect {
            Code => synth.push(tok),
            Block => {
                assert!(self.object.is_some());
                self.ident = Some(tok.clone());
                let object = self.object.clone().unwrap();
                synth.push(Token::raw(Name(0), "sub"));
                synth.push(Token::raw(Blank, " "));
                synth.push(Token::raw(Name(0), "vcl_init"));
                synth.push(Token::raw(Blank, " "));
                synth.push(Token::raw(OpeningBlock, "{"));
                synth.push(Token::raw(Blank, "\n\t"));
                synth.push(Token::raw(Name(0), "new"));
                synth.push(Token::raw(Blank, " "));
                synth.push(tok.to_synth(self.source));
                synth.push(Token::raw(Blank, " "));
                synth.push(Token::raw(Delim('='), "="));
                synth.push(Token::raw(Blank, " "));
                synth.push(object.to_synth(self.source));
                synth.push(Token::raw(OpeningGroup, "("));
            }
            Value => {
                assert!(self.field.is_some());
                assert!(self.symbol.is_some());
                assert_eq!(&self.source[&tok], "=");
                let field = self.field.clone().unwrap();
                synth.push(Token::raw(Blank, "\t\t"));
                synth.push(field.to_synth(self.source));
                synth.push(Token::raw(Blank, " "));
                synth.push(Token::raw(Delim('='), "="));
                synth.push(Token::raw(Blank, " "));
                self.symbol = None;
            }
            EndOfField => synth.push(tok),
            Arguments => {
                assert!(self.ident.is_some());
                assert!(self.method.is_some());
                match self.symbol {
                    Some(_) => {
                        self.symbol = None;
                        let ident = self.ident.clone().unwrap();
                        let method = self.method.clone().unwrap();
                        let mut sym = String::new();
                        sym += &self.source[&ident];
                        sym.push('.');
                        sym += &self.source[&method];
                        synth.push(Token::raw(Blank, "\t"));
                        synth.push(Token::dyn(Name(1), sym));
                        synth.push(Token::raw(OpeningGroup, "("));
                    }
                    None => synth.push(tok),
                }
            }
            EndOfMethod => {
                self.expect = SemiColon;
                synth.push(Token::raw(ClosingGroup, ")"));
                synth.push(Token::raw(Delim(';'), ";"));
                synth.push(Token::raw(Blank, "\n"));
            }
            _ => (),
        };
        synth
    }

    fn exec<'a, W: Write>(&mut self, mut out: W)
    -> PvclResult {
        for t in Tokenizer::new(self.source.chars()) {
            let tok = match self.balance(&t) {
                Some(msg) => t.turn_bad(msg),
                None => t,
            };
            self.token = Some(tok.clone());
            for t2 in self.process(tok) {
                match t2.lexeme {
                    Bad(_) => Err(SyntaxError(t2))?,
                    _ => write!(out, "{}", &self.source[&t2])?,
                }
            }
        }
        if self.expect.pvcl() || self.groups != 0 || self.blocks != 0 {
            assert!(self.token.is_some());
            let token = self.token.clone().unwrap();
            Err(SyntaxError(token.turn_bad("incomplete VCL")))?;
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
