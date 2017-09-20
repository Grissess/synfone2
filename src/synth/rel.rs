use std::{cmp, mem};
use super::*;

#[derive(Debug)]
pub enum RelOp {
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    LessEqual,
    Less,
}

#[derive(Debug)]
pub struct Rel {
    pub left: GenBox,
    pub right: GenBox,
    pub op: RelOp,
    pub buf: SampleBuffer,
}

impl Generator for Rel {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer {
        let left_buf = self.left.eval(params);
        let right_buf = self.right.eval(params);

        match left_buf.rate {
            Rate::Sample => {
                self.buf.rate = Rate::Sample;

                let bound = match right_buf.rate {
                    Rate::Sample => cmp::min(left_buf.len(), right_buf.len()),
                    Rate::Control => left_buf.len(),
                };
                for i in 0..bound {
                    let val = left_buf[i];
                    let thres = match right_buf.rate {
                        Rate::Sample => right_buf[i],
                        Rate::Control => right_buf.first(),
                    };
                    self.buf[i] = if match self.op {
                        RelOp::Greater => val > thres,
                        RelOp::GreaterEqual => val >= thres,
                        RelOp::Equal => val == thres,
                        RelOp::NotEqual => val != thres,
                        RelOp::LessEqual => val <= thres,
                        RelOp::Less => val < thres,
                    } {
                        1.0
                    } else {
                        0.0
                    };
                }
            },
            Rate::Control => {
                let val = left_buf.first();
                let thres = right_buf.first();
                self.buf.set(if match self.op {
                    RelOp::Greater => val > thres,
                    RelOp::GreaterEqual => val >= thres,
                    RelOp::Equal => val == thres,
                    RelOp::NotEqual => val != thres,
                    RelOp::LessEqual => val <= thres,
                    RelOp::Less => val < thres,
                } {
                    1.0
                } else {
                    0.0
                });
            },
        }

        &self.buf
    }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}
