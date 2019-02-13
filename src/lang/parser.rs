use std::{mem, fmt};
use std::error::Error;
use std::collections::HashMap;
use super::*;
use synth::*;

/*
macro_rules! dprintln {
    ( $( $x:expr ),* ) => { eprintln!( $( $x ),* ) }
}
*/

macro_rules! dprintln {
    ( $( $x:expr ),* ) => { () }
}

#[derive(Debug)]
pub enum ErrorKind {
    Unexpected(TokType, TokType),
    Unparseable(TokType, String),
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

        ret.desc = match ret.kind {
            ErrorKind::Unexpected(found, expected) => format!("Found {:?}, expected {:?}", found, expected),
            ErrorKind::Unparseable(found, ref term) => format!("Cannot consume {:?} token in {}", found, term),
            ErrorKind::ExpectedOp(c, found) => format!("Expected {:?}, found {:?}", c, found),
            ErrorKind::UnknownGen(ref s) => format!("Unknown generator name {}", s),
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
    fn description(&self) -> &str {
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
    env: Environment,
    token: Token,
    pushback: Option<Token>,
    factories: HashMap<String, &'static GeneratorFactory>,
}

impl<T: Iterator<Item=char>> Parser<T> {
    pub fn new(mut tzr: Tokenizer<T>, env: Environment) -> Result<Parser<T>, Box<Error>> {
        let token = tzr.next_token()?;
        Ok(Parser {
            tzr: tzr,
            env: env,
            token: token,
            pushback: None,
            factories: all_factories(),
        })
    }

    pub fn push_back(&mut self, tok: Token) {
        match self.pushback {
            None => {
                self.pushback = Some(tok);
            },
            Some(_) => panic!("too many pushbacks on Parser"),
        }
    }

    pub fn cur_token(&self) -> &Token {
        match self.pushback {
            Some(ref tok) => tok,
            None => &self.token,
        }
    }

    pub fn expect(&mut self, ty: TokType) -> Result<Token, Box<Error>> {
        if ty != self.cur_token().to_type() {
            Err(ErrorType::new(ErrorKind::Unexpected(self.token.to_type(), ty)).into())
        } else {
            Ok(match self.pushback {
                Some(_) => mem::replace(&mut self.pushback, None).unwrap(),
                None => mem::replace(&mut self.token, self.tzr.next_token()?),
            })
        }
    }

    pub fn expect_ident(&mut self) -> Result<String, Box<Error>> {
        match self.expect(TokType::Ident)? {
            Token::Ident(s) => Ok(s),
            _ => unreachable!(),
        }
    }

    pub fn expect_op(&mut self, oper: char) -> Result<(), Box<Error>> {
        dprintln!("expect_op: {:?} ({})", self.cur_token(), oper);
        match *self.cur_token() {
            Token::Oper(c) if c == oper => { self.expect(TokType::Oper)?; Ok(()) },
            _ => Err(ErrorType::new(ErrorKind::ExpectedOp(oper, self.cur_token().to_type())).into()),
        }
    }

    pub fn peek_op(&self, oper: char) -> bool {
        dprintln!("peek_op: {:?} ({})", self.cur_token(), oper);
        match *self.cur_token() {
            Token::Oper(c) if c == oper => true,
            _ => false
        }
    }

    pub fn parse_gen_vec(&mut self) -> Result<Vec<GenBox>, Box<Error>> {
        let mut ret: Vec<GenBox> = Vec::new();
        self.expect_op('[')?;


        loop {
            if self.expect_op(']').is_ok() {
                break;
            }

            ret.push(self.parse_gen_rel()?);

            if self.expect_op(',').is_err() {
                self.expect_op(']')?;
                break;
            }
        }

        Ok(ret)
    }

    pub fn parse_gen_rel(&mut self) -> Result<GenBox, Box<Error>> {
        let left = self.parse_gen_terms()?;

        match *self.cur_token() {
            Token::Oper(c) => {
                if c == '>' || c == '!' || c == '<' || c == '=' { // TODO: Conflict with param name
                    self.expect(TokType::Oper)?;
                    let relop = match (c, self.cur_token()) {
                        ('<', &Token::Oper('=')) => { self.expect(TokType::Oper)?; RelOp::LessEqual },
                        ('=', &Token::Oper('=')) => { self.expect(TokType::Oper)?; RelOp::Equal },
                        ('>', &Token::Oper('=')) => { self.expect(TokType::Oper)?; RelOp::Greater },
                        ('!', &Token::Oper('=')) => { self.expect(TokType::Oper)?; RelOp::NotEqual },
                        ('<', _) => RelOp::Less,
                        ('>', _) => RelOp::Greater,
                        _ => return Err(ErrorType::new(ErrorKind::Unparseable(TokType::Oper, "rel expr".to_string())).into()),
                    };
                    let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                    params.vars.insert("0".to_string(), ParamValue::Generator(left));
                    params.vars.insert("1".to_string(), ParamValue::String(relop.to_param_string().to_string()));
                    params.vars.insert("2".to_string(), ParamValue::Generator(self.parse_gen_rel()?));
                    let factory = self.factories.get("rel").ok_or(ErrorType::new(ErrorKind::UnknownGen("rel".to_string())))?;
                    factory.new(&mut params).map_err(Into::into)
                } else {
                    Ok(left)
                }
            },
            _ => Ok(left),
        }
    }

    pub fn parse_gen_terms(&mut self) -> Result<GenBox, Box<Error>> {
        let mut gens: Vec<GenBox> = Vec::new();
        gens.push(self.parse_gen_factors()?);

        loop {
            match *self.cur_token() {
                Token::Oper('+') => {
                    self.expect_op('+')?;
                    gens.push(self.parse_gen_factors()?);
                },
                Token::Oper('-') => {
                    self.expect_op('-')?;
                    let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                    params.vars.insert("0".to_string(), ParamValue::Generator(self.parse_gen_factors()?));
                    let factory = self.factories.get("negate").ok_or(ErrorType::new(ErrorKind::UnknownGen("negate".to_string())))?;
                    gens.push(factory.new(&mut params).map_err(GenFactoryErrorType::from)?);
                },
                _ => break,
            }
        }

        if gens.len() == 1 {
            return Ok(gens.pop().unwrap());
        }

        let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
        for (idx, gen) in gens.into_iter().enumerate() {
            params.vars.insert(idx.to_string(), ParamValue::Generator(gen));
        }
        let factory = self.factories.get("add").ok_or(ErrorType::new(ErrorKind::UnknownGen("add".to_string())))?;
        factory.new(&mut params).map_err(Into::into)
    }

    pub fn parse_gen_factors(&mut self) -> Result<GenBox, Box<Error>> {
        let mut gens: Vec<GenBox> = Vec::new();
        gens.push(self.parse_gen()?);

        loop {
            match *self.cur_token() {
                Token::Oper('*') => {
                    self.expect_op('*')?;
                    gens.push(self.parse_gen()?);
                },
                Token::Oper('/') => {
                    self.expect_op('/')?;
                    let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                    params.vars.insert("0".to_string(), ParamValue::Generator(self.parse_gen()?));
                    let factory = self.factories.get("reciprocate").ok_or(ErrorType::new(ErrorKind::UnknownGen("reciprocate".to_string())))?;
                    gens.push(factory.new(&mut params).map_err(GenFactoryErrorType::from)?);
                },
                _ => break,
            }
        }

        if gens.len() == 1 {
            return Ok(gens.pop().unwrap());
        }

        let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
        for (idx, gen) in gens.into_iter().enumerate() {
            params.vars.insert(idx.to_string(), ParamValue::Generator(gen));
        }
        let factory = self.factories.get("mul").ok_or(ErrorType::new(ErrorKind::UnknownGen("mul".to_string())))?;
        factory.new(&mut params).map_err(Into::into)
    }

    pub fn parse_gen(&mut self) -> Result<GenBox, Box<Error>> {
        match *self.cur_token() {
            Token::Integer(v) => {
                self.expect(TokType::Integer)?;
                let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                params.vars.insert("0".to_string(), ParamValue::String("_".to_string()));
                params.vars.insert("1".to_string(), ParamValue::Integer(v));
                let factory = self.factories.get("param").ok_or(ErrorType::new(ErrorKind::UnknownGen("param".to_string())))?;
                factory.new(&mut params).map_err(Into::into)
            },
            Token::Float(v) => {
                self.expect(TokType::Float)?;
                let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                params.vars.insert("0".to_string(), ParamValue::String("_".to_string()));
                params.vars.insert("1".to_string(), ParamValue::Float(v));
                let factory = self.factories.get("param").ok_or(ErrorType::new(ErrorKind::UnknownGen("param".to_string())))?;
                factory.new(&mut params).map_err(Into::into)
            },
            Token::Ident(_) => {
                let name = self.expect_ident()?;
                if self.peek_op('(') {
                    let mut params = self.parse_factory_params()?;
                    let factory = match self.factories.get(&name) {
                        Some(fac) => fac,
                        None => return Err(ErrorType::new(ErrorKind::UnknownGen(name)).into()),
                    };
                    factory.new(&mut params).map_err(Into::into)
                } else {
                    let mut params = FactoryParameters { env: self.env.clone(), ..Default::default() };
                    params.vars.insert("0".to_string(), ParamValue::String(name));
                    let factory = self.factories.get("param").ok_or(ErrorType::new(ErrorKind::UnknownGen("param".to_string())))?;
                    factory.new(&mut params).map_err(Into::into)
                }
            },
            Token::Oper('(') => {
                dprintln!("consuming paren in parse_gen");
                self.expect(TokType::Oper)?;
                let ret = self.parse_gen_rel()?;
                dprintln!("parenthesized generator is concluding");
                self.expect_op(')')?;
                Ok(ret)
            },
            _ => Err(ErrorType::new(ErrorKind::Unparseable(self.cur_token().to_type(), "gen".to_string())).into()),
        }
    }

    pub fn parse_factory_params(&mut self) -> Result<FactoryParameters, Box<Error>> {
        dprintln!("consuming paren in factory_params");
        self.expect_op('(')?;

        let mut params: FactoryParameters = FactoryParameters { env: self.env.clone(), ..Default::default() };
        let mut ctr = 0;
        loop {
            if self.expect_op(')').is_ok() {
                break;
            }
            let (nm, vl, new_ctr) = self.parse_param(ctr)?;
            params.vars.insert(nm, vl);
            ctr = new_ctr;

            dprintln!("before factory_params comma, tok is {:?}", self.cur_token());
            if self.expect_op(',').map_err(|e| dprintln!("factory_params consume comma failed: {:?}", e)).is_err() {
                dprintln!("factory_params is concluding");
                self.expect_op(')')?;
                break;
            }
        }

        Ok(params)
    }

    pub fn parse_param(&mut self, pos: usize) -> Result<(String, ParamValue, usize), Box<Error>> {
        let mut ctr = pos;
        let name = match self.expect_ident() {
            Ok(nm) => {
                if self.expect_op('=').is_ok() {
                    nm
                } else {
                    self.push_back(Token::Ident(nm));
                    ctr += 1;
                    (ctr - 1).to_string()
                }
            },
            Err(_) => {
                ctr += 1;
                (ctr - 1).to_string()
            },
        };

        dprintln!("about to consume param value, token is {:?}", self.cur_token());

        match self.cur_token().clone() {  // FIXME: Does this really need to be cloned?
            Token::String(ref v) => { self.expect(TokType::String)?; Ok((name, ParamValue::String(v.clone()), ctr)) },
            Token::Integer(_) | Token::Float(_) | Token::Ident(_) | Token::Oper('(') => Ok((name, ParamValue::Generator(self.parse_gen_rel()?), ctr)),
            _ => Err(ErrorType::new(ErrorKind::Unparseable(self.cur_token().to_type(), "param value".to_string())).into()),
        }
    }
}
