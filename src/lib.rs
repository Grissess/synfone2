#![feature(associated_consts)]

extern crate byteorder;
extern crate rand;

pub mod types;
pub use types::*;

pub mod synth;
pub mod proto;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
