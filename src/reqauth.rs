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

#[must_use = "preprocessors are lazy and do nothing unless consumed"]
pub struct RequestAuthority<I: Iterator<Item=RcToken>>(Flow<I>);

impl<I> RequestAuthority<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> RequestAuthority<I> {
        RequestAuthority(Flow::new(input))
    }
}

impl<I> RequestAuthority<I>
where I: Iterator<Item=RcToken> {

    fn process(&mut self, rctok: RcToken) -> RcToken {
        if rctok.borrow().lexeme == Name(1) {
            let tok = rctok.borrow();

            if tok.as_str() == "req.authority" {
                return Token::raw(tok.lexeme, "req.http.host");
            }

            if tok.as_str() == "bereq.authority" {
                return Token::raw(tok.lexeme, "bereq.http.host");
            }
        }

        rctok
    }
}

impl<I> Iterator for RequestAuthority<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            Some(rc) => Some(self.process(rc)),
            None => None,
        }
    }
}
