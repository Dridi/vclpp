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

use tok::Flow;
use tok::Lexeme::*;
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

#[must_use = "preprocessors are lazy and do nothing unless consumed"]
pub struct DeclarativeObject<I: Iterator<Item=RcToken>> {
    flow: Flow<I>,
    output: Vec<RcToken>,
    expect: Expected,
    broken: bool,
    ident: Option<RcToken>,
    object: Option<RcToken>,
    symbol: Option<RcToken>,
    field: Option<RcToken>,
    method: Option<RcToken>,
}

impl<I> DeclarativeObject<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> DeclarativeObject<I> {
        DeclarativeObject {
            flow: Flow::new(input),
            output: vec!(),
            expect: Code,
            broken: false,
            ident: None,
            object: None,
            symbol: None,
            field: None,
            method: None,
        }
    }

    fn reset(&mut self) {
        assert!(self.flow.groups == 0);
        assert!(self.flow.blocks == 0);
        self.expect = Code;
        self.ident = None;
        self.object = None;
        self.symbol = None;
        self.field = None;
        self.method = None;
        // NB: only reset parsing state
    }

    fn push(&mut self, tok: RcToken) {
        let lex = tok.lexeme;
        self.broken |= lex == Bad;
        match lex {
            Bad => self.output = vec!(tok),
            _ => self.output.push(tok),
        }
    }

    fn error(&mut self) {
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
        let bust = self.flow.bust(msg);
        self.push(bust);
    }

    fn process(&mut self, tok: RcToken) {
        let lex = tok.lexeme;
        match (self.expect, self.flow.blocks, self.flow.groups, lex) {
            (_, _, _, Bad) => return self.push(tok),

            (Code, 0, _, Name(0)) => (),
            (Code, 0, _, Name(1)) => {
                self.object = Some(RcToken::clone(&tok));
                self.expect = Ident;
            }
            (Code, 0, _, Name(_)) => {
                let bust = self.flow.bust("invalid identifier");
                return self.push(bust);
            }
            (Code, _, _, _) => (),

            // NB. Abandon comments inside preprocessed code
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) => return,

            (Ident, _, _, Name(0)) => self.expect = Block,
            (Ident, _, _, Blank) => return,
            (Ident, _, _, _) => return self.error(),

            (Block, _, _, OpeningBlock) => self.expect = Dot,
            (Block, _, _, Blank) => return,
            (Block, _, _, _) => return self.error(),

            (Dot, _, _, ClosingBlock) => {
                if self.field.is_none() && self.method.is_none() {
                    self.push(Token::raw(ClosingGroup, ")"));
                    self.push(Token::raw(Delim(';'), ";"));
                    self.push(Token::raw(Blank, "\n"));
                }
                assert!(self.flow.groups == 0);
                assert!(self.flow.blocks == 0);
                self.reset();
            }
            (Dot, _, _, Prop) => self.expect = Member,
            (Dot, _, _, Blank) => return,
            (Dot, _, _, _) => return self.error(),

            (Member, _, _, Name(0)) => {
                self.symbol = Some(RcToken::clone(&tok));
                self.expect = FieldOrMethod;
            }
            (Member, _, _, Name(_)) => return self.error(),
            (Member, _, _, Blank) => return,
            (Member, _, _, _) => return self.error(),

            (FieldOrMethod, _, _, Delim('=')) => {
                if self.method.is_some() {
                    let bust = self.flow.bust("field after methods");
                    return self.push(bust);
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
                assert!(self.flow.groups == 1);
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
            (FieldOrMethod, _, _, _) => return self.error(),

            (Value, _, 0, Delim(';')) => return self.error(),
            (Value, _, _, Blank) => return,
            (Value, _, _, _) => self.expect = EndOfField,

            (EndOfField, _, 0, Delim(';')) => self.expect = Dot,
            (EndOfField, _, _, _) => (),

            (Arguments, _, 0, ClosingGroup) => self.expect = EndOfMethod,
            // XXX: insufficient arguments parsing
            (Arguments, _, _, _) => (),

            (SemiColon, _, 0, Delim(';')) => self.expect = Dot,
            (SemiColon, _, _, _) => return self.error(),

            (_, _, _, _) => unreachable!(),
        }
        match self.expect {
            Code => self.push(tok),
            Block => {
                assert!(self.object.is_some());
                self.ident = Some(RcToken::clone(&tok));
                let object = self.object.take().unwrap();
                self.push(Token::raw(Name(0), "sub"));
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Name(0), "vcl_init"));
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(OpeningBlock, "{"));
                self.push(Token::raw(Blank, "\n\t"));
                self.push(Token::raw(Name(0), "new"));
                self.push(Token::raw(Blank, " "));
                self.push(tok.to_synth());
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Delim('='), "="));
                self.push(Token::raw(Blank, " "));
                self.push(object.to_synth());
                self.push(Token::raw(OpeningGroup, "("));
                self.object = Some(object);
            }
            Value => {
                assert!(self.field.is_some());
                assert!(self.symbol.is_some());
                assert_eq!(tok.as_str(), "=");
                let field = self.field.take().unwrap();
                self.push(Token::raw(Blank, "\t\t"));
                self.push(field.to_synth());
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Delim('='), "="));
                self.push(Token::raw(Blank, " "));
                self.field = Some(field);
                self.symbol = None;
            }
            EndOfField => self.push(tok),
            Arguments => {
                assert!(self.ident.is_some());
                assert!(self.method.is_some());
                match self.symbol {
                    Some(_) => {
                        self.symbol = None;
                        let ident = self.ident.take().unwrap();
                        let method = self.method.take().unwrap();
                        let mut sym = String::new();
                        sym += ident.as_str();
                        sym.push('.');
                        sym += method.as_str();
                        self.push(Token::raw(Blank, "\t"));
                        self.push(Token::dyn(Name(1), sym));
                        self.push(Token::raw(OpeningGroup, "("));
                        self.ident = Some(ident);
                        self.method = Some(method);
                    }
                    None => self.push(tok),
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
        if self.output.len() > 0 {
            return Some(self.output.remove(0));
        }
        while self.output.len() == 0 {
            match self.flow.next() {
                Some(tok) => self.process(tok),
                None => {
                    if !self.broken && self.expect != Code {
                        self.broken = true;
                        return self.flow.incomplete();
                    }
                    return None;
                }
            }
        }
        Some(self.output.remove(0))
    }
}
