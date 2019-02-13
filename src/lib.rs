extern crate byteorder;
extern crate rand;
extern crate unicode_xid;

pub mod types;
pub use types::*;

pub mod synth;
pub mod proto;
pub mod lang;
pub mod client;
pub mod monitor;

#[cfg(feature = "graphics")]
pub mod graphics;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
