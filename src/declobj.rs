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
use tok::Token;

use self::Expected::*;

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

pub struct DeclarativeObject<I: Iterator<Item=Token>> {
    input: I,
    output: Vec<Token>,
    broken: bool,
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

impl<I> DeclarativeObject<I>
where I: Iterator<Item=Token> {
    pub fn new(input: I) -> DeclarativeObject<I> {
        DeclarativeObject {
            input: input,
            output: vec!(),
            broken: false,
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
        // NB: only reset parsing state
    }

    fn push(&mut self, tok: Token) {
        if !tok.lexeme.is_valid() {
            self.broken = true;
        }
        self.output.push(tok);
    }

    fn balance(&mut self, tok: &Token) {
        assert!(self.groups >= 0);
        assert!(self.blocks >= 0);
        if tok.lexeme == OpeningBlock && self.groups > 0 {
            self.push(tok.turn_bad("opening a block inside an expression"));
            return;
        }

        match tok.lexeme {
            OpeningGroup => self.groups += 1,
            ClosingGroup => self.groups -= 1,
            OpeningBlock => self.blocks += 1,
            ClosingBlock => self.blocks -= 1,
            _ => (),
        }

        if self.groups < 0 || self.blocks < 0 {
            self.push(tok.turn_bad("unbalanced brackets"));
        }
    }

    fn error(&mut self, tok: Token) {
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
        self.push(tok.turn_bad(msg));
    }

    fn process(&mut self, tok: Token) {
        match (self.expect, self.blocks, self.groups, tok.lexeme) {
            (_, _, _, Bad(_)) => {
                self.push(tok);
                return;
            }

            (Code, 0, _, Name(0)) => (),
            (Code, 0, _, Name(1)) => {
                self.object = Some(tok.clone());
                self.expect = Ident;
            }
            (Code, 0, _, Name(_)) => {
                self.push(tok.turn_bad("invalid identifier"));
                return;
            }
            (Code, _, _, _) => (),

            // NB. Abandon comments inside preprocessed code
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) => return,

            (Ident, _, _, Name(0)) => self.expect = Block,
            (Ident, _, _, Blank) => return,
            (Ident, _, _, _) => return self.error(tok),

            (Block, _, _, OpeningBlock) => self.expect = Dot,
            (Block, _, _, Blank) => return,
            (Block, _, _, _) => return self.error(tok),

            (Dot, _, _, ClosingBlock) => {
                if self.field.is_none() && self.method.is_none() {
                    self.push(Token::raw(ClosingGroup, ")"));
                    self.push(Token::raw(Delim(';'), ";"));
                    self.push(Token::raw(Blank, "\n"));
                }
                assert!(self.groups == 0);
                assert!(self.blocks == 0);
                self.reset();
            }
            (Dot, _, _, Prop) => self.expect = Member,
            (Dot, _, _, Blank) => return,
            (Dot, _, _, _) => return self.error(tok),

            (Member, _, _, Name(0)) => {
                self.symbol = Some(tok.clone());
                self.expect = FieldOrMethod;
            }
            (Member, _, _, Name(_)) => return self.error(tok),
            (Member, _, _, Blank) => return,
            (Member, _, _, _) => return self.error(tok),

            (FieldOrMethod, _, _, Delim('=')) => {
                if self.method.is_some() {
                    self.push(tok.turn_bad("field after methods"));
                    return;
                }
                if self.field.is_some() {
                    self.push(Token::raw(Delim(','), ","));
                }
                self.push(Token::raw(Blank, "\n"));
                self.field = self.symbol.clone();
                self.expect = Value;
            }
            (FieldOrMethod, _, _, OpeningGroup) => {
                assert!(self.groups == 1);
                if self.method.is_none() {
                    self.push(Token::raw(ClosingGroup, ")"));
                    self.push(Token::raw(Delim(';'), ";"));
                    self.push(Token::raw(Blank, "\n"));
                }
                self.method = self.symbol.clone();
                self.expect = Arguments;
            }
            (FieldOrMethod, _, _, Blank) => return,
            (FieldOrMethod, _, _, _) => return self.error(tok),

            (Value, _, 0, Delim(';')) => return self.error(tok),
            (Value, _, _, Blank) => return,
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
            Code => self.push(tok),
            Block => {
                assert!(self.object.is_some());
                self.ident = Some(tok.clone());
                let object = self.object.clone().unwrap();
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
            }
            Value => {
                assert!(self.field.is_some());
                assert!(self.symbol.is_some());
                assert_eq!(tok.as_str(), "=");
                let field = self.field.clone().unwrap();
                self.push(Token::raw(Blank, "\t\t"));
                self.push(field.to_synth());
                self.push(Token::raw(Blank, " "));
                self.push(Token::raw(Delim('='), "="));
                self.push(Token::raw(Blank, " "));
                self.symbol = None;
            }
            EndOfField => self.push(tok),
            Arguments => {
                assert!(self.ident.is_some());
                assert!(self.method.is_some());
                match self.symbol {
                    Some(_) => {
                        self.symbol = None;
                        let ident = self.ident.clone().unwrap();
                        let method = self.method.clone().unwrap();
                        let mut sym = String::new();
                        sym += ident.as_str();
                        sym.push('.');
                        sym += method.as_str();
                        self.push(Token::raw(Blank, "\t"));
                        self.push(Token::dyn(Name(1), sym));
                        self.push(Token::raw(OpeningGroup, "("));
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
where I: Iterator<Item=Token> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.broken {
            return None;
        }
        if self.output.len() > 0 {
            return Some(self.output.remove(0));
        }
        loop {
            match self.input.next() {
                Some(tok) => {
                    self.balance(&tok);
                    self.token = Some(tok.clone());
                    if !self.broken {
                        self.process(tok);
                    }
                }
                None => {
                    if self.expect.pvcl() || self.groups != 0 || self.blocks != 0 {
                        assert!(self.token.is_some());
                        let token = self.token.clone().unwrap();
                        self.broken = true;
                        return Some(token.turn_bad("incomplete VCL"));
                    }
                    return None;
                }
            }
            if !self.expect.pvcl() || self.output.len() != 0 {
                break;
            }
        }
        assert!(self.output.len() > 0);
        Some(self.output.remove(0))
    }
}
