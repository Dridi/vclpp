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

use std::fmt;
use std::ops::Index;
use std::str::Chars;

use self::Handling::*;
use self::Lexeme::*;

/* ------------------------------------------------------------------- */

#[derive(Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    offset: usize,
    newline: bool,
}

impl Position {
    fn new() -> Position {
        Position {
            line: 0,
            column: 0,
            offset: 0,
            newline: true,
        }
    }

    fn consume(&mut self, c: char) {
        if self.newline {
            self.line += 1;
            self.column = 0;
        }
        self.newline = c == '\n';
        self.column += 1; // XXX: not quite true
        self.offset += c.len_utf8();
    }

    fn move_cursor_to(&mut self, p: &Position) {
        self.line = p.line;
        self.column = p.column;
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.line, self.column)
    }
}

/* ------------------------------------------------------------------- */

#[derive(Clone, Copy, Debug)]
pub enum Lexeme {
    Blank,
    Name(usize), // number of dots in a name
    Prop,
    Integer,
    Number,
    Delim(char),
    SimpleString,
    BlockString,
    Comment,
    CComment,
    CxxComment,
    OpeningGroup,
    ClosingGroup,
    OpeningBlock,
    ClosingBlock,
    Bad(&'static str),
}

#[derive(Clone, Copy)]
pub struct Token {
    pub lexeme: Lexeme,
    pub start: Position,
    pub end: Position,
}

impl<'a> Index<&'a Token> for String {
    type Output = str;

    fn index(&self, tok: &'a Token) -> &str {
        &self.as_str()[tok.start.offset..tok.end.offset]
    }
}

/* ------------------------------------------------------------------- */

enum Handling {
    MayNeedMore,
    NeedsMore,
    HasChar,
    CurrentReady,
    PreviousReady,
    Done,
    Dry,
}

pub struct Tokenizer<'a> {
    chars: Chars<'a>,
    lexeme: Option<Lexeme>,
    start: Position,
    end: Position,
    previous: char,
    handling: Handling,
}

impl<'a> Tokenizer<'a> {
    pub fn new(chars: Chars) -> Tokenizer {
        Tokenizer {
            chars: chars,
            lexeme: None,
            start: Position::new(),
            end: Position::new(),
            previous: '?', // doesn't matter when lexeme is None
            handling: NeedsMore,
        }
    }

    fn to_token(&self) -> Token {
        assert!(self.lexeme.is_some());
        Token {
            lexeme: self.lexeme.unwrap(),
            start: self.start,
            end: self.end,
        }
    }

