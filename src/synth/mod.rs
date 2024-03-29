#![allow(non_upper_case_globals)]

use super::Sample;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};
use std::{cmp, fmt, iter, mem, slice};

use ::byteorder::ByteOrder;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Rate {
    Sample,
    Control,
}

#[derive(Debug)]
pub struct SampleBuffer {
    pub samples: Vec<Sample>,
    pub rate: Rate,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub sample_rate: f32,
    pub default_buffer_size: usize,
}

impl Default for Environment {
    fn default() -> Environment {
        Environment {
            sample_rate: 44100.0,
            default_buffer_size: 64,
        }
    }
}

pub struct Parameters {
    pub env: Environment,
    pub vars: HashMap<String, f32>,
}

impl Default for Parameters {
    fn default() -> Parameters {
        Parameters {
            env: Default::default(),
            vars: HashMap::new(),
        }
    }
}

impl SampleBuffer {
    pub fn new(sz: usize) -> SampleBuffer {
        let mut samples = Vec::with_capacity(sz);
        samples.extend(iter::repeat(0 as Sample).take(sz));
        SampleBuffer {
            samples: samples,
            rate: Rate::Sample,
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn iter(&self) -> slice::Iter<f32> {
        self.samples.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<f32> {
        self.samples.iter_mut()
    }

    pub fn first(&self) -> Sample {
        *self.samples.first().unwrap()
    }

    pub fn set(&mut self, val: Sample) {
        self.samples[0] = val;
        self.rate = Rate::Control;
    }

    pub fn update_from(&mut self, other: &SampleBuffer) {
        self.rate = other.rate;
        match self.rate {
            Rate::Sample => {
                let len = cmp::min(self.len(), other.len());
                self.samples[..len].clone_from_slice(&other.samples[..len]);
            }
            Rate::Control => {
                self.samples[0] = other.samples[0];
            }
        }
    }

    pub fn sum_into(&mut self, other: &SampleBuffer) {
        match self.rate {
            Rate::Sample => {
                match other.rate {
                    Rate::Sample => {
                        for (elt, oelt) in self.samples.iter_mut().zip(other.samples.iter()) {
                            *elt += *oelt;
                        }
                    }
                    Rate::Control => {
                        for elt in &mut self.samples {
                            *elt += other.samples[0];
                        }
                    }
                };
            }
            Rate::Control => {
                self.samples[0] += other.samples[0];
            }
        }
    }

    pub fn mul_into(&mut self, other: &SampleBuffer) {
        match self.rate {
            Rate::Sample => {
                match other.rate {
                    Rate::Sample => {
                        for (elt, oelt) in self.samples.iter_mut().zip(other.samples.iter()) {
                            *elt *= *oelt;
                        }
                    }
                    Rate::Control => {
                        for elt in &mut self.samples {
                            *elt *= other.samples[0];
                        }
                    }
                };
            }
            Rate::Control => {
                self.samples[0] *= other.samples[0];
            }
        }
    }

    pub fn zero(&mut self) {
        for i in 0..self.len() {
            self.samples[i] = 0.0;
        }
    }

    pub fn size(&self) -> usize {
        mem::size_of::<Sample>() * self.samples.len()
    }

    pub fn write_bytes(&self, buf: &mut [u8]) {
        // FIXME: Depends on f32 instead of Sample alias
        ::byteorder::LittleEndian::write_f32_into(&self.samples, buf);
    }
}

impl Index<usize> for SampleBuffer {
    type Output = Sample;
    fn index(&self, idx: usize) -> &Sample {
        &self.samples[idx]
    }
}

impl IndexMut<usize> for SampleBuffer {
    fn index_mut(&mut self, idx: usize) -> &mut Sample {
        &mut self.samples[idx]
    }
}

impl Clone for SampleBuffer {
    fn clone(&self) -> SampleBuffer {
        SampleBuffer {
            samples: self.samples.clone(),
            rate: self.rate,
        }
    }
}

pub trait Generator: Debug + Send {
    fn eval<'a>(&'a mut self, params: &Parameters) -> &'a SampleBuffer;
    fn buffer(&self) -> &SampleBuffer;
    fn set_buffer(&mut self, buf: SampleBuffer) -> SampleBuffer;
}

pub type GenBox = Box<dyn Generator>;

#[derive(Debug)]
pub enum GenFactoryError {
    MissingRequiredParam(String, usize),
    CannotConvert(ParamKind, ParamKind),
    BadType(ParamKind),
}

#[derive(Debug)]
pub struct GenFactoryErrorType {
    pub kind: GenFactoryError,
    desc: String,
}

impl GenFactoryErrorType {
    pub fn new(kind: GenFactoryError) -> GenFactoryErrorType {
        let mut ret = GenFactoryErrorType {
            kind: kind,
            desc: "".to_string(),
        };

        ret.desc = match ret.kind {
            GenFactoryError::MissingRequiredParam(ref name, pos) => {
                format!("Needed a parameter named {} or at pos {}", name, pos)
            }
            GenFactoryError::CannotConvert(from, to) => {
                format!("Cannot convert {:?} to {:?}", from, to)
            }
            GenFactoryError::BadType(ty) => format!("Bad parameter type {:?}", ty),
        };

        ret
    }

    pub fn with_description(kind: GenFactoryError, desc: String) -> GenFactoryErrorType {
        GenFactoryErrorType {
            kind: kind,
            desc: desc,
        }
    }
}

impl From<GenFactoryError> for GenFactoryErrorType {
    fn from(e: GenFactoryError) -> GenFactoryErrorType {
        GenFactoryErrorType::new(e)
    }
}

impl Error for GenFactoryErrorType {
    fn description(&self) -> &str {
        &self.desc
    }
}

impl fmt::Display for GenFactoryErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.to_string())
    }
}

