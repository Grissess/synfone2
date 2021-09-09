use super::{
    FactoryParameters, GenBox, GenFactoryError, Generator, GeneratorFactory, ParamValue,
    Parameters, Rate, SampleBuffer,
};
use std::mem;

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
    fn buffer(&self) -> &SampleBuffer {
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct TriangleFactory;

impl GeneratorFactory for TriangleFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(Triangle {
            freq: params.remove_param("freq", 0)?.into_gen()?,
            phase: params
                .get_param("phase", 1, &mut ParamValue::Float(0.0))
                .as_f32()?,
            buf: SampleBuffer::new(params.env.default_buffer_size),
        }))
    }
}

pub static Factory: TriangleFactory = TriangleFactory;
