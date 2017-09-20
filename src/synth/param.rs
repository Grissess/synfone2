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
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
