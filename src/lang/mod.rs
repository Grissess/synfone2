pub mod tokenizer;
pub use self::tokenizer::Tokenizer;

pub enum Token {
    Ident(String),
    Integer(isize),
    Float(f32),
    Oper(char),
    String(String),
    EOF,
}

