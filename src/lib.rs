#![feature(unicode)]

extern crate byteorder;
extern crate rand;
extern crate unicode_xid;
extern crate xml;

pub mod types;
pub use types::*;

pub mod synth;
pub mod proto;
pub mod lang;
pub mod client;
pub mod monitor;
pub mod seq;

#[cfg(feature = "graphics")]
pub mod graphics;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
