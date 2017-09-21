use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use super::*;

pub struct Lexemes {
    radix_point: char,
    exponent_chars: String,
    string_delim: String,
    esc_intro: char,
    esc_hex: char,
    esc_oct: char,
    com_outer: char,
    com_inner: char,
    escapes: HashMap<char, char>
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
    BadEscapeValue(EscapeKind, String, Option<Box<Error>>),
    BadNumericLiteral(NumericKind, String, Option<Box<Error>>),
    UnknownChar(char),
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

        ret.desc = match &ret.kind {
            &ErrorKind::UnexpectedEOF(ref loc) => format!("Unexpected EOF {}", match loc {
                &Location::InString => "in string constant",
                &Location::InStringEscape => "in string escape",
            }),
            &ErrorKind::BadEscapeValue(ref kind, ref val, ref err) => format!("Bad {} escape {}: {:?}", match kind {
                &EscapeKind::Hexadecimal => "hexadecimal",
                &EscapeKind::Octal => "octal",
            }, val, err),
            &ErrorKind::BadNumericLiteral(ref kind, ref val, ref err) => format!("Bad {} literal {}: {:?}", match kind {
                &NumericKind::Integer => "integer",
                &NumericKind::Float => "floating point",
            }, val, err),
            &ErrorKind::UnknownChar(c) => format!("Unknown character {}", c),
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

impl Error for ErrorType {
    fn description<'a>(&'a self) -> &'a str {
        &self.desc
    }

    fn cause(&self) -> Option<&Error> {
        match &self.kind {
            &ErrorKind::UnexpectedEOF(_) => None,
            &ErrorKind::BadEscapeValue(_, _, ref err) => match err {
                &Some(ref err) => Some(&**err),
                &None => None,
            },
            &ErrorKind::BadNumericLiteral(_, _, ref err) => match err {
                &Some(ref err) => Some(&**err),
                &None => None,
            },
            &ErrorKind::UnknownChar(_) => None,
        }
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.description())
    }
}

// NB: linear in size of set. This is practically fine for very small sets, but shouldn't be used
// otherwise.
fn char_in(s: &str, c: char) -> bool {
    s.chars().find(|&x| x == c).map_or(false, |_| true)
}

pub struct Tokenizer<T: Iterator<Item=char>> {
    reader: T,
    pushback: Option<char>,
    lexemes: Lexemes,
}

impl<T: Iterator<Item=char>> Tokenizer<T> {
    pub fn new(reader: T) -> Tokenizer<T> {
        Tokenizer {
            reader: reader,
            pushback: None,
            lexemes: Default::default(),
        }
    }

    fn push_back(&mut self, c: char) -> bool {
        match self.pushback {
            None => {
                self.pushback = Some(c);
                true
            },
            Some(_) => false,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.pushback {
            Some(c) => {
                self.pushback = None;
                Some(c)
            },
            None => self.reader.next(),
        }
    }

    fn next_token(&mut self) -> Result<Token, ErrorType> {
        let mut c = self.next_char();
        if c == None {
            return Ok(Token::EOF);
        }
        let mut cc = c.unwrap();

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
                        return Err(ErrorType::new(ErrorKind::UnexpectedEOF(Location::InStringEscape)));
                    }
                    let ecc = ec.unwrap();

                    if ecc == self.lexemes.esc_hex {
                        let mut value = String::new();
                        loop {
                            let sc = self.next_char();
                            if None == sc {
                                return Err(ErrorType::new(ErrorKind::UnexpectedEOF(Location::InStringEscape)));
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
                            return Err(ErrorType::new(ErrorKind::BadEscapeValue(EscapeKind::Hexadecimal, value, Some(Box::new(err)))));
                        }
                        let rc = ::std::char::from_u32(rc.unwrap());
                        match rc {
                            Some(rcc) => buffer.push(rcc),
                            None => return Err(ErrorType::new(ErrorKind::BadEscapeValue(EscapeKind::Hexadecimal, value, None))),
                        }
                        continue;
                    }

                    if ecc == self.lexemes.esc_oct {
                        let mut value = String::new();
                        loop {
                            let sc = self.next_char();
                            if None == sc {
                                return Err(ErrorType::new(ErrorKind::UnexpectedEOF(Location::InStringEscape)));
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
                            return Err(ErrorType::new(ErrorKind::BadEscapeValue(EscapeKind::Octal, value, Some(Box::new(err)))));
                        }
                        let rc = ::std::char::from_u32(rc.unwrap());
                        match rc {
                            Some(rcc) => buffer.push(rcc),
                            None => return Err(ErrorType::new(ErrorKind::BadEscapeValue(EscapeKind::Octal, value, None))),
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
                } else {
                    buffer.push(cc);
                    buffer.push(ncc);
                }
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
                    Err(err) => Err(ErrorType::new(ErrorKind::BadNumericLiteral(NumericKind::Float, buffer, Some(Box::new(err))))),
                }
            } else {
                match buffer.parse::<isize>() {
                    Ok(v) => Ok(Token::Integer(v)),
                    Err(err) => Err(ErrorType::new(ErrorKind::BadNumericLiteral(NumericKind::Integer, buffer, Some(Box::new(err))))),
                }
            };
        }

        /* Identifiers */
        if cc.is_xid_start() {
            let mut buffer = String::new();
            buffer.push(cc);

            loop {
                let nc = self.next_char();
                if nc == None {
                    return Ok(Token::Ident(buffer));
                }
                let ncc = nc.unwrap();

                if ncc.is_xid_continue() {
                    buffer.push(ncc);
                } else {
                    self.push_back(ncc);
                    break;
                }
            }

            return Ok(Token::Ident(buffer));
        }

        /* Everything else */
        return Ok(Token::Oper(cc));
    }
}

impl<T: Iterator<Item=char>> Iterator for Tokenizer<T> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match self.next_token() {
            Err(_) => None,
            Ok(Token::EOF) => None,
            Ok(t) => Some(t),
        }
    }
}
