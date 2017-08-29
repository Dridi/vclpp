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
    Open,
    Header,
    Close,
}

#[must_use = "preprocessors are lazy and do nothing unless consumed"]
pub struct HeaderArray<I: Iterator<Item=RcToken>> {
    flow: Flow<I>,
    expect: Expected,
    broken: bool,
    token: Option<RcToken>,
    header: Option<RcToken>,
}

impl<I> HeaderArray<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> HeaderArray<I> {
        HeaderArray {
            flow: Flow::new(input),
            expect: Code,
            broken: false,
            token: None,
            header: None,
        }
    }

    fn process(&mut self, tok: RcToken) -> Option<RcToken> {
        match (self.expect, self.flow.blocks, self.flow.groups, tok.lexeme) {
            (_, _, _, Bad) => Some(tok),

            (Code, 0, _, _) => Some(tok),
            (Code, _, _, Name(1)) => {
                assert!(self.token.is_none());
                assert!(self.header.is_none());
                match tok.as_str() {
                    "obj.http" |
                    "req.http" |
                    "resp.http" |
                    "bereq.http" |
                    "beresp.http" => {
                        self.token = Some(RcToken::clone(&tok));
                        self.expect = Open;
                        None
                    }
                    &_ => Some(tok),
                }
            }
            (Code, _, _, _) => Some(tok),

            (Open, _, _, OpeningArray) => {
                assert!(self.token.is_some());
                assert!(self.header.is_none());
                self.expect = Header;
                None
            }
            (Open, _, _, _) => Some(self.flow.bust("expected '[' or '.'")),

            (Header, _, _, Name(0)) => {
                self.expect = Close;
                self.header = Some(RcToken::clone(&tok));
                None
            }
            (Header, _, _, _) => Some(self.flow.bust("expected header name")),

            (Close, _, _, ClosingArray) => {
                assert!(self.token.is_some());
                assert!(self.header.is_some());
                let var = self.token.take().unwrap();
                let hdr = self.header.take().unwrap();
                let tok = format!("{}.{}", var.as_str(),
                    hdr.as_str());
                self.expect = Code;
                Some(Token::dyn(Name(2), tok))
            }
            (Close, _, _, _) => Some(self.flow.bust("expected ']'")),
        }
    }
}

impl<I> Iterator for HeaderArray<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.broken {
            return None;
        }
        let mut tok = None;
        while tok.is_none() {
            tok = match self.flow.next() {
                Some(tok) => {
                    match self.process(tok) {
                        Some(res) => {
                            self.broken |= res.lexeme == Bad;
                            Some(res)
                        }
                        None => None
                    }
                }
                None => {
                    assert!(self.expect == Code);
                    break;
                }
            };
        }
        tok
    }
}
