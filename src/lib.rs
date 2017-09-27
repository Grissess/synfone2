#![feature(unicode)]

extern crate byteorder;
extern crate rand;
extern crate unicode_xid;

pub mod types;
pub use types::*;

pub mod synth;
pub mod proto;
pub mod lang;
pub mod client;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
