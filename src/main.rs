use std::io;
use std::io::*;

extern crate synfone;
use synfone::synth::*;
use synfone::lang::*;

const FRAMES: usize = 44100 * 2;

const GEN: &'static str = "add(mul(sine(param('freq', 440)), 0.5), mul(sine(param('freq2', 660)), 0.5))";

fn main() {
    let mut params = Parameters::default();
    
    let mut gen = Parser::new(Tokenizer::new(GEN.chars())).expect("Failed to get first token").parse().expect("Failed to compile generator");

    let mut counter = 0;
    let mut out = io::stdout();
    let mut outbuf: Vec<u8> = Vec::new();
    
    params.vars.insert("freq".to_string(), 440.0);
    params.vars.insert("freq2".to_string(), 660.0);

    while counter < FRAMES {
        *params.vars.get_mut("freq").unwrap() = 440.0 + 440.0 * ((counter as f32) / (FRAMES as f32));
        *params.vars.get_mut("freq2").unwrap() = 660.0 + 220.0 * ((counter as f32) / (FRAMES as f32));
        let buf = gen.eval(&params);
        let curlen = outbuf.len();
        outbuf.reserve_exact(buf.size() - curlen);
        unsafe { outbuf.set_len(buf.size()); }
        buf.bytes(&mut outbuf);
        out.write_all(&outbuf).expect("failed to write to stdout");
        counter += buf.len();
    }
}
