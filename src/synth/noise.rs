use std::mem;
use super::*;

use ::rand::{XorShiftRng, Rng, SeedableRng};

#[derive(Debug)]
pub struct Noise {
    pub rng: XorShiftRng,
    pub buf: SampleBuffer,
}

impl Generator for Noise {
    fn eval<'a>(&'a mut self, _params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        for i in 0..self.buf.len() {
            self.buf[i] = self.rng.next_f32();
        }

        &self.buf
    }
    fn buffer<'a>(&'a self) -> &'a SampleBuffer { &self.buf }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct NoiseFactory;

impl GeneratorFactory for NoiseFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        let mut seed: [u32; 4] = ::rand::random();
        for i in seed.iter_mut() {
            *i = ::rand::random()
        }

        Ok(Box::new(Noise {
            rng: XorShiftRng::from_seed(::rand::random()),
            buf: SampleBuffer::new(params.env.default_buffer_size),
        }))
    }
}

pub static Factory: NoiseFactory = NoiseFactory;
