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
use std::io::stdin;
use std::ops::Range;
use std::str::Chars;
use std::string::String;

use Handling::*;
use Kind::*;

/* ------------------------------------------------------------------- */

#[derive(Clone, Copy)]
struct Position {
    line: usize,
    column: usize,
    offset: usize,
}

impl Position {
    fn new() -> Position {
        Position {
            line: 1,
            column: 0,
            offset: 0,
        }
    }

    fn consume(&mut self, c: char) {
        match c {
            '\n' => {
                self.line += 1;
                self.column = 0;
            },
            _ => self.column += 1, // XXX: not quite true
        }
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
enum Kind {
    Blank,
    Name,
    Prop,
    Integer,
    Number,
    Delim,
    SimpleString,
    MultilineString,
    Comment,
    CComment,
    CxxComment,
    OpeningBracket,
    ClosingBracket,
    OpeningCurlyBrace,
    ClosingCurlyBrace,
    Bad(&'static str),
}

#[derive(Clone, Copy)]
struct Token {
    kind: Option<Kind>,
    start: Position,
    end: Position,
}

impl Token {
    fn to_range(&self) -> Range<usize> {
        self.start.offset..self.end.offset
    }
}

/* ------------------------------------------------------------------- */

enum Handling {
    NeedsMore,
    HasChar,
    CurrentReady,
    PreviousReady,
    Done,
    Dry,
}

struct Tokenizer<'a> {
    chars: Chars<'a>,
    token: Token,
    previous: Option<char>,
    handling: Handling,
}

impl<'a> Tokenizer<'a> {

    fn new(chars: Chars) -> Tokenizer {
        Tokenizer {
            chars: chars,
            token: Token {
                kind: None,
                start: Position::new(),
                end: Position::new(),
            },
            previous: None,
            handling: NeedsMore,
        }
    }

    fn next_state(&self, c: char) -> (Option<Kind>, Handling) {
        match (self.token.kind, self.previous, c) {
            (None, _, ' ')  |
            (None, _, '\n') |
            (None, _, '\r') |
            (None, _, '\t') => (Some(Blank), CurrentReady),
            (None, _, 'a'...'z') |
            (None, _, 'A'...'Z') => (Some(Name), NeedsMore),
            (None, _, '0'...'9') => (Some(Integer), NeedsMore),
            (None, _, '.') => (Some(Prop), CurrentReady),
            (None, _, '/') => (Some(Delim), NeedsMore),
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
            (None, _, ';') => (Some(Delim), CurrentReady),
            (None, _, '"') => (Some(SimpleString), NeedsMore),
            (None, _, '#') => (Some(Comment), NeedsMore),
            (None, _, '(') => (Some(OpeningBracket), CurrentReady),
            (None, _, ')') => (Some(ClosingBracket), CurrentReady),
            (None, _, '{') => (Some(OpeningCurlyBrace), NeedsMore),
            (None, _, '}') => (Some(ClosingCurlyBrace), CurrentReady),

            (None, _, _) => (Some(Bad("unexpected character")), Done),

            (Some(OpeningCurlyBrace), Some('{'), '"')
                => (Some(MultilineString), NeedsMore),
            (Some(OpeningCurlyBrace), _, _)
                => (Some(OpeningCurlyBrace), PreviousReady),

            (Some(Delim), Some('/'), '*') => (Some(CComment), NeedsMore),
            (Some(Delim), Some('/'), '/') => (Some(CxxComment), NeedsMore),
            (Some(Delim), Some('/'), _) => (Some(Delim), PreviousReady),

            (Some(Name), Some('.'), '.') => (Some(Bad("invalid name")), Done),
            (Some(Name), _, 'a'...'z') |
            (Some(Name), _, 'A'...'Z') |
            (Some(Name), _, '0'...'9') |
            (Some(Name), _, '_') |
            (Some(Name), _, '-') |
            (Some(Name), _, '.') => (Some(Name), NeedsMore),
            (Some(Name), _, _) => (Some(Name), PreviousReady),

            (Some(Integer), _, '.') => (Some(Number), NeedsMore),
            (Some(Integer), _, '0'...'9') => (Some(Integer), NeedsMore),
            (Some(Integer), _, _) => (Some(Integer), PreviousReady),

            (Some(Number), _, '.') => (Some(Bad("invalid number")), Done),
            (Some(Number), _, '0'...'9') => (Some(Number), NeedsMore),
            (Some(Number), _, _) => (Some(Number), PreviousReady),

            (Some(SimpleString), _, '\n')
                => (Some(Bad("invalid string")), Done),
            (Some(SimpleString), _, '"')
                => (Some(SimpleString), CurrentReady),
            (Some(SimpleString), _, _) => (Some(SimpleString), NeedsMore),

            (Some(MultilineString), Some('"'), '}')
                => (Some(MultilineString), CurrentReady),
            (Some(MultilineString), _, _)
                => (Some(MultilineString), NeedsMore),

            (Some(Comment), _, '\n') => (Some(Comment), PreviousReady),
            (Some(Comment), _, _) => (Some(Comment), NeedsMore),

            (Some(CComment), Some('*'), '/')
                => (Some(CComment), CurrentReady),
            (Some(CComment), _, _) => (Some(CComment), NeedsMore),

            (Some(CxxComment), _, '\n') => (Some(CxxComment), PreviousReady),
            (Some(CxxComment), _, _) => (Some(CxxComment), NeedsMore),

            (_, _, _) => {
                panic!("{:?}, {:?}, '{}'",
                    self.token.kind, self.previous, c)
            }
        }
    }

    fn next_char(&mut self) {

        let c = match self.handling {
            NeedsMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    if self.token.kind.is_some() {
                        self.token.kind = Some(Bad("incomplete VCL"));
                        self.handling = Done;
                    }
                    else {
                        self.handling = Dry;
                    }
                    return;
                }
            },
            HasChar => {
                assert!(self.previous.is_some());
                self.previous.unwrap()
            },
            _ => panic!()
        };

        let (kind, handling) = self.next_state(c);

        match handling {
            PreviousReady => (),
            HasChar => panic!(),
            _ => {
                self.token.end.consume(c);
            }
        }

        if self.token.kind.is_none() {
            self.token.start.move_cursor_to(&self.token.end);
        }

        self.handling = handling;
        self.token.kind = kind;
        self.previous = Some(c);
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.handling {
            CurrentReady => {
                self.handling = NeedsMore;
                self.token.kind = None;
                self.token.start = self.token.end;
            },
            PreviousReady => {
                self.handling = HasChar;
                self.token.kind = None;
                self.token.start = self.token.end;
            },
            Done | Dry => return None,
            _ => ()
        }

        loop {
            match self.handling {
                NeedsMore | HasChar => self.next_char(),
                Done => return Some(self.token),
                Dry => return None,
                _ => break
            }
        }

        match self.handling {
            CurrentReady | PreviousReady => Some(self.token),
            _ => panic!()
        }
    }
}

/* ------------------------------------------------------------------- */

// TODO: preprocessor

/* ------------------------------------------------------------------- */

fn main() {
    let mut buf = String::new();

    loop {
        match stdin().read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) => panic!("error: {}", e)
        }
    }

    for tok in Tokenizer::new(buf.chars()) {
        match tok.kind {
            Some(_) => print!("[{}...{}] ", tok.start, tok.end),
            None => panic!()
        }
        match tok.kind {
            Some(Bad(s)) => println!("bad token: {}", s),
            Some(_) => {
                println!("token: {:?} '{}'", tok.kind,
                    &buf.as_str()[tok.to_range()]);
            }
            None => ()
        }
    }
}
