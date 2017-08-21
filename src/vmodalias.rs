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

use std::collections::HashMap;

use tok::Flow;
use tok::Lexeme::*;
use tok::RcToken;
use tok::Token;

use self::Expected::*;

#[derive(Clone, Copy, PartialEq)]
enum Expected {
    Code,
    Vmod,
    From,
    Alias,
    Path,
    SemiColon,
}

pub struct VmodAlias<I: Iterator<Item=RcToken>> {
    flow: Flow<I>,
    aliases: HashMap<String, String>,
    expect: Expected,
    vmod: Option<RcToken>,
}

impl<I> VmodAlias<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> VmodAlias<I> {
        VmodAlias {
            flow: Flow::new(input),
            aliases: HashMap::new(),
            expect: Code,
            vmod: None,
        }
    }

    fn process(&mut self, rctok: RcToken) -> Option<RcToken> {
        let lex = rctok.borrow().lexeme;
        match (self.expect, self.flow.blocks, self.flow.groups, lex) {
            (_, _, _, Bad) => Some(rctok),

            (Code, 0, 0, Name(0)) => {
                if rctok.borrow().as_str() == "import" {
                    self.expect = Vmod;
                }
                Some(rctok)
            }
            (Code, _, _, Name(1)) => {
                {
                    let tok = rctok.borrow();
                    let tok_str = tok.as_str();
                    for (alias, name) in &self.aliases {
                        if tok_str.starts_with(alias.as_str()) {
                            let idx = tok_str.find('.').unwrap() + 1;
                            let mut real_str = name.clone();
                            real_str.push_str(&tok_str[idx..]);
                            return Some(Token::dyn(Name(1), real_str));
                        }
                    }
                }
                Some(rctok)
            }
            (Code, _, _, _) => Some(rctok),

            // NB. Preserve blanks and comments
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) |
            (_, _, _, Blank) => Some(rctok),

            (Vmod, _, _, Name(0)) => {
                self.expect = From;
                self.vmod = Some(RcToken::clone(&rctok));
                Some(rctok)
            }
            (Vmod, _, _, _) =>
                Some(rctok.borrow().turn_bad("expected vmod name")),

            (From, _, _, Name(0)) => {
                if rctok.borrow().as_str() == "as" {
                    if self.vmod.is_none() {
                        return Some(rctok.borrow()
                            .turn_bad("expected 'from' or ';'"))
                    }
                    self.expect = Alias;
                    return None;
                }
                if rctok.borrow().as_str() == "from" {
                    self.expect = Path;
                    self.vmod = None;
                    return Some(rctok);
                }
                Some(rctok.borrow().turn_bad("expected 'from' or 'as'"))
            }
            (From, _, _, Delim(';')) => {
                self.expect = Code;
                Some(rctok)
            }
            (From, _, _, _) =>
                Some(rctok.borrow().turn_bad("expected 'from' or 'as'")),

            (Alias, _, _, Name(0)) => {
                let vmod = self.vmod.take().unwrap();
                let name = format!("{}.", vmod.borrow().as_str());
                let alias = format!("{}.", rctok.borrow().as_str());
                if self.aliases.insert(alias, name).is_some() {
                    unimplemented!()
                }
                self.expect = From;
                None
            }
            (Alias, _, _, _) =>
                Some(rctok.borrow().turn_bad("unexpected vmod alias")),

            (Path, _, _, SimpleString) |
            (Path, _, _, BlockString) => {
                self.expect = SemiColon;
                Some(rctok)
            }
            (Path, _, _, _) =>
                Some(rctok.borrow().turn_bad("unexpected vmod path")),

            (SemiColon, _, _, Delim(';')) => {
                self.expect = Code;
                Some(rctok)
            }
            (SemiColon, _, _, _) =>
                Some(rctok.borrow().turn_bad("expected ';'")),
        }
    }
}

impl<I> Iterator for VmodAlias<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rctok = None;
        while rctok.is_none() {
            rctok = match self.flow.next() {
                Some(rctok) => self.process(rctok),
                None => {
                    if self.expect != Code {
                        return self.flow.incomplete();
                    }
                    break;
                }
            };
        }
        rctok
    }
}
