use super::Token;
use std::collections::HashMap;
use std::error::Error;
use std::io::Read;
use std::{fmt, fs, io};
use unicode_xid::UnicodeXID;

pub struct Lexemes {
    radix_point: char,
    exponent_chars: String,
    string_delim: String,
    esc_intro: char,
    esc_hex: char,
    esc_oct: char,
    com_outer: char,
    com_inner: char,
    include_delim: char,
    escapes: HashMap<char, char>,
}

impl Default for Lexemes {
    fn default() -> Lexemes {
        let mut ret = Lexemes {
            radix_point: '.',
            exponent_chars: "eE".to_string(),
            string_delim: "'\"".to_string(),
            esc_intro: '\\',
            esc_hex: 'x',
            esc_oct: 'o',
            com_outer: '/',
            com_inner: '*',
            include_delim: '#',
            escapes: HashMap::new(),
        };

        ret.escapes.insert('n', '\n');
        ret.escapes.insert('t', '\t');
        ret.escapes.insert('r', '\r');
        ret.escapes.insert('"', '"');
        ret.escapes.insert('\'', '\'');

        ret
    }
}

#[derive(Debug)]
pub enum Location {
    InString,
    InStringEscape,
    InInclude,
}

#[derive(Debug)]
pub enum EscapeKind {
    Hexadecimal,
    Octal,
}

#[derive(Debug)]
pub enum NumericKind {
    Integer,
    Float,
}

#[derive(Debug)]
pub enum ErrorKind {
    UnexpectedEOF(Location),
    BadEscapeValue(EscapeKind, String, Option<Box<dyn Error>>),
    BadNumericLiteral(NumericKind, String, Option<Box<dyn Error>>),
    UnknownChar(char),
    IncludeError(io::Error),
    TooManyRecursions(usize),
}

#[derive(Debug)]
pub struct ErrorType {
    pub kind: ErrorKind,
    desc: String,
}

impl ErrorType {
    pub fn new(kind: ErrorKind) -> ErrorType {
        let mut ret = ErrorType {
            kind: kind,
            desc: "".to_string(),
        };

        ret.desc = match ret.kind {
            ErrorKind::UnexpectedEOF(ref loc) => format!(
                "Unexpected EOF {}",
                match *loc {
                    Location::InString => "in string constant",
                    Location::InStringEscape => "in string escape",
                    Location::InInclude => "in include",
                }
            ),
            ErrorKind::BadEscapeValue(ref kind, ref val, ref err) => format!(
                "Bad {} escape {}: {:?}",
                match *kind {
                    EscapeKind::Hexadecimal => "hexadecimal",
                    EscapeKind::Octal => "octal",
                },
                val,
                err
            ),
            ErrorKind::BadNumericLiteral(ref kind, ref val, ref err) => format!(
                "Bad {} literal {}: {:?}",
                match *kind {
                    NumericKind::Integer => "integer",
                    NumericKind::Float => "floating point",
                },
                val,
                err
            ),
            ErrorKind::UnknownChar(c) => format!("Unknown character {}", c),
            ErrorKind::IncludeError(ref e) => format!("Error including file: {:?}", e),
            ErrorKind::TooManyRecursions(n) => format!("Include recursed too many times ({})", n),
        };

        ret
    }

    pub fn with_description(kind: ErrorKind, description: String) -> ErrorType {
        ErrorType {
            kind: kind,
            desc: description,
        }
    }
}

impl From<io::Error> for ErrorType {
    fn from(e: io::Error) -> Self {
        Self::new(ErrorKind::IncludeError(e))
    }
}

impl Error for ErrorType {
    fn description(&self) -> &str {
        &self.desc
    }

    fn cause(&self) -> Option<&dyn Error> {
        match self.kind {
            ErrorKind::BadNumericLiteral(_, _, ref err)
            | ErrorKind::BadEscapeValue(_, _, ref err) => match *err {
                Some(ref err) => Some(&**err),
                None => None,
            },
            _ => None,
        }
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_string())
    }
}

// NB: linear in size of set. This is practically fine for very small sets, but shouldn't be used
// otherwise.
fn char_in(s: &str, c: char) -> bool {
    s.chars().find(|&x| x == c).map_or(false, |_| true)
}

pub struct ResumableChars {
    string: String,
    pos: usize,
}

impl ResumableChars {
    pub fn new(s: String) -> ResumableChars {
        ResumableChars { string: s, pos: 0 }
    }
}

impl Iterator for ResumableChars {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        if self.pos >= self.string.len() {
            None
        } else {
            let mut iter = self.string[self.pos..].char_indices();
            match iter.next() {
                Some((_pos, ch)) => {
                    self.pos += match iter.next() {
                        Some((pos, _)) => pos,
                        None => self.string.len(),
                    };
                    Some(ch)
                }
                None => None,
            }
        }
    }
}

