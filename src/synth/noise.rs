use std::mem;
use super::*;

use ::rand::{XorShiftRng, Rng};

#[derive(Debug)]
pub struct Noise {
    pub rng: XorShiftRng,
    pub buf: SampleBuffer,
}

impl Generator for Noise {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        for i in 0..self.buf.len() {
            self.buf[i] = self.rng.next_f32();
        }

        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

