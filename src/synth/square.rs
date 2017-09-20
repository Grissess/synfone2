use super::*;

#[derive(Debug)]
pub struct Square {
    pub freq: GenBox,
    pub phase: f32,
    pub buf: SampleBuffer,
}

impl Generator for Square {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        let pvel = self.freq.eval(params).first() / params.env.sample_rate;
        for i in 0..self.buf.len() {
            self.buf[i] = if ((self.phase + pvel * (i as f32)) % 1.0) < 0.5 {
                -1.0
            } else {
                1.0
            };
        }

        self.phase = (self.phase + pvel * (self.buf.len() as f32)) % 1.0;
        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