pub struct Tokenizer<T: Iterator<Item = char>> {
    reader: T,
    reader_stack: Vec<ResumableChars>,
    pushback: Option<char>,
    lexemes: Lexemes,
}

impl<T: Iterator<Item = char>> Tokenizer<T> {
    const MAX_INCLUDE_RECURSIONS: usize = 256;

    pub fn new(reader: T) -> Tokenizer<T> {
        Tokenizer {
            reader: reader,
            reader_stack: Vec::new(),
            pushback: None,
            lexemes: Default::default(),
        }
    }

    fn push_back(&mut self, c: char) -> bool {
        match self.pushback {
            None => {
                self.pushback = Some(c);
                true
            }
            Some(_) => false,
        }
    }

    pub fn push_reader(&mut self, rc: ResumableChars) -> Result<(), ErrorType> {
        if self.reader_stack.len() > Self::MAX_INCLUDE_RECURSIONS {
            Err(ErrorType::new(ErrorKind::TooManyRecursions(
                self.reader_stack.len(),
            )))
        } else {
            self.reader_stack.push(rc);
            Ok(())
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.pushback {
            Some(c) => {
                self.pushback = None;
                Some(c)
            }
            None => {
                let mut ret = None;
                let mut produced_idx: usize = 0;

                for (idx, rc) in self.reader_stack.iter_mut().enumerate().rev() {
                    match rc.next() {
                        Some(c) => {
                            ret = Some(c);
                            produced_idx = idx;
                            break;
                        }
                        None => {}
                    }
                }

                match ret {
                    Some(c) => {
                        self.reader_stack.truncate(produced_idx + 1);
                        Some(c)
                    }
                    None => self.reader.next(),
                }
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Token, ErrorType> {
        let mut c = self.next_char();
        if c == None {
            return Ok(Token::EOF);
        }
        let mut cc = c.unwrap();

        /* Whitespace */
        while cc.is_whitespace() {
            c = self.next_char();
            if c == None {
                return Ok(Token::EOF);
            }
            cc = c.unwrap();
        }

        /* Comments */
        if cc == self.lexemes.com_outer {
            let nc = self.next_char();
            if nc == None {
                return Ok(Token::Oper(cc));
            }
            let ncc = nc.unwrap();
            if ncc == self.lexemes.com_inner {
                loop {
                    match self.next_char() {
                        None => return Ok(Token::EOF),
                        Some(x) if x == self.lexemes.com_inner => match self.next_char() {
                            None => return Ok(Token::EOF),
                            Some(x) if x == self.lexemes.com_outer => return self.next_token(),
                            Some(_) => continue,
                        },
                        Some(_) => continue,
                    }
                }
            } else {
                self.push_back(ncc);
                return Ok(Token::Oper(cc));
            }
        }

        /* Inclusion */
        if cc == self.lexemes.include_delim {
            let mut buffer = String::new();

            loop {
                let nc = self.next_char();
                if nc == None {
                    return Err(ErrorType::new(ErrorKind::UnexpectedEOF(
                        Location::InInclude,
                    )));
                }
                let ncc = nc.unwrap();

                if ncc == self.lexemes.include_delim {
                    break;
                } else {
                    buffer.push(ncc);
                }
            }

            let mut f = match fs::File::open(buffer) {
                Err(err) => return Err(ErrorType::new(ErrorKind::IncludeError(err))),
                Ok(f) => f,
            };
            let mut contents = String::new();
            f.read_to_string(&mut contents)?;
            self.push_reader(ResumableChars::new(contents))?;
            return self.next_token();
        }

        /* Strings */
        if char_in(&self.lexemes.string_delim, cc) {
            let mut buffer = String::new();

            loop {
                let nc = self.next_char();
                if nc == None {
                    return Err(ErrorType::new(ErrorKind::UnexpectedEOF(Location::InString)));
                }
                let ncc = nc.unwrap();
                if ncc == self.lexemes.esc_intro {
                    let ec = self.next_char();
                    if ec == None {
                        return Err(ErrorType::new(ErrorKind::UnexpectedEOF(
                            Location::InStringEscape,
                        )));
                    }
                    let ecc = ec.unwrap();

                    if ecc == self.lexemes.esc_hex {
                        let mut value = String::new();
                        loop {
                            let sc = self.next_char();
                            if None == sc {
                                return Err(ErrorType::new(ErrorKind::UnexpectedEOF(
                                    Location::InStringEscape,
                                )));
                            }
                            let scc = sc.unwrap();

                            if scc.is_digit(16) {
                                value.push(scc);
                            } else {
                                self.push_back(scc);
                                break;
                            }
                        }
                        let rc = u32::from_str_radix(&value, 16);
                        if let Err(err) = rc {
                            return Err(ErrorType::new(ErrorKind::BadEscapeValue(
                                EscapeKind::Hexadecimal,
                                value,
                                Some(Box::new(err)),
                            )));
                        }
                        let rc = ::std::char::from_u32(rc.unwrap());
                        match rc {
                            Some(rcc) => buffer.push(rcc),
                            None => {
                                return Err(ErrorType::new(ErrorKind::BadEscapeValue(
                                    EscapeKind::Hexadecimal,
                                    value,
                                    None,
                                )))
                            }
                        }
                        continue;
                    }

                    if ecc == self.lexemes.esc_oct {
                        let mut value = String::new();
                        loop {
                            let sc = self.next_char();
                            if None == sc {
                                return Err(ErrorType::new(ErrorKind::UnexpectedEOF(
                                    Location::InStringEscape,
                                )));
                            }
                            let scc = sc.unwrap();

                            if scc.is_digit(8) {
                                value.push(scc);
                            } else {
                                self.push_back(scc);
                                break;
                            }
                        }
                        let rc = u32::from_str_radix(&value, 8);
                        if let Err(err) = rc {
                            return Err(ErrorType::new(ErrorKind::BadEscapeValue(
                                EscapeKind::Octal,
                                value,
                                Some(Box::new(err)),
                            )));
                        }
                        let rc = ::std::char::from_u32(rc.unwrap());
                        match rc {
                            Some(rcc) => buffer.push(rcc),
                            None => {
                                return Err(ErrorType::new(ErrorKind::BadEscapeValue(
                                    EscapeKind::Octal,
                                    value,
                                    None,
                                )))
                            }
                        }
                        continue;
                    }

                    buffer.push(*self.lexemes.escapes.get(&ecc).unwrap_or(&ecc));
                    continue;
                }

                if ncc == cc {
                    return Ok(Token::String(buffer));
                }

                buffer.push(ncc);
            }
        }

        /* Numeric constants */
        if cc.is_digit(10) {
            let mut radix = 10;
            let mut buffer = String::new();
            let mut floating = false;

            if cc == '0' {
                let nc = self.next_char();
                if nc == None {
                    return Ok(Token::Integer(0));
                }
                let ncc = nc.unwrap();

                if ncc == self.lexemes.esc_hex {
                    radix = 16;
                } else if ncc == self.lexemes.esc_oct {
                    radix = 8;
                } else if ncc == self.lexemes.radix_point {
                    floating = true;
                    buffer.push(cc);
                    buffer.push(ncc);
                } else if ncc.is_digit(10) {
                    buffer.push(cc);
                    buffer.push(ncc);
                } else {
                    self.push_back(ncc);
                    return Ok(Token::Integer(0));
                }
            } else {
                buffer.push(cc);
            }

            loop {
                let dc = self.next_char();
                if dc == None {
                    break;
                }
                let dcc = dc.unwrap();

                if dcc.is_digit(radix) {
                    buffer.push(dcc);
                } else if dcc == self.lexemes.radix_point {
                    floating = true;
                    buffer.push(dcc);
                } else if floating && char_in(&self.lexemes.exponent_chars, dcc) {
                    buffer.push(dcc);
                } else {
                    self.push_back(dcc);
                    break;
                }
            }

            return if floating {
                match buffer.parse::<f32>() {
                    Ok(v) => Ok(Token::Float(v)),
                    Err(err) => Err(ErrorType::new(ErrorKind::BadNumericLiteral(
                        NumericKind::Float,
                        buffer,
                        Some(Box::new(err)),
                    ))),
                }
            } else {
                match buffer.parse::<isize>() {
                    Ok(v) => Ok(Token::Integer(v)),
                    Err(err) => Err(ErrorType::new(ErrorKind::BadNumericLiteral(
                        NumericKind::Integer,
                        buffer,
                        Some(Box::new(err)),
                    ))),
                }
            };
        }

        /* Identifiers */
        if UnicodeXID::is_xid_start(cc) {
            let mut buffer = String::new();
            buffer.push(cc);

            loop {
                let nc = self.next_char();
                if nc == None {
                    return Ok(Token::Ident(buffer));
                }
                let ncc = nc.unwrap();

                if UnicodeXID::is_xid_continue(ncc) {
                    buffer.push(ncc);
                } else {
                    self.push_back(ncc);
                    break;
                }
            }

            return Ok(Token::Ident(buffer));
        }

        /* Everything else */
        Ok(Token::Oper(cc))
    }
}

impl<T: Iterator<Item = char>> Iterator for Tokenizer<T> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match self.next_token() {
            Err(_) | Ok(Token::EOF) => None,
            Ok(t) => Some(t),
        }
    }
}
