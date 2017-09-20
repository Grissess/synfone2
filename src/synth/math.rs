use super::*;
use std::mem;

#[derive(Debug)]
pub struct Add {
    pub terms: Vec<GenBox>,
    pub buf: SampleBuffer,
}

impl Generator for Add {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        if self.terms.is_empty() {
            self.buf.zero();
        } else {
            let (first, next) = self.terms.split_at_mut(1);
            self.buf.update_from(first[0].eval(params));
            for term in next {
                self.buf.sum_into(term.eval(params));
            }
        }
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

#[derive(Debug)]
pub struct Mul {
    pub factors: Vec<GenBox>,
    pub buf: SampleBuffer,
}

impl Generator for Mul {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        if self.factors.is_empty() {
            self.buf.zero();
        } else {
            let (first, next) = self.factors.split_at_mut(1);
            self.buf.update_from(first[0].eval(params));
            for factor in next {
                self.buf.mul_into(factor.eval(params));
            }
        }
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

#[derive(Debug)]
pub struct Negate {
    pub value: GenBox,
    pub buf: SampleBuffer,
}

impl Generator for Negate {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.update_from(self.value.eval(params));
        match self.buf.rate {
            Rate::Sample => {
                for v in self.buf.iter_mut() {
                    *v *= -1.0;
                }
            },
            Rate::Control => {
                self.buf[0] *= -1.0;
            },
        }
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

#[derive(Debug)]
pub struct Reciprocate {
    pub value: GenBox,
    pub buf: SampleBuffer,
}

impl Generator for Reciprocate {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.update_from(self.value.eval(params));
        match self.buf.rate {
            Rate::Sample => {
                for v in self.buf.iter_mut() {
                    *v = v.powf(-1.0);
                }
            },
            Rate::Control => {
                self.buf[0] = self.buf[0].powf(-1.0);
            },
        }
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