    fn next_state(&self, c: char) -> (Option<Lexeme>, Handling) {
        match (self.lexeme, self.previous, c) {
            (None, _, ' ')  |
            (None, _, '\n') |
            (None, _, '\r') |
            (None, _, '\t') => (Some(Blank), CurrentReady),
            (None, _, 'a'...'z') |
            (None, _, 'A'...'Z') => (Some(Name(0)), MayNeedMore),
            (None, _, '0'...'9') => (Some(Integer), MayNeedMore),
            (None, _, '.') => (Some(Prop), CurrentReady),
            (None, _, '/') => (Some(Delim('/')), MayNeedMore),
            (None, _, '+') |
            (None, _, '-') |
            (None, _, '*') |
            (None, _, '=') |
            (None, _, '<') |
            (None, _, '>') |
            (None, _, '~') |
            (None, _, '!') |
            (None, _, '&') |
            (None, _, '|') |
            (None, _, ',') |
            (None, _, ';') => (Some(Delim(c)), CurrentReady),
            (None, _, '"') => (Some(SimpleString), NeedsMore),
            (None, _, '#') => (Some(Comment), NeedsMore),
            (None, _, '(') => (Some(OpeningGroup), CurrentReady),
            (None, _, ')') => (Some(ClosingGroup), CurrentReady),
            (None, _, '{') => (Some(OpeningBlock), MayNeedMore),
            (None, _, '}') => (Some(ClosingBlock), CurrentReady),

            (None, _, _) => (Some(Bad("unexpected character")), Done),

            (Some(OpeningBlock), '{', '"') => (Some(BlockString), NeedsMore),
            (Some(OpeningBlock), _, _) => (Some(OpeningBlock), PreviousReady),

            (Some(Delim(_)), '/', '*') => (Some(CComment), NeedsMore),
            (Some(Delim(_)), '/', '/') => (Some(CxxComment), NeedsMore),
            (Some(Delim(_)), '/', _) => (Some(Delim('/')), PreviousReady),

            (Some(Name(_)), '.', '.') => (Some(Bad("invalid name")), Done),
            (Some(Name(d)), _, 'a'...'z') |
            (Some(Name(d)), _, 'A'...'Z') |
            (Some(Name(d)), _, '0'...'9') |
            (Some(Name(d)), _, '_') |
            (Some(Name(d)), _, '-') => (Some(Name(d)), MayNeedMore),
            (Some(Name(d)), _, '.') => (Some(Name(d+1)), NeedsMore),
            (Some(Name(_)), '.', _) => (Some(Bad("invalid name")), Done),
            (Some(Name(d)), _, _) => (Some(Name(d)), PreviousReady),

            (Some(Integer), _, '.') => (Some(Number), MayNeedMore),
            (Some(Integer), _, '0'...'9') => (Some(Integer), MayNeedMore),
            (Some(Integer), _, _) => (Some(Integer), PreviousReady),

            (Some(Number), _, '.') => (Some(Bad("invalid number")), Done),
            (Some(Number), _, '0'...'9') => (Some(Number), MayNeedMore),
            (Some(Number), _, _) => (Some(Number), PreviousReady),

            (Some(SimpleString), _, '\n')
                => (Some(Bad("invalid string")), Done),
            (Some(SimpleString), _, '"')
                => (Some(SimpleString), CurrentReady),
            (Some(SimpleString), _, _) => (Some(SimpleString), NeedsMore),

            (Some(BlockString), '"', '}') => (Some(BlockString), CurrentReady),
            (Some(BlockString), _, _) => (Some(BlockString), NeedsMore),

            (Some(Comment), _, '\n') => (Some(Comment), CurrentReady),
            (Some(Comment), _, _) => (Some(Comment), NeedsMore),

            (Some(CComment), '*', '/') => (Some(CComment), CurrentReady),
            (Some(CComment), _, _) => (Some(CComment), NeedsMore),

            (Some(CxxComment), _, '\n') => (Some(CxxComment), CurrentReady),
            (Some(CxxComment), _, _) => (Some(CxxComment), NeedsMore),

            (_, _, _) => {
                unreachable!("{:?}, '{}', '{}'", self.lexeme, self.previous, c)
            }
        }
    }

    fn next_char(&mut self) {
        let c = match self.handling {
            NeedsMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    if self.lexeme.is_some() {
                        self.lexeme = Some(Bad("incomplete VCL"));
                        self.handling = Done;
                    }
                    else {
                        self.handling = Dry;
                    }
                    return;
                }
            },
            MayNeedMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    let (lexeme, _) = self.next_state('\0');
                    self.lexeme = lexeme;
                    self.handling = Done;
                    return;
                }
            },
            HasChar => self.previous,
            _ => unreachable!()
        };

        let (lexeme, handling) = self.next_state(c);

        match handling {
            PreviousReady => (),
            HasChar => unreachable!(),
            _ => self.end.consume(c),
        }

        if self.lexeme.is_none() {
            self.start.move_cursor_to(&self.end);
        }

        self.handling = handling;
        self.lexeme = lexeme;
        self.previous = c;
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.handling {
            CurrentReady => {
                self.handling = NeedsMore;
                self.lexeme = None;
                self.start = self.end;
            },
            PreviousReady => {
                self.handling = HasChar;
                self.lexeme = None;
                self.start = self.end;
            },
            Done | Dry => return None,
            _ => ()
        }

        loop {
            match self.handling {
                MayNeedMore |
                NeedsMore |
                HasChar => self.next_char(),
                Done => return Some(self.to_token()),
                Dry => return None,
                _ => break
            }
        }

        match self.handling {
            CurrentReady | PreviousReady => Some(self.to_token()),
            _ => unreachable!()
        }
    }
}
