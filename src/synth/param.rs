use super::*;

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub default: Sample,
    pub buf: SampleBuffer,
}

impl Generator for Param {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.set(*params.vars.get(&self.name).unwrap_or(&self.default));
        &self.buf
    }
    fn buffer(&self) -> &SampleBuffer { &self.buf }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct ParamFactory;

impl GeneratorFactory for ParamFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(Param {
            name: params.get_req_param("name", 0)?.as_string()?,
            default: params.get_param("default", 1, &mut ParamValue::Float(0.0)).as_f32()?,
            buf: SampleBuffer::new(1),
        }))
    }
}

pub static Factory: ParamFactory = ParamFactory;
