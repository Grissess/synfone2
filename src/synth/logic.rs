use std::{mem, cmp};
use super::*;

#[derive(Debug)]
pub struct IfElse {
    pub cond: GenBox,
    pub iftrue: GenBox,
    pub iffalse: GenBox,
    pub buf: SampleBuffer,
}

impl Generator for IfElse {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        let cond_buf = self.cond.eval(params);
        let iftrue_buf = self.iftrue.eval(params);
        let iffalse_buf = self.iffalse.eval(params);
        
        if (
            cond_buf.rate == Rate::Control &&
            iftrue_buf.rate == Rate::Control &&
            iffalse_buf.rate == Rate::Control
        ) {
            self.buf.set(if cond_buf.first() >= 0.5 {
                iftrue_buf.first()
            } else {
                iffalse_buf.first()
            });
            return &self.buf;
        }

        self.buf.rate = Rate::Sample;

        let mut bound = self.buf.len();
        if cond_buf.rate == Rate::Sample { bound = cmp::min(bound, cond_buf.len()); }
        if iftrue_buf.rate == Rate::Sample { bound = cmp::min(bound, iftrue_buf.len()); }
        if iffalse_buf.rate == Rate::Sample { bound = cmp::min(bound, iffalse_buf.len()); }

        for i in 0..bound {
            let tv = match iftrue_buf.rate {
                Rate::Sample => iftrue_buf[i],
                Rate::Control => iftrue_buf.first(),
            };
            let fv = match iffalse_buf.rate {
                Rate::Sample => iffalse_buf[i],
                Rate::Control => iffalse_buf.first(),
            };
            let cv = match cond_buf.rate {
                Rate::Sample => cond_buf[i],
                Rate::Control => cond_buf.first(),
            };
            self.buf[i] = if cv >= 0.5 {
                tv
            } else {
                fv
            };
        }

        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
