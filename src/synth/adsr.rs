use super::*;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Phase {
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug)]
pub struct DAHDSR {
    pub delay: GenBox,
    pub attack: GenBox,
    pub hold: GenBox,
    pub decay: GenBox,
    pub sustain: GenBox,
    pub release: GenBox,
    pub gate: GenBox,
    pub phase: Phase,
    pub cur: f32,
    pub attack_cd: f32,
    pub decay_cd: f32,
    pub buf: SampleBuffer,
}

impl Generator for DAHDSR {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        self.buf.rate = Rate::Sample;

        let delay = self.delay.eval(params).first();
        let attack = self.attack.eval(params).first();
        let hold = self.attack.eval(params).first();
        let decay = self.decay.eval(params).first();
        let sustain = self.sustain.eval(params).first();
        let release = self.release.eval(params).first();
        let gate = self.gate.eval(params).first();

        if gate >= 0.5 {
            if self.phase == Phase::Release {
                self.phase = Phase::Delay;
                self.attack_cd = delay;
                self.cur = 0.0;
            }
        } else{
            self.phase = Phase::Release;
        }

        for samp in self.buf.samples.iter_mut() {
            match self.phase {
                Phase::Delay => {
                    self.attack_cd -= 1.0;
                    if self.attack_cd <= 0.0 {
                        self.phase = Phase::Attack;
                    }
                },
                Phase::Attack => {
                    self.cur += attack;
                    if self.cur >= 1.0 {
                        self.cur = 1.0;
                        self.phase = Phase::Hold;
                        self.decay_cd = hold;
                    }
                },
                Phase::Hold => {
                    self.decay_cd -= 1.0;
                    if self.decay_cd <= 0.0 {
                        self.phase = Phase::Decay;
                    }
                },
                Phase::Decay => {
                    self.cur -= decay;
                    if self.cur <= sustain {
                        self.cur = sustain;
                        self.phase = Phase::Sustain;
                    }
                },
                Phase::Sustain => {
                    self.cur = sustain;
                },
                Phase::Release => {
                    self.cur -= release;
                    if self.cur < 0.0 {
                        self.cur = 0.0;
                    }
                },
            }
            *samp = self.cur;
        }

        &self.buf
    }
    fn buffer(&self) -> &SampleBuffer { &self.buf }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct DAHDSRFactory;

impl GeneratorFactory for DAHDSRFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        Ok(Box::new(DAHDSR {
            delay: params.remove_param("delay", 1)?.into_gen()?,
            attack: params.remove_param("attack", 2)?.into_gen()?,
            hold: params.remove_param("hold", 3)?.into_gen()?,
            decay: params.remove_param("decay", 4)?.into_gen()?,
            sustain: params.remove_param("sustain", 5)?.into_gen()?,
            release: params.remove_param("release", 6)?.into_gen()?,
            gate: params.remove_param("gate", 0)?.into_gen()?,
            phase: Phase::Release,
            cur: 0.0,
            attack_cd: 0.0,
            decay_cd: 0.0,
            buf: SampleBuffer::new(params.env.default_buffer_size),
        }))
    }
}

pub static Factory: DAHDSRFactory = DAHDSRFactory;
