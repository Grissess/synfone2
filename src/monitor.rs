use super::*;
use std::collections::HashMap;
use synth::SampleBuffer;

pub struct VoiceDatum {
    pitch: Pitch,
    ampl: f32,
}

pub enum Datum {
    Voices(Vec<VoiceDatum>),
    Samples(SampleBuffer),
    Playtime(f32, f32),
}

pub enum DatumKind {
    Voices,
    Samples,
    Playtime,
}

impl<'a> From<&'a Datum> for DatumKind {
    fn from(d: &'a Datum) -> DatumKind {
        match *d {
            Datum::Voices(_) => DatumKind::Voices,
            Datum::Samples(_) => DatumKind::Samples,
            Datum::Playtime(_, _) => DatumKind::Playtime,
        }
    }
}

pub type Data = HashMap<DatumKind, Datum>;

pub trait Monitor {
    fn process(&mut self, data: &Data);
}

pub type MonBox = Box<dyn Monitor>;
