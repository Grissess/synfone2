use std::io;
use std::io::*;
use std::net::*;

extern crate synfone;
use synfone::synth::*;
use synfone::lang::*;
use synfone::client::*;

const GEN: &'static str = "mul(sine(param('v_freq', 500)), ifelse(rel(param('v_frame'), '<', param('v_deadline')), param('v_amp'), 0.0))";

fn main() {
    let env = Environment::default();

    let gen = Parser::new(Tokenizer::new(GEN.chars())).expect("Failed to get first token").parse().expect("Failed to compile generator");
    let sock = UdpSocket::bind("0.0.0.0:13676").expect("Failed to bind socket");

    let mut client = Client::new(sock, vec![gen], env).expect("Failed to create client");
    let mut buf: Vec<u8> = Vec::new();
    let mut out = io::stdout();

    eprintln!("Starting.");

    while client.pump(&mut buf) {
        out.write_all(&buf).expect("Failed to write samples");
    }

    eprintln!("Exiting.");
}
