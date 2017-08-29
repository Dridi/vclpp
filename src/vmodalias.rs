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

#[must_use = "preprocessors are lazy and do nothing unless consumed"]
pub struct VmodAlias<I: Iterator<Item=RcToken>> {
    flow: Flow<I>,
    aliases: HashMap<String, String>,
    expect: Expected,
    broken: bool,
    vmod: Option<RcToken>,
}

impl<I> VmodAlias<I>
where I: Iterator<Item=RcToken> {
    pub fn new(input: I) -> VmodAlias<I> {
        VmodAlias {
            flow: Flow::new(input),
            aliases: HashMap::new(),
            expect: Code,
            broken: false,
            vmod: None,
        }
    }

    fn process(&mut self, tok: RcToken) -> Option<RcToken> {
        let lex = tok.lexeme;
        match (self.expect, self.flow.blocks, self.flow.groups, lex) {
            (_, _, _, Bad) => Some(tok),

            (Code, 0, 0, Name(0)) => {
                if tok.as_str() == "import" {
                    self.expect = Vmod;
                }
                Some(tok)
            }
            (Code, _, _, Name(1)) => {
                {
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
                Some(tok)
            }
            (Code, _, _, _) => Some(tok),

            // NB. Preserve blanks and comments
            (_, _, _, Comment) |
            (_, _, _, CComment) |
            (_, _, _, CxxComment) |
            (_, _, _, Blank) => Some(tok),

            (Vmod, _, _, Name(0)) => {
                self.expect = From;
                self.vmod = Some(RcToken::clone(&tok));
                Some(tok)
            }
            (Vmod, _, _, _) =>
                Some(self.flow.bust("expected vmod name")),

            (From, _, _, Name(0)) => {
                if tok.as_str() == "as" {
                    if self.vmod.is_none() {
                        return Some(self.flow.bust("expected 'from' or ';'"))
                    }
                    self.expect = Alias;
                    return None;
                }
                if tok.as_str() == "from" {
                    self.expect = Path;
                    self.vmod = None;
                    return Some(tok);
                }
                Some(self.flow.bust("expected 'from', 'as' or ';'"))
            }
            (From, _, _, Delim(';')) => {
                self.expect = Code;
                Some(tok)
            }
            (From, _, _, _) =>
                Some(self.flow.bust("expected 'from', 'as' or ';'")),

            (Alias, _, _, Name(0)) => {
                let vmod = self.vmod.take().unwrap();
                let name = format!("{}.", vmod.as_str());
                let alias = format!("{}.", tok.as_str());
                if self.aliases.insert(alias, name).is_some() {
                    return Some(self.flow.bust("duplicate alias"));
                }
                self.expect = From;
                None
            }
            (Alias, _, _, _) => Some(self.flow.bust("expected vmod alias")),

            (Path, _, _, SimpleString) |
            (Path, _, _, BlockString) => {
                self.expect = SemiColon;
                Some(tok)
            }
            (Path, _, _, _) => Some(self.flow.bust("expected vmod path")),

            (SemiColon, _, _, Delim(';')) => {
                self.expect = Code;
                Some(tok)
            }
            (SemiColon, _, _, _) => Some(self.flow.bust("expected ';'")),
        }
    }
}

impl<I> Iterator for VmodAlias<I>
where I: Iterator<Item=RcToken> {
    type Item = RcToken;

    fn next(&mut self) -> Option<Self::Item> {
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
                    if !self.broken && self.expect != Code {
                        self.broken = true;
                        return self.flow.incomplete();
                    }
                    break;
                }
            };
        }
        tok
    }
}
