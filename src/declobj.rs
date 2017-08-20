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

use tok::Lexeme::*;
use tok::Nest;
use tok::RcToken;
use tok::Token;

use self::Expected::*;

#[derive(Clone, Copy, PartialEq)]
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

pub struct DeclarativeObject<I: Iterator<Item=RcToken>> {
    input: I,
    output: Vec<RcToken>,
    broken: bool,
    expect: Expected,
    nest: Nest,
    ident: Option<RcToken>,
    object: Option<RcToken>,
    symbol: Option<RcToken>,
    field: Option<RcToken>,
    method: Option<RcToken>,
    token: Option<RcToken>,
}

impl<I> DeclarativeObject<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> DeclarativeObject<I> {
        DeclarativeObject {
            input: input,
            output: vec!(),
            broken: false,
            expect: Code,
            nest: Nest::new(),
            ident: None,
            object: None,
            symbol: None,
            field: None,
            method: None,
            token: None,
        }
    }

    fn reset(&mut self) {
        assert!(self.nest.groups == 0);
        assert!(self.nest.blocks == 0);
        self.expect = Code;
        self.ident = None;
        self.object = None;
        self.symbol = None;
        self.field = None;
        self.method = None;
        // NB: only reset parsing state
    }

    fn push(&mut self, rctok: RcToken) {
        self.broken |= rctok.borrow().lexeme == Bad;
        self.output.push(rctok);
    }

    fn error(&mut self, rctok: &RcToken) {
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
        self.push(rctok.borrow().turn_bad(msg));
    }

    fn process(&mut self, rctok: RcToken) {
        let lex = rctok.borrow().lexeme;
        match (self.expect, self.nest.blocks, self.nest.groups, lex) {
            (_, _, _, Bad) => {
                self.push(rctok);
                return;
            }

            (Code, 0, _, Name(0)) => (),
            (Code, 0, _, Name(1)) => {
                self.object = Some(RcToken::clone(&rctok));
                self.expect = Ident;
            }
            (Code, 0, _, Name(_)) => {
                self.push(rctok.borrow().turn_bad("invalid identifier"));
                return;
            }
            (Code, _, _, _) => (),

            // NB. Abandon comments inside preprocessed code
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) => return,

            (Ident, _, _, Name(0)) => self.expect = Block,
            (Ident, _, _, Blank) => return,
            (Ident, _, _, _) => return self.error(&rctok),

            (Block, _, _, OpeningBlock) => self.expect = Dot,
            (Block, _, _, Blank) => return,
            (Block, _, _, _) => return self.error(&rctok),

            (Dot, _, _, ClosingBlock) => {
                if self.field.is_none() && self.method.is_none() {
                    self.push(Token::raw(ClosingGroup, ")"));
                    self.push(Token::raw(Delim(';'), ";"));
                    self.push(Token::raw(Blank, "\n"));
                }
                assert!(self.nest.groups == 0);
                assert!(self.nest.blocks == 0);
                self.reset();
            }
            (Dot, _, _, Prop) => self.expect = Member,
            (Dot, _, _, Blank) => return,
            (Dot, _, _, _) => return self.error(&rctok),

            (Member, _, _, Name(0)) => {
                self.symbol = Some(RcToken::clone(&rctok));
                self.expect = FieldOrMethod;
            }
            (Member, _, _, Name(_)) => return self.error(&rctok),
            (Member, _, _, Blank) => return,
            (Member, _, _, _) => return self.error(&rctok),

            (FieldOrMethod, _, _, Delim('=')) => {
                if self.method.is_some() {
                    self.push(rctok.borrow().turn_bad("field after methods"));
                    return;
                }
                if self.field.is_some() {
                    self.push(Token::raw(Delim(','), ","));
                }
                self.push(Token::raw(Blank, "\n"));
                let symbol = self.symbol.take().unwrap();
                self.field = Some(RcToken::clone(&symbol));
                self.symbol = Some(symbol);
                self.expect = Value;
            }
            (FieldOrMethod, _, _, OpeningGroup) => {
                assert!(self.nest.groups == 1);
                if self.method.is_none() {
                    self.push(Token::raw(ClosingGroup, ")"));
                    self.push(Token::raw(Delim(';'), ";"));
                    self.push(Token::raw(Blank, "\n"));
                }
                let symbol = self.symbol.take().unwrap();
                self.method = Some(RcToken::clone(&symbol));
                self.symbol = Some(symbol);
                self.expect = Arguments;
            }
            (FieldOrMethod, _, _, Blank) => return,
            (FieldOrMethod, _, _, _) => return self.error(&rctok),

            (Value, _, 0, Delim(';')) => return self.error(&rctok),
            (Value, _, _, Blank) => return,
            (Value, _, _, _) => self.expect = EndOfField,

            (EndOfField, _, 0, Delim(';')) => self.expect = Dot,
            (EndOfField, _, _, _) => (),

            (Arguments, _, 0, ClosingGroup) => self.expect = EndOfMethod,
            // XXX: insufficient arguments parsing
            (Arguments, _, _, _) => (),

            (SemiColon, _, 0, Delim(';')) => self.expect = Dot,
            (SemiColon, _, _, _) => return self.error(&rctok),

            (_, _, _, _) => unreachable!(),
        }
        match self.expect {
            Code => self.push(rctok),
            Block => {
                assert!(self.object.is_some());
                self.ident = Some(RcToken::clone(&rctok));
                let object = self.object.take().unwrap();
                self.push(Token::raw(Name(0), "sub"));
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Name(0), "vcl_init"));
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(OpeningBlock, "{"));
                self.push(Token::raw(Blank, "\n\t"));
                self.push(Token::raw(Name(0), "new"));
                self.push(Token::raw(Blank, " "));
                self.push(rctok.borrow().to_synth());
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Delim('='), "="));
                self.push(Token::raw(Blank, " "));
                self.push(object.borrow().to_synth());
                self.push(Token::raw(OpeningGroup, "("));
                self.object = Some(object);
            }
            Value => {
                assert!(self.field.is_some());
                assert!(self.symbol.is_some());
                assert_eq!(rctok.borrow().as_str(), "=");
                let field = self.field.take().unwrap();
                self.push(Token::raw(Blank, "\t\t"));
                self.push(field.borrow().to_synth());
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Delim('='), "="));
                self.push(Token::raw(Blank, " "));
                self.field = Some(field);
                self.symbol = None;
            }
            EndOfField => self.push(rctok),
            Arguments => {
                assert!(self.ident.is_some());
                assert!(self.method.is_some());
                match self.symbol {
                    Some(_) => {
                        self.symbol = None;
                        let ident = self.ident.take().unwrap();
                        let method = self.method.take().unwrap();
                        let mut sym = String::new();
                        sym += ident.borrow().as_str();
                        sym.push('.');
                        sym += method.borrow().as_str();
                        self.push(Token::raw(Blank, "\t"));
                        self.push(Token::dyn(Name(1), sym));
                        self.push(Token::raw(OpeningGroup, "("));
                        self.ident = Some(ident);
                        self.method = Some(method);
                    }
                    None => self.push(rctok),
                }
            }
            EndOfMethod => {
                self.expect = SemiColon;
                self.push(Token::raw(ClosingGroup, ")"));
                self.push(Token::raw(Delim(';'), ";"));
                self.push(Token::raw(Blank, "\n"));
            }
            _ => (),
        };
    }
}

impl<I> Iterator for DeclarativeObject<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.broken {
            return None;
        }
        if self.output.len() > 0 {
            return Some(self.output.remove(0));
        }
        loop {
            match self.input.next() {
                Some(rctok) => {
                    self.nest.update(&rctok);
                    self.token = Some(RcToken::clone(&rctok));
                    if !self.broken {
                        self.process(rctok);
                    }
                }
                None => {
                    #[cfg(kcov)]
                    assert!(self.input.next().is_none()); // good behavior?

                    if self.expect != Code {
                        self.broken = true;
                        match self.token {
                            Some(ref rctok) => return Some(rctok.borrow()
                                .turn_bad("incomplete VCL")),
                            None => unreachable!(),
                        }
                    }
                    return None;
                }
            }
            if self.expect == Code || self.output.len() != 0 {
                break;
            }
        }
        assert!(self.output.len() > 0);
        Some(self.output.remove(0))
    }
}
