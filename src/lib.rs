#![feature(associated_consts)]
#![feature(unicode)]

extern crate byteorder;
extern crate rand;

pub mod types;
pub use types::*;

pub mod synth;
pub mod proto;
pub mod lang;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