impl Into<Box<dyn Error>> for GenFactoryError {
    fn into(self) -> Box<dyn Error> {
        Box::new(GenFactoryErrorType::new(self))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ParamKind {
    Integer,
    Float,
    String,
    Generator,
}

#[derive(Debug)]
pub enum ParamValue {
    Integer(isize),
    Float(f32),
    String(String),
    Generator(GenBox),
}

impl ParamValue {
    pub fn kind(&self) -> ParamKind {
        match *self {
            ParamValue::Integer(_) => ParamKind::Integer,
            ParamValue::Float(_) => ParamKind::Float,
            ParamValue::String(_) => ParamKind::String,
            ParamValue::Generator(_) => ParamKind::Generator,
        }
    }

    pub fn as_isize(&mut self) -> Result<isize, GenFactoryError> {
        match *self {
            ParamValue::Integer(v) => Ok(v),
            ParamValue::Float(v) => Ok(v as isize),
            ParamValue::String(ref v) => v
                .parse()
                .map_err(|_| GenFactoryError::CannotConvert(ParamKind::String, ParamKind::Integer)),
            ParamValue::Generator(ref mut g) => Ok(g.eval(&Default::default()).first() as isize),
        }
    }

    pub fn as_f32(&mut self) -> Result<f32, GenFactoryError> {
        match *self {
            ParamValue::Integer(v) => Ok(v as f32),
            ParamValue::Float(v) => Ok(v),
            ParamValue::String(ref v) => v
                .parse()
                .map_err(|_| GenFactoryError::CannotConvert(ParamKind::String, ParamKind::Float)),
            ParamValue::Generator(ref mut g) => Ok(g.eval(&Default::default()).first()),
        }
    }

    pub fn as_string(&self) -> Result<String, GenFactoryError> {
        match *self {
            ParamValue::Integer(v) => Ok(v.to_string()),
            ParamValue::Float(v) => Ok(v.to_string()),
            ParamValue::String(ref v) => Ok(v.clone()),
            ParamValue::Generator(_) => Err(GenFactoryError::CannotConvert(
                ParamKind::Generator,
                ParamKind::String,
            )),
        }
    }

    pub fn into_gen(self) -> Result<GenBox, GenFactoryError> {
        match self {
            ParamValue::Integer(v) => Ok(Box::new(self::param::Param {
                name: "_".to_string(),
                default: v as f32,
                buf: SampleBuffer::new(1),
            })),
            ParamValue::Float(v) => Ok(Box::new(self::param::Param {
                name: "_".to_string(),
                default: v,
                buf: SampleBuffer::new(1),
            })),
            ParamValue::String(_) => Err(GenFactoryError::CannotConvert(
                ParamKind::String,
                ParamKind::Generator,
            )),
            ParamValue::Generator(g) => Ok(g),
        }
    }
}

impl<'a> From<&'a ParamValue> for ParamKind {
    fn from(val: &'a ParamValue) -> ParamKind {
        val.kind()
    }
}

#[derive(Debug, Default)]
pub struct FactoryParameters {
    pub env: Environment,
    pub vars: HashMap<String, ParamValue>,
}

impl FactoryParameters {
    pub fn get_param<'a, 'b: 'a>(
        &'a mut self,
        name: &str,
        position: usize,
        default: &'b mut ParamValue,
    ) -> &'a mut ParamValue {
        let position = position.to_string();

        match (
            self.vars.contains_key(name),
            self.vars.contains_key(&position),
        ) {
            (true, _) => self.vars.get_mut(name).unwrap(),
            (false, true) => self.vars.get_mut(&position).unwrap(),
            (false, false) => default,
        }
    }

