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
use tok::RcToken;

pub struct BracketCheck<I: Iterator<Item=RcToken>> {
    input: I,
    groups: isize,
    blocks: isize,
}

impl<I> BracketCheck<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> BracketCheck<I> {
        BracketCheck {
            input: input,
            groups: 0,
            blocks: 0,
        }
    }
}

impl<I> Iterator for BracketCheck<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.groups < 0 || self.blocks < 0 {
            return None;
        }

        let rctok = match self.input.next() {
            Some(tok) => tok,
            None => return None,
        };

        {
            let tok = rctok.borrow();

            if tok.lexeme == OpeningBlock && self.groups > 0 {
                self.groups = -1;
                return Some(tok.turn_bad("opening a block inside an expression"));
            }

            match tok.lexeme {
                OpeningGroup => self.groups += 1,
                ClosingGroup => self.groups -= 1,
                OpeningBlock => self.blocks += 1,
                ClosingBlock => self.blocks -= 1,
                _ => (),
            }

            if self.groups < 0 || self.blocks < 0 {
                return Some(tok.turn_bad("unbalanced brackets"));
            }
        }

        Some(rctok)
    }
}
