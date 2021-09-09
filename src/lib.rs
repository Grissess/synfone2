extern crate byteorder;
extern crate rand;
extern crate unicode_xid;
extern crate xml;
#[macro_use]
extern crate failure;

pub mod types;
pub use types::*;

pub mod client;
pub mod lang;
pub mod monitor;
pub mod proto;
pub mod seq;
pub mod synth;

#[cfg(feature = "graphics")]
pub mod graphics;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
