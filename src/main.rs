use std::io;
use std::io::*;

extern crate synfone;
use synfone::synth::*;

const FRAMES: usize = 44100 * 2;

fn main() {
    let mut params = Parameters::default();
    
    let mut freq: GenBox = Box::new(Param { name: "freq".to_string(), default: 440.0, buf: SampleBuffer::new(1) });
    let mut sg: GenBox = Box::new(Sine { freq: freq, phase: 0.0, buf: SampleBuffer::new(params.env.default_buffer_size) });

    let mut counter = 0;
    let mut out = io::stdout();
    
    params.vars.insert("freq".to_string(), 440.0);

    while counter < FRAMES {
        *params.vars.get_mut("freq").unwrap() = 440.0 + 440.0 * ((counter as f32) / (FRAMES as f32));
        let buf = sg.eval(&params);
        out.write_all(buf.bytes());
        counter += buf.len();
    }
}
