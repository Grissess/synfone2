use std::mem;
use super::*;

use ::rand::{XorShiftRng, Rng, SeedableRng};
use ::rand::os::OsRng;

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
    fn buffer<'a>(&'a self) -> &'a SampleBuffer { &self.buf }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

static mut _rand_gen: Option<OsRng> = None;

pub struct NoiseFactory;

impl GeneratorFactory for NoiseFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        if unsafe { &_rand_gen }.is_none() {
            unsafe {_rand_gen = Some(OsRng::new().expect("Couldn't initialize OS random")); }
        }

        let mut seed: [u32; 4] = Default::default();
        for i in seed.iter_mut() {
            *i = unsafe { &mut _rand_gen }.as_mut().unwrap().next_u32();
        }

        Ok(Box::new(Noise {
            rng: XorShiftRng::from_seed(seed),
            buf: SampleBuffer::new(params.env.default_buffer_size),
        }))
    }
}

pub static Factory: NoiseFactory = NoiseFactory;
