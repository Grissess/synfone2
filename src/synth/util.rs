use super::{
    mem, FactoryParameters, GenBox, GenFactoryError, Generator, GeneratorFactory, Parameters, Rate,
    SampleBuffer,
};

#[derive(Debug)]
pub struct ControlRate {
    pub value: GenBox,
    pub buf: SampleBuffer,
}

impl ControlRate {
    pub fn new(mut gen: GenBox) -> ControlRate {
        gen.set_buffer(SampleBuffer::new(1));
        ControlRate {
            value: gen,
            buf: SampleBuffer::new(1),
        }
    }
}

impl Generator for ControlRate {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Control;
        self.buf.update_from(self.value.eval(params));
        &self.buf
    }
    fn buffer(&self) -> &SampleBuffer {
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct ControlRateFactory;

impl GeneratorFactory for ControlRateFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(ControlRate::new(
            params.remove_param("gen", 0)?.into_gen()?,
        )))
    }
}

pub static FactoryControlRate: ControlRateFactory = ControlRateFactory;

#[derive(Debug)]
pub struct SampleRate {
    pub buf: SampleBuffer,
}

impl Generator for SampleRate {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.set(params.env.sample_rate);
        &self.buf
    }
    fn buffer(&self) -> &SampleBuffer {
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct SampleRateFactory;

impl GeneratorFactory for SampleRateFactory {
    fn new(&self, _params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(SampleRate {
            buf: SampleBuffer::new(1),
        }))
    }
}

pub static FactorySampleRate: SampleRateFactory = SampleRateFactory;
