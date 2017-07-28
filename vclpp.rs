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
use std::io::BufWriter;
use std::io::Write;
use std::io::stdin;
use std::io::stdout;
use std::ops::Index;
use std::str::Chars;
use std::string::String;

use Expected::*;
use Handling::*;
use Lexeme::*;

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
enum Lexeme {
    Blank,
    Name,
    Prop,
    Integer,
    Number,
    Delim,
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
struct Token {
    lexeme: Option<Lexeme>,
    start: Position,
    end: Position,
}

impl<'a> Index<&'a Token> for String {
    type Output = str;

    fn index(&self, tok: &'a Token) -> &str {
        &self.as_str()[tok.start.offset..tok.end.offset]
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
    previous: char,
    handling: Handling,
}

impl<'a> Tokenizer<'a> {

    fn new(chars: Chars) -> Tokenizer {
        Tokenizer {
            chars: chars,
            token: Token {
                lexeme: None,
                start: Position::new(),
                end: Position::new(),
            },
            previous: '?', // doesn't matter when token.lexeme is None
            handling: NeedsMore,
        }
    }

    fn next_state(&self, c: char) -> (Option<Lexeme>, Handling) {
        match (self.token.lexeme, self.previous, c) {
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
            (None, _, '(') => (Some(OpeningGroup), CurrentReady),
            (None, _, ')') => (Some(ClosingGroup), CurrentReady),
            (None, _, '{') => (Some(OpeningBlock), NeedsMore),
            (None, _, '}') => (Some(ClosingBlock), CurrentReady),

            (None, _, _) => (Some(Bad("unexpected character")), Done),

            (Some(OpeningBlock), '{', '"') => (Some(BlockString), NeedsMore),
            (Some(OpeningBlock), _, _) => (Some(OpeningBlock), PreviousReady),

            (Some(Delim), '/', '*') => (Some(CComment), NeedsMore),
            (Some(Delim), '/', '/') => (Some(CxxComment), NeedsMore),
            (Some(Delim), '/', _) => (Some(Delim), PreviousReady),

            (Some(Name), '.', '.') => (Some(Bad("invalid name")), Done),
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

            (Some(BlockString), '"', '}') => (Some(BlockString), CurrentReady),
            (Some(BlockString), _, _) => (Some(BlockString), NeedsMore),

            (Some(Comment), _, '\n') => (Some(Comment), PreviousReady),
            (Some(Comment), _, _) => (Some(Comment), NeedsMore),

            (Some(CComment), '*', '/') => (Some(CComment), CurrentReady),
            (Some(CComment), _, _) => (Some(CComment), NeedsMore),

            (Some(CxxComment), _, '\n') => (Some(CxxComment), PreviousReady),
            (Some(CxxComment), _, _) => (Some(CxxComment), NeedsMore),

            (_, _, _) => {
                panic!("{:?}, '{}', '{}'",
                    self.token.lexeme, self.previous, c)
            }
        }
    }

    fn next_char(&mut self) {
        let c = match self.handling {
            NeedsMore => match self.chars.next() {
                Some(c) => c,
                None => {
                    if self.token.lexeme.is_some() {
                        self.token.lexeme = Some(Bad("incomplete VCL"));
                        self.handling = Done;
                    }
                    else {
                        self.handling = Dry;
                    }
                    return;
                }
            },
            HasChar => self.previous,
            _ => panic!()
        };

        let (lexeme, handling) = self.next_state(c);

        match handling {
            PreviousReady => (),
            HasChar => panic!(),
            _ => {
                self.token.end.consume(c);
            }
        }

        if self.token.lexeme.is_none() {
            self.token.start.move_cursor_to(&self.token.end);
        }

        self.handling = handling;
        self.token.lexeme = lexeme;
        self.previous = c;
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.handling {
            CurrentReady => {
                self.handling = NeedsMore;
                self.token.lexeme = None;
                self.token.start = self.token.end;
            },
            PreviousReady => {
                self.handling = HasChar;
                self.token.lexeme = None;
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

#[derive(Clone, Copy, Debug)]
enum Expected {
    Code,
}

struct Preprocessor {
    expect: Expected,
    groups: isize,
    blocks: isize,
}

impl Preprocessor {
    fn exec<'a, W: Write>(src: &'a String, mut out: BufWriter<W>) {
        let mut pp = Preprocessor {
            expect: Code,
            groups: 0,
            blocks: 0,
        };
        for tok in Tokenizer::new(src.chars()) {
            assert!(tok.lexeme.is_some());
            match (pp.expect, tok.lexeme.unwrap()) {
                (Code, OpeningGroup) => pp.groups += 1,
                (Code, ClosingGroup) => pp.groups -= 1,
                (Code, OpeningBlock) => pp.blocks += 1,
                (Code, ClosingBlock) => pp.blocks -= 1,
                (_, _) => (),
            }
            match (pp.groups, pp.blocks) {
                (-1, _) => unimplemented!(),
                (_, -1) => unimplemented!(),
                (_, 0) => {
                    if pp.groups > 0 {
                        unimplemented!();
                    }
                }
                (_, _) => (),
            }
            match pp.expect {
                Code => write!(out, "{}", &src[&tok]),
            };
        }
        if pp.groups != 0 || pp.blocks != 0 {
            unimplemented!();
        }
    }
}

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

    Preprocessor::exec(&buf, BufWriter::new(stdout()));
}
