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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lexeme {
    Blank,
    Name(usize), // number of dots in a name
    Prop,
    Integer,
    Number,
    Delim(char),
    SimpleString,
    BlockString,
    InlineC(bool), // are we already in C code?
    Comment,
    CComment,
    CxxComment,
    OpeningGroup,
    ClosingGroup,
    OpeningBlock,
    ClosingBlock,
    Bad
}

#[derive(Clone)]
pub struct Token {
    pub lexeme: Lexeme,
    pub start: Position,
    pub end: Position,
    text: String,
}

impl Token {
    pub fn turn_bad(&self, msg: &'static str) -> Self {
        assert!(self.lexeme != Bad);
        assert!(!self.synthetic());
        Token {
            lexeme: Bad,
            start: self.start,
            end: self.end,
            text: msg.to_string(),
        }
    }

    pub fn raw(lex: Lexeme, msg: &'static str) -> Self {
        Token {
            lexeme: lex,
            start: Position::new(),
            end: Position::new(),
            text: msg.to_string(),
        }
    }

    pub fn dyn(lex: Lexeme, msg: String) -> Self {
        Token {
            lexeme: lex,
            start: Position::new(),
            end: Position::new(),
            text: msg,
        }
    }

    pub fn to_synth(&self) -> Self {
        Self::dyn(self.lexeme, self.text.clone())
    }

    pub fn as_str<'a>(&'a self) -> &'a str {
        self.text.as_str()
    }

    fn synthetic(&self) -> bool {
        self.start.line == 0
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
}

#[must_use = "tokenizers are lazy and do nothing unless consumed"]
pub struct Tokenizer<'a> {
    chars: Chars<'a>,
    lexeme: Option<Lexeme>,
    text: Option<String>,
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
            text: Some(String::new()),
            start: Position::new(),
            end: Position::new(),
            previous: '?', // doesn't matter when lexeme is None
            handling: NeedsMore,
        }
    }

    pub fn error(&mut self, msg: &'static str) -> Lexeme {
        match self.text {
            Some(ref mut text) => {
                text.clear();
                text.push_str(msg);
            }
            None => unreachable!()
        }
        Bad
    }

    fn to_token(&mut self) -> Token {
        assert!(self.lexeme.is_some());
        assert!(self.text.is_some());
        let text = self.text.take().unwrap();
        self.text = Some(String::new());
        Token {
            lexeme: self.lexeme.unwrap(),
            start: self.start,
            end: self.end,
            text: text,
        }
    }

    fn next_state(&mut self, c: char) -> (Lexeme, Handling) {
        if self.lexeme.is_none() {
            return match c {
                ' '  |
                '\n' |
                '\r' |
                '\t' => (Blank, MayNeedMore),
                'C' => (InlineC(false), MayNeedMore),
                'a'...'z' |
                'A'...'Z' => (Name(0), MayNeedMore),
                '0'...'9' => (Integer, MayNeedMore),
                '.' => (Prop, CurrentReady),
                '/' => (Delim('/'), MayNeedMore),
                '+' |
                '-' |
                '*' |
                '=' |
                '<' |
                '>' |
                '~' |
                '!' |
                '&' |
                '|' |
                ',' |
                ';' => (Delim(c), CurrentReady),
                '"' => (SimpleString, NeedsMore),
                '#' => (Comment, NeedsMore),
                '(' => (OpeningGroup, CurrentReady),
                ')' => (ClosingGroup, CurrentReady),
                '{' => (OpeningBlock, MayNeedMore),
                '}' => (ClosingBlock, CurrentReady),
                _ => (self.error("unexpected character"), Done),
            };
        }
        match (self.lexeme.unwrap(), self.previous, c) {
            (Blank, _, ' ')  |
            (Blank, _, '\n') |
            (Blank, _, '\r') |
            (Blank, _, '\t') => (Blank, MayNeedMore),
            (Blank, _, _) => (Blank, PreviousReady),

            (OpeningBlock, '{', '"') => (BlockString, NeedsMore),
            (OpeningBlock, _, _) => (OpeningBlock, PreviousReady),

            (Delim(_), '/', '*') => (CComment, NeedsMore),
            (Delim(_), '/', '/') => (CxxComment, NeedsMore),
            (Delim(_), '/', _) => (Delim('/'), PreviousReady),

            (Name(_), '.', '.') => (self.error("invalid name"), Done),
            (Name(d), _, 'a'...'z') |
            (Name(d), _, 'A'...'Z') |
            (Name(d), _, '0'...'9') |
            (Name(d), _, '_') |
            (Name(d), _, '-') => (Name(d), MayNeedMore),
            (Name(d), _, '.') => (Name(d+1), NeedsMore),
            (Name(_), '.', _) => (self.error("invalid name"), Done),
            (Name(d), _, _) => (Name(d), PreviousReady),

            (Integer, _, '.') => (Number, MayNeedMore),
            (Integer, _, '0'...'9') => (Integer, MayNeedMore),
            (Integer, _, _) => (Integer, PreviousReady),

            (Number, _, '.') => (self.error("invalid number"), Done),
            (Number, _, '0'...'9') => (Number, MayNeedMore),
            (Number, _, _) => (Number, PreviousReady),

            (SimpleString, _, '\n') => (self.error("invalid string"), Done),
            (SimpleString, _, '"') => (SimpleString, CurrentReady),
            (SimpleString, _, _) => (SimpleString, NeedsMore),

            (BlockString, '"', '}') => (BlockString, CurrentReady),
            (BlockString, _, _) => (BlockString, NeedsMore),

            (InlineC(false), 'C', '{') => (InlineC(true), NeedsMore),
            (InlineC(false), 'C', _) => {
                self.lexeme = Some(Name(0));
                self.next_state(c)
            }
            (InlineC(false), _, _) => unreachable!(),

            (InlineC(true), '}', 'C') => (InlineC(true), CurrentReady),
            (InlineC(true), _, _) => (InlineC(true), NeedsMore),

            (Comment, _, '\n') => (Comment, CurrentReady),
            (Comment, _, _) => (Comment, MayNeedMore),

            (CComment, '*', '/') => (CComment, CurrentReady),
            (CComment, _, _) => (CComment, NeedsMore),

            (CxxComment, _, '\n') => (CxxComment, CurrentReady),
            (CxxComment, _, _) => (CxxComment, MayNeedMore),

            (_, _, _) => {
                unreachable!("{:?}, '{}', '{}'", self.lexeme, self.previous, c)
            }
        }
    }

    fn consume(&mut self, c: char) {
        match self.text {
            Some(ref mut text) => text.push(c),
            None => unreachable!(),
        }
        self.end.consume(c);
    }

    fn next_char(&mut self) {
        let c = match self.handling {
            NeedsMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    self.lexeme = Some(self.error("incomplete VCL"));
                    self.handling = Done;
                    return;
                }
            },
            MayNeedMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    let (lexeme, _) = self.next_state('\0');
                    self.lexeme = Some(lexeme);
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
            _ => self.consume(c),
        }

        if self.lexeme.is_none() {
            self.start.move_cursor_to(&self.end);
        }

        self.handling = handling;
        self.lexeme = Some(lexeme);
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
            Done => return None,
            _ => ()
        }

        loop {
            match self.handling {
                MayNeedMore |
                NeedsMore |
                HasChar => self.next_char(),
                Done => return Some(self.to_token()),
                _ => break
            }
        }

        match self.handling {
            CurrentReady | PreviousReady => Some(self.to_token()),
            _ => unreachable!()
        }
    }
}
