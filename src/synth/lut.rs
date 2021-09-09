use super::{
    mem, FactoryParameters, GenBox, GenFactoryError, Generator, GeneratorFactory, ParamValue,
    Parameters, Rate, Sample, SampleBuffer,
};

#[derive(Debug)]
pub struct Lut {
    pub freq: GenBox,
    pub phase: f32,
    pub lut: Vec<Sample>,
    pub buf: SampleBuffer,
}

impl Generator for Lut {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        let pvel = self.freq.eval(params).first() / params.env.sample_rate;
        for i in 0..self.buf.len() {
            self.buf[i] = self.lut
                [(((self.phase + pvel * (i as f32)) % 1.0) * (self.lut.len() as f32)) as usize];
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

pub struct LutDataFactory;

impl GeneratorFactory for LutDataFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(Lut {
            freq: params.remove_param("freq", 0)?.into_gen()?,
            phase: params
                .get_param("phase", 1, &mut ParamValue::Float(0.0))
                .as_f32()?,
            buf: SampleBuffer::new(params.env.default_buffer_size),
            lut: {
                let mut lut: Vec<Sample> = Vec::new();
                let mut i = 0;

                while let Ok(samp) = params.get_req_param("_", 2 + i).and_then(|pv| pv.as_f32()) {
                    lut.push(samp);
                    i += 1;
                }

                if lut.is_empty() {
                    return Err(GenFactoryError::MissingRequiredParam(
                        "samples".to_string(),
                        2,
                    ));
                }

                lut
            },
        }))
    }
}

pub static FactoryLutData: LutDataFactory = LutDataFactory;

pub struct LutGenFactory;

impl GeneratorFactory for LutGenFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        eprintln!("LutGenFactory::new({:?})", params);
        Ok(Box::new(Lut {
            freq: params.remove_param("freq", 2)?.into_gen()?,
            phase: params
                .get_param("phase", 3, &mut ParamValue::Float(0.0))
                .as_f32()?,
            buf: SampleBuffer::new(params.env.default_buffer_size),
            lut: {
                let mut gen = params.remove_param("gen", 0)?.into_gen()?;
                let samps = params.get_req_param("samples", 1)?.as_f32()?;
                let var = params
                    .get_param("var", 4, &mut ParamValue::String("lut_freq".to_string()))
                    .as_string()?;
                let mut genparams = Parameters {
                    env: params.env.clone(),
                    ..Default::default()
                };
                genparams.env.sample_rate = samps;
                genparams.vars.insert(var, 1.0);

                gen.set_buffer(SampleBuffer::new(samps as usize));
                gen.eval(&genparams);

                gen.set_buffer(SampleBuffer::new(0)).samples
            },
        }))
    }
}

pub static FactoryLutGen: LutGenFactory = LutGenFactory;
