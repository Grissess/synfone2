use std::io;
use std::io::*;

extern crate rand;
use rand::{Rng, SeedableRng};
extern crate synfone;
use synfone::synth::*;

const FRAMES: usize = 44100 * 2;

fn main() {
    let mut params = Parameters::default();
    
    //let mut freq: GenBox = Box::new(Param { name: "freq".to_string(), default: 440.0, buf: SampleBuffer::new(1) });
    //let mut sg: GenBox = Box::new(Saw { freq: freq, phase: 0.0, buf: SampleBuffer::new(params.env.default_buffer_size) });
    let mut osrng = rand::os::OsRng::new().expect("Couldn't initialize OS RNG");
    let mut seed: [u32; 4] = Default::default();
    for i in seed.iter_mut() {
        *i = osrng.next_u32();
    }
    let mut sg: GenBox = Box::new(Noise { rng: rand::XorShiftRng::from_seed(seed), buf: SampleBuffer::new(params.env.default_buffer_size) });

    let mut freq2: GenBox = Box::new(Param { name: "freq2".to_string(), default: 660.0, buf: SampleBuffer::new(1) });
    let mut sg2: GenBox = Box::new(Sine { freq: freq2, phase: 0.0, buf: SampleBuffer::new(params.env.default_buffer_size) });

    let mut half1: GenBox = Box::new(Param { name: "half".to_string(), default: 1.0, buf: SampleBuffer::new(1) });
    let mut half2: GenBox = Box::new(Param { name: "half".to_string(), default: 0.0, buf: SampleBuffer::new(1) });
    let mut sc1: GenBox = Box::new(Mul { factors: vec![sg, half1], buf: SampleBuffer::new(params.env.default_buffer_size) });
    let mut sc2: GenBox = Box::new(Mul { factors: vec![sg2, half2], buf: SampleBuffer::new(params.env.default_buffer_size) });
    let mut gen: GenBox = Box::new(Add { terms: vec![sc1, sc2], buf: SampleBuffer::new(params.env.default_buffer_size) });

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
        out.write_all(&outbuf);
        counter += buf.len();
    }
}
