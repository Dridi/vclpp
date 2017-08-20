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

pub struct BracketCheck<I: Iterator<Item=RcToken>> {
    input: I,
    broken: bool,
    nest: Nest,
    token: Option<RcToken>,
}

impl<I> BracketCheck<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> BracketCheck<I> {
        BracketCheck {
            input: input,
            broken: false,
            nest: Nest::new(),
            token: None,
        }
    }
}

impl<I> BracketCheck<I>
where I: Iterator<Item=RcToken> {

    fn process(&mut self, rctok: RcToken) -> RcToken {
        self.nest.update(&rctok);
        self.token = Some(RcToken::clone(&rctok));

        {
            let tok = rctok.borrow();

            if tok.lexeme == OpeningBlock && self.nest.groups > 0 {
                return tok.turn_bad("block inside an expression");
            }

            if self.nest.groups < 0 || self.nest.blocks < 0 {
                return tok.turn_bad("unbalanced brackets");
            }
        }

        rctok
    }

    fn process_last(&mut self) -> Option<RcToken> {
        #[cfg(kcov)]
        assert!(self.input.next().is_none()); // good behavior?

        return if self.nest.groups != 0 || self.nest.blocks != 0 {
            let last_tok = self.token.take().unwrap();
            let bad_tok = last_tok.borrow().turn_bad("incomplete VCL");
            Some(bad_tok)
        }
        else {
            None
        }
    }
}

impl<I> Iterator for BracketCheck<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.broken {
            return None;
        }

        let rctok = match self.input.next() {
            Some(rc) => Some(self.process(rc)),
            None => self.process_last(),
        };

        self.broken |= match &rctok {
            &Some(ref tok) => tok.borrow().lexeme == Bad,
            &None => false,
        };

        rctok
    }
}
