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
struct Token {
    lexeme: Lexeme,
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
    lexeme: Option<Lexeme>,
    start: Position,
    end: Position,
    previous: char,
    handling: Handling,
}

impl<'a> Tokenizer<'a> {
    fn new(chars: Chars) -> Tokenizer {
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
            (None, _, 'A'...'Z') => (Some(Name(0)), NeedsMore),
            (None, _, '0'...'9') => (Some(Integer), NeedsMore),
            (None, _, '.') => (Some(Prop), CurrentReady),
            (None, _, '/') => (Some(Delim('/')), NeedsMore),
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
            (None, _, '{') => (Some(OpeningBlock), NeedsMore),
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
            (Some(Name(d)), _, '-') => (Some(Name(d)), NeedsMore),
            (Some(Name(d)), _, '.') => (Some(Name(d+1)), NeedsMore),
            (Some(Name(d)), _, _) => (Some(Name(d)), PreviousReady),

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
                    self.lexeme, self.previous, c)
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
            HasChar => self.previous,
            _ => panic!()
        };

        let (lexeme, handling) = self.next_state(c);

        match handling {
            PreviousReady => (),
            HasChar => panic!(),
            _ => {
                self.end.consume(c);
            }
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
                NeedsMore | HasChar => self.next_char(),
                Done => return Some(self.to_token()),
                Dry => return None,
                _ => break
            }
        }

        match self.handling {
            CurrentReady | PreviousReady => Some(self.to_token()),
            _ => panic!()
        }
    }
}

/* ------------------------------------------------------------------- */

#[derive(Clone, Copy, Debug)]
enum Expected {
    Code,
    Ident,
    Block,
    Dot,
    Member,
    FieldOrMethod,
    Value,
    EndOfField,
    Arguments,
    EndOfMethod,
    SemiColon,
}

struct Preprocessor {
    expect: Expected,
    groups: isize,
    blocks: isize,
    ident: Option<Token>,
    object: Option<Token>,
    symbol: Option<Token>,
    field: Option<Token>,
    method: Option<Token>,
}

impl Preprocessor {
    fn new() -> Preprocessor {
        Preprocessor {
            expect: Code,
            groups: 0,
            blocks: 0,
            ident: None,
            object: None,
            symbol: None,
            field: None,
            method: None,
        }
    }

    fn balance(&mut self, tok: Token) -> Result<(), Token> {
        try!(match (tok.lexeme, self.groups) {
            (OpeningBlock, 0) => Ok(()),
            (OpeningBlock, _) => Err(tok),
            (_, _) => Ok(()),
        });
        match tok.lexeme {
            OpeningGroup => self.groups += 1,
            ClosingGroup => self.groups -= 1,
            OpeningBlock => self.blocks += 1,
            ClosingBlock => self.blocks -= 1,
            _ => (),
        }
        assert!(self.groups >= -1);
        assert!(self.blocks >= -1);
        match (self.groups, self.blocks) {
            (-1, _) |
            (_, -1) => Err(tok),
            (0, 0) => Ok(()),
            (_, 0) => Err(tok),
            (_, _) => Ok(()),
        }
    }

    fn exec<'a, W: Write>(src: &'a String, mut out: BufWriter<W>)
    -> Result<(), Token> {
        let mut pp = Preprocessor::new();
        for tok in Tokenizer::new(src.chars()) {
            try!(pp.balance(tok));
            match (pp.expect, pp.blocks, pp.groups, tok.lexeme) {
                (Code, 0, _, Name(0)) => (),
                (Code, 0, _, Name(1)) => {
                    pp.object = Some(tok);
                    pp.expect = Ident;
                }
                (Code, 0, _, Name(_)) => unimplemented!(),
                (Code, _, _, _) => (),

                // NB. Abandon comments inside preprocessed code
                (_, _, _, Comment) |
                (_, _, _, CComment) |
                (_, _, _, CxxComment) => continue,

                (Ident, _, _, Name(0)) => pp.expect = Block,
                (Ident, _, _, Blank) => continue,
                (Ident, _, _, _) => unimplemented!(),

                (Block, _, _, OpeningBlock) => pp.expect = Dot,
                (Block, _, _, Blank) => continue,
                (Block, _, _, _) => unimplemented!(),

                (Dot, _, _, ClosingBlock) => {
                    if pp.field.is_none() && pp.method.is_none() {
                        write!(out, ");\n");
                    }
                    assert!(pp.groups == 0);
                    assert!(pp.blocks == 0);
                    pp = Preprocessor::new();
                }
                (Dot, _, _, Prop) => pp.expect = Member,
                (Dot, _, _, Blank) => continue,
                (Dot, _, _, _) => unimplemented!(),

                (Member, _, _, Name(0)) => {
                    pp.symbol = Some(tok);
                    pp.expect = FieldOrMethod;
                }
                (Member, _, _, Name(_)) => unimplemented!(),
                (Member, _, _, Blank) => continue,
                (Member, _, _, _) => unimplemented!(),

                (FieldOrMethod, _, _, Delim('=')) => {
                    if pp.method.is_some() {
                        return Err(tok);
                    }
                    if pp.field.is_some() {
                        write!(out, ", ");
                    }
                    write!(out, "\n");
                    pp.field = pp.symbol;
                    pp.expect = Value;
                }
                (FieldOrMethod, _, _, OpeningGroup) => {
                    assert!(pp.groups == 1);
                    if pp.method.is_none() {
                        write!(out, ");\n");
                    }
                    pp.method = pp.symbol;
                    pp.expect = Arguments;
                }
                (FieldOrMethod, _, _, Blank) => continue,
                (FieldOrMethod, _, _, _) => unimplemented!(),

                (Value, _, 0, Delim(';')) => pp.expect = EndOfField,
                (Value, _, _, _) => (),

                (Arguments, _, 0, ClosingGroup) => pp.expect = EndOfMethod,
                (Arguments, _, _, _) => (),

                (SemiColon, _, 0, Delim(';')) => pp.expect = Dot,
                (SemiColon, _, _, _) => (),

                (_, _, _, _) => panic!(),
            }
            match pp.expect {
                Code => write!(out, "{}", &src[&tok]),
                Block => {
                    assert!(pp.object.is_some());
                    pp.ident = Some(tok);
                    write!(out, "sub vcl_init {{\n\tnew {} = {}(",
                        &src[&tok], &src[&pp.object.unwrap()])
                }
                Value => {
                    assert!(pp.field.is_some());
                    match pp.symbol {
                        Some(_) => {
                            assert_eq!(&src[&tok], "=");
                            pp.symbol = None;
                            write!(out, "\t\t{} =", &src[&pp.field.unwrap()])
                        }
                        None => write!(out, "{}", &src[&tok])
                    }
                }
                EndOfField => {
                    pp.expect = Dot;
                    Ok(())
                }
                Arguments => {
                    assert!(pp.ident.is_some());
                    assert!(pp.method.is_some());
                    match pp.symbol {
                        Some(_) => {
                            pp.symbol = None;
                            write!(out, "\t{}.{}(", &src[&pp.ident.unwrap()],
                                &src[&pp.method.unwrap()])
                        }
                        None => write!(out, "{}", &src[&tok])
                    }
                }
                EndOfMethod => {
                    pp.expect = SemiColon;
                    write!(out, ");\n")
                }
                _ => Ok(()),
            };
        }
        if pp.groups != 0 || pp.blocks != 0 {
            unimplemented!();
        }
        Ok(())
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

    match Preprocessor::exec(&buf, BufWriter::new(stdout())) {
        Err(tok) => panic!("{:?}", tok.lexeme),
        _ => ()
    }
}
