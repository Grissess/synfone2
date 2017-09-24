pub mod tokenizer;
pub use self::tokenizer::Tokenizer;
pub mod parser;
pub use self::parser::Parser;

// NB: No Eq due to embedded f32
#[derive(Debug,PartialEq,Clone)]
pub enum Token {
    Ident(String),
    Integer(isize),
    Float(f32),
    Oper(char),
    String(String),
    EOF,
}

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum TokType {
    Ident,
    Integer,
    Float,
    Oper,
    String,
    EOF,
}

impl Token {
    pub fn to_type(&self) -> TokType {
        match *self {
            Token::Ident(_) => TokType::Ident,
            Token::Integer(_) => TokType::Integer,
            Token::Float(_) => TokType::Float,
            Token::Oper(_) => TokType::Oper,
            Token::String(_) => TokType::String,
            Token::EOF => TokType::EOF,
        }
    }
}

impl<'a> From<&'a Token> for TokType {
    fn from(tok: &'a Token) -> TokType {
        tok.to_type()
    }
}
