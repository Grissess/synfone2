use std::{mem, fmt};
use std::error::Error;
use std::collections::HashMap;
use super::*;
use synth::*;

#[derive(Debug)]
pub enum ErrorKind {
    Unexpected(TokType, TokType),
    ExpectedOp(char, TokType),
    UnknownGen(String),
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
            &ErrorKind::Unexpected(found, expected) => format!("Found {:?}, expected {:?}", found, expected),
            &ErrorKind::ExpectedOp(c, found) => format!("Expected {:?}, found {:?}", c, found),
            &ErrorKind::UnknownGen(ref s) => format!("Unknown generator name {}", s),
        };

        ret
    }

    pub fn with_description(kind: ErrorKind, desc: String) -> ErrorType {
        ErrorType {
            kind: kind,
            desc: desc,
        }
    }
}

impl Error for ErrorType {
    fn description<'a>(&'a self) -> &'a str {
        &self.desc
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.description())
    }
}

pub struct Parser<T: Iterator<Item=char>> {
    tzr: Tokenizer<T>,
    token: Token,
    pushback: Option<Token>,
    factories: HashMap<String, &'static GeneratorFactory>,
}

impl<T: Iterator<Item=char>> Parser<T> {
    pub fn new(mut tzr: Tokenizer<T>) -> Result<Parser<T>, Box<Error>> {
        let token = tzr.next_token()?;
        Ok(Parser {
            tzr: tzr,
            token: token,
            pushback: None,
            factories: all_factories(),
        })
    }

    pub fn push_back(&mut self, tok: Token) {
        match self.pushback {
            None => {
                self.pushback = Some(mem::replace(&mut self.token, tok));
            },
            Some(_) => panic!("too many pushbacks on Parser"),
        }
    }

    pub fn expect(&mut self, ty: TokType) -> Result<Token, Box<Error>> {
        if ty == self.token.to_type() {
            Ok(mem::replace(&mut self.token, match self.pushback {
                Some(_) => mem::replace(&mut self.pushback, None).unwrap(),
                None => self.tzr.next_token()?,
            }))
        } else {
            Err(ErrorType::new(ErrorKind::Unexpected(self.token.to_type(), ty)).into())
        }
    }

    pub fn expect_ident(&mut self) -> Result<String, Box<Error>> {
        match self.expect(TokType::Ident)? {
            Token::Ident(s) => Ok(s),
            _ => unreachable!(),
        }
    }

    pub fn expect_op(&mut self, oper: char) -> Result<(), Box<Error>> {
        match self.token {
            Token::Oper(c) if c == oper => { self.expect(TokType::Oper)?; Ok(()) },
            _ => Err(ErrorType::new(ErrorKind::ExpectedOp(oper, self.token.to_type())).into()),
        }
    }

    pub fn parse(&mut self) -> Result<GenBox, Box<Error>> {
        self.parse_gen()
    }

    pub fn parse_gen(&mut self) -> Result<GenBox, Box<Error>> {
        let name = self.expect_ident()?;

        self.expect_op('(')?;
        let mut params: FactoryParameters = Default::default();
        let mut ctr = 0;
        loop {
            if self.expect_op(')').is_ok() {
                break;
            }
            let (nm, vl, new_ctr) = self.parse_param(ctr)?;
            params.vars.insert(nm, vl);
            ctr = new_ctr;

            if self.expect_op(',').is_err() {
                eprintln!("No comma: {:?}", self.token);
                self.expect_op(')')?;
                break;
            }
        }

        let factory = match self.factories.get(&name) {
            Some(fac) => fac,
            None => return Err(ErrorType::new(ErrorKind::UnknownGen(name)).into()),
        };
        factory.new(&mut params).map_err(Into::into)
    }

    pub fn parse_param(&mut self, pos: usize) -> Result<(String, ParamValue, usize), Box<Error>> {
        let mut ctr = pos;
        let name = match self.expect_ident() {
            Ok(nm) => {
                if self.expect_op('=').is_ok() {
                    nm
                } else {
                    match &self.token {
                        &Token::Oper(c) if c == '(' => {
                            self.push_back(Token::Ident(nm));
                            ctr += 1;
                            (ctr - 1).to_string()
                        },
                        _ => return Err(ErrorType::new(ErrorKind::Unexpected(self.token.to_type(), TokType::Ident)).into()),
                    }
                }
            },
            Err(_) => {
                ctr += 1;
                (ctr - 1).to_string()
            },
        };

        let ret = match self.token {
            Token::Integer(v) => Ok((name, ParamValue::Integer(v), ctr)),
            Token::Float(v) => Ok((name, ParamValue::Float(v), ctr)),
            Token::String(ref v) => Ok((name, ParamValue::String(v.clone()), ctr)),
            Token::Ident(_) => return Ok((name, ParamValue::Generator(self.parse_gen()?), ctr)),
            _ => return Err(ErrorType::new(ErrorKind::Unexpected(self.token.to_type(), TokType::Ident)).into()),
        };

        let tp = self.token.to_type();
        self.expect(tp);
        ret
    }
}
