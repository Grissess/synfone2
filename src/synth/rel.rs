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

/* TODO
impl<T: PartialEq<isize>> From<T> for RelOp {
    fn from(i: T) -> RelOp {
        match i {
            0 => RelOp::Greater,
            1 => RelOp::GreaterEqual,
            _ => RelOp::Equal,
            3 => RelOp::NotEqual,
            4 => RelOp::LessEqual,
            5 => RelOp::Less,
        }
    }
}
*/

impl<'a> From<&'a str> for RelOp {
    fn from(s: &'a str) -> RelOp {
        if s == ">" {
            RelOp::Greater
        } else if s == ">=" {
            RelOp::GreaterEqual
        } else if s == "!=" {
            RelOp::NotEqual
        } else if s == "<=" {
            RelOp::LessEqual
        } else if s == "<" {
            RelOp::Less
        } else {
            RelOp::Equal
        }
    }
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
    fn buffer(&self) -> &SampleBuffer { &self.buf }
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer {
        mem::replace(&mut self.buf, buf)
    }
}

pub struct RelFactory;

impl GeneratorFactory for RelFactory {
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError> {
        let op = match *params.get_req_param("rel", 1)? {
            /* TODO
            ParamValue::Integer(v) => v.into(),
            ParamValue::Float(v) => (v as isize).into(),
            */
            ParamValue::Integer(_) => return Err(GenFactoryError::BadType(ParamKind::Integer)),
            ParamValue::Float(_) => return Err(GenFactoryError::BadType(ParamKind::Float)),
            ParamValue::String(ref v) => (&*v as &str).into(),
            ParamValue::Generator(_) => return Err(GenFactoryError::BadType(ParamKind::Generator)),
        };
        let left = params.remove_param("left", 0)?.into_gen()?;
        let right = params.remove_param("right", 2)?.into_gen()?;
        let buf = SampleBuffer::new(cmp::max(left.buffer().len(), right.buffer().len()));
        Ok(Box::new(Rel {
            left: left,
            right: right,
            op: op,
            buf: buf,
        }))
    }
}

pub static Factory: RelFactory = RelFactory;
