use std::f32::consts::PI;
use super::*;

const TAU: f32 = 2f32 * PI;

pub struct Sine {
    freq: GenBox,
    phase: f32,
    buf: SampleBuffer,
}

impl Generator for Sine {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Control;

        let pvel = TAU * self.freq.eval(params).first() / params.env.sample_rate;
        for i in 0..self.buf.len() {
            self.buf[i] = (self.phase + pvel * (i as f32)).sin()
        }

        self.phase = (self.phase + pvel * (self.buf.len() as f32)) % TAU;
        &self.buf
    }
}
