extern crate byteorder;
extern crate rand;
extern crate unicode_xid;
extern crate quick_xml;
extern crate midly;

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
