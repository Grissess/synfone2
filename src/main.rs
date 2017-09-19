use std::io;
use std::io::*;

extern crate synfone;
use synfone::synth::*;

const FRAMES: usize = 44100 * 2;

fn main() {
    let mut params = Default::default();
    
    let mut freq: GenBox = Box::new(Param { name: "freq".to_string(), default: 440.0, buf: SampleBuffer::new(1) });
    let mut sg: GenBox = Box::new(Sine { freq: freq, phase: 0.0, buf: SampleBuffer::new(params.env.default_buffer_size) });

    let mut counter = 0;
    let mut out = io::stderr();

    while counter < FRAMES {
        let buf = sg.eval(&params);
        out.write_all(buf.bytes());
        counter += buf.len();
    }
}