    pub fn get_req_param(
        &mut self,
        name: &str,
        position: usize,
    ) -> Result<&mut ParamValue, GenFactoryError> {
        let pos = position.to_string();

        match (self.vars.contains_key(name), self.vars.contains_key(&pos)) {
            (true, _) => Ok(self.vars.get_mut(name).unwrap()),
            (false, true) => Ok(self.vars.get_mut(&pos).unwrap()),
            (false, false) => Err(GenFactoryError::MissingRequiredParam(
                name.to_string(),
                position,
            )),
        }
    }

    pub fn remove_param(
        &mut self,
        name: &str,
        position: usize,
    ) -> Result<ParamValue, GenFactoryError> {
        match self
            .vars
            .remove(name)
            .or_else(|| self.vars.remove(&position.to_string()))
        {
            Some(v) => Ok(v),
            None => Err(GenFactoryError::MissingRequiredParam(
                name.to_string(),
                position,
            )),
        }
    }

    pub fn get_pos_params(&mut self) -> Vec<ParamValue> {
        let mut ret = Vec::new();

        for i in 0.. {
            match self.vars.remove(&i.to_string()) {
                Some(v) => ret.push(v),
                None => return ret,
            }
        }

        unreachable!()
    }
}

pub trait GeneratorFactory {
    // NB: Like above, &self is for object safety. This should have an associated type, but that
    // would compromise object safety; for the same reason, the return of this may only be a
    // Box<Generator>, which necessitates allocation.
    fn new(&self, params: &mut FactoryParameters) -> Result<GenBox, GenFactoryError>;
}

pub mod param;
pub use self::param::Param;
pub mod math;
pub use self::math::{Add, Mul, Negate, Reciprocate};
pub mod rel;
pub use self::rel::{Rel, RelOp};
pub mod logic;
pub use self::logic::IfElse;
pub mod util;
pub use self::util::{ControlRate, SampleRate};
pub mod lut;
pub use self::lut::Lut;
pub mod sine;
pub use self::sine::Sine;
pub mod saw;
pub use self::saw::Saw;
pub mod triangle;
pub use self::triangle::Triangle;
pub mod square;
pub use self::square::Square;
pub mod noise;
pub use self::noise::Noise;
pub mod adsr;
pub use self::adsr::DAHDSR;

pub fn all_factories() -> HashMap<String, &'static dyn GeneratorFactory> {
    let mut ret = HashMap::new();

    ret.insert(
        "param".to_string(),
        &self::param::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "add".to_string(),
        &self::math::FactoryAdd as &dyn GeneratorFactory,
    );
    ret.insert(
        "mul".to_string(),
        &self::math::FactoryMul as &dyn GeneratorFactory,
    );
    ret.insert(
        "negate".to_string(),
        &self::math::FactoryNegate as &dyn GeneratorFactory,
    );
    ret.insert(
        "reciprocate".to_string(),
        &self::math::FactoryReciprocate as &dyn GeneratorFactory,
    );
    ret.insert(
        "rel".to_string(),
        &self::rel::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "ifelse".to_string(),
        &self::logic::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "controlrate".to_string(),
        &self::util::FactoryControlRate as &dyn GeneratorFactory,
    );
    ret.insert(
        "samplerate".to_string(),
        &self::util::FactorySampleRate as &dyn GeneratorFactory,
    );
    ret.insert(
        "lutdata".to_string(),
        &self::lut::FactoryLutData as &dyn GeneratorFactory,
    );
    ret.insert(
        "lutgen".to_string(),
        &self::lut::FactoryLutGen as &dyn GeneratorFactory,
    );
    ret.insert(
        "sine".to_string(),
        &self::sine::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "saw".to_string(),
        &self::saw::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "triangle".to_string(),
        &self::triangle::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "square".to_string(),
        &self::square::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "noise".to_string(),
        &self::noise::Factory as &dyn GeneratorFactory,
    );
    ret.insert(
        "dahdsr".to_string(),
        &self::adsr::Factory as &dyn GeneratorFactory,
    );

    ret
}
