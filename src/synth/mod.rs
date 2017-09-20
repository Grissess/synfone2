use std::{iter, cmp, slice, mem};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};
use std::collections::HashMap;
use super::*;

use ::byteorder::ByteOrder;

#[derive(PartialEq,Eq,Clone,Copy,Debug)]
pub enum Rate {
    Sample,
    Control,
}

#[derive(Debug)]
pub struct SampleBuffer {
    pub samples: Vec<Sample>,
    pub rate: Rate,
}

pub struct Environment {
    pub sample_rate: f32,
    pub default_buffer_size: usize,
}

impl Default for Environment {
    fn default() -> Environment {
        Environment {
            sample_rate: 44100.0,
            default_buffer_size: 64,
        }
    }
}

pub struct Parameters {
    pub env: Environment,
    pub vars: HashMap<String, f32>,
}

impl Default for Parameters {
    fn default() -> Parameters {
        Parameters {
            env: Default::default(),
            vars: HashMap::new(),
        }
    }
}

impl SampleBuffer {
    pub fn new(sz: usize) -> SampleBuffer {
        let mut samples = Vec::with_capacity(sz);
        samples.extend(iter::repeat(0 as Sample).take(sz));
        SampleBuffer {
            samples: samples,
            rate: Rate::Sample,
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn first(&self) -> Sample {
        *self.samples.first().unwrap()
    }

    pub fn set(&mut self, val: Sample) {
        self.samples[0] = val;
        self.rate = Rate::Control;
    }

    pub fn update_from(&mut self, other: &SampleBuffer) {
        self.rate = other.rate;
        match self.rate {
            Rate::Sample => {
                for i in 0..cmp::min(self.len(), other.len()) {
                    self.samples[i] = other.samples[i];
                }
            },
            Rate::Control => {
                self.samples[0] = other.samples[0];
            },
        }
    }

    pub fn sum_into(&mut self, other: &SampleBuffer) {
        match self.rate {
            Rate::Sample => {
                let bound = match other.rate {
                    Rate::Sample => cmp::min(self.len(), other.len()),
                    Rate::Control => self.len(),
                };
                for i in 0..bound {
                    self.samples[i] += match other.rate {
                        Rate::Sample => other.samples[i],
                        Rate::Control => other.samples[0],
                    };
                }
            },
            Rate::Control => {
                self.samples[0] += other.samples[0];
            },
        }
    }

    pub fn mul_into(&mut self, other: &SampleBuffer) {
        match self.rate {
            Rate::Sample => {
                let bound = match other.rate {
                    Rate::Sample => cmp::min(self.len(), other.len()),
                    Rate::Control => self.len(),
                };
                for i in 0..bound {
                    self.samples[i] *= match other.rate {
                        Rate::Sample => other.samples[i],
                        Rate::Control => other.samples[0],
                    };
                }
            },
            Rate::Control => {
                self.samples[0] *= other.samples[0];
            },
        }
    }

    pub fn zero(&mut self) {
        for i in 0..self.len() {
            self.samples[i] = 0.0;
        }
    }

    pub fn size(&self) -> usize {
        mem::size_of::<Sample>() * self.samples.len()
    }

    pub fn bytes(&self, buf: &mut [u8]) {
        // FIXME: Depends on f32 instead of Sample alias
        ::byteorder::LittleEndian::write_f32_into(&self.samples, buf);
    }
}

impl Index<usize> for SampleBuffer {
    type Output = Sample;
    fn index(&self, idx: usize) -> &Sample { &self.samples[idx] }
}

impl IndexMut<usize> for SampleBuffer {
    fn index_mut(&mut self, idx: usize) -> &mut Sample { &mut self.samples[idx] }
}

pub trait Generator : Debug {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer;
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer;
}

pub type GenBox = Box<Generator>;

pub mod param;
pub use self::param::Param;
pub mod math;
pub use self::math::{Add, Mul};
pub mod sine;
pub use self::sine::Sine;
pub mod saw;
pub use self::saw::Saw;
pub mod triangle;
pub use self::triangle::Triangle;
pub mod square;
pub use self::square::Square;
//pub mod asdr;
//pub use self::asdr::ASDR;
