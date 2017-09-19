use std::{iter, cmp, slice, mem};
use std::ops::{Index, IndexMut};
use std::collections::HashMap;
use super::*;

#[derive(PartialEq,Eq,Clone,Copy)]
pub enum Rate {
    Sample,
    Control,
}

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
                for i in 0..cmp::min(self.len(), other.len()) {
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
                for i in 0..cmp::min(self.len(), other.len()) {
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

    pub fn bytes<'a>(&'a self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(
                self.samples.as_ptr() as *const u8,
                self.samples.len() * mem::size_of::<Sample>(),
            )
        }
    }
}

impl Index<usize> for SampleBuffer {
    type Output = Sample;
    fn index(&self, idx: usize) -> &Sample { &self.samples[idx] }
}

impl IndexMut<usize> for SampleBuffer {
    fn index_mut(&mut self, idx: usize) -> &mut Sample { &mut self.samples[idx] }
}

pub trait Generator {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer;
}

pub type GenBox = Box<Generator>;

pub mod param;
pub use self::param::Param;
pub mod math;
pub use self::math::{Add, Mul};
pub mod sine;
pub use self::sine::Sine;
//pub mod saw;
//pub use saw::Saw;
//pub mod triangle;
//pub use triangle::Triangle;

