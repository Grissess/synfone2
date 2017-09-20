use std::mem;
use super::*;

#[derive(Debug)]
pub struct Triangle {
    pub freq: GenBox,
    pub phase: f32,
    pub buf: SampleBuffer,
}

impl Generator for Triangle {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        let pvel = self.freq.eval(params).first() / params.env.sample_rate;
        for i in 0..self.buf.len() {
            let ph = (self.phase + pvel * (i as f32)) % 1.0;
            self.buf[i] = if ph < 0.25 {
                4.0 * ph
            } else if ph > 0.75 {
                4.0 * ph - 4.0
            } else {
                -4.0 * ph + 2.0
            };
        }

        self.phase = (self.phase + pvel * (self.buf.len() as f32)) % 1.0;
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
