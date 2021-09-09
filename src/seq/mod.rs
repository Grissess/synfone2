pub mod sequencer;
pub use self::sequencer::*;
pub mod file;

use std::collections::HashMap;
use std::{cmp, ops};

use super::Pitch;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seconds(pub f32);

impl Eq for Seconds {}

impl PartialOrd for Seconds {
    fn partial_cmp(&self, other: &Seconds) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Seconds {
    fn cmp(&self, other: &Seconds) -> cmp::Ordering {
        self.partial_cmp(other).expect("Encountered NaN Seconds")
    }
}

impl ops::Add for Seconds {
    type Output = Seconds;
    fn add(self, rhs: Seconds) -> Seconds {
        Seconds(self.0 + rhs.0)
    }
}

impl ops::Sub for Seconds {
    type Output = Seconds;
    fn sub(self, rhs: Seconds) -> Seconds {
        Seconds(self.0 - rhs.0)
    }
}

impl<RHS> ops::Mul<RHS> for Seconds
where
    f32: ops::Mul<RHS, Output = f32>,
{
    type Output = Seconds;
    fn mul(self, rhs: RHS) -> Seconds {
        Seconds(self.0.mul(rhs))
    }
}

impl<RHS> ops::Div<RHS> for Seconds
where
    f32: ops::Div<RHS, Output = f32>,
{
    type Output = Seconds;
    fn div(self, rhs: RHS) -> Seconds {
        Seconds(self.0.div(rhs))
    }
}

pub type Ticks = u64;

#[derive(Debug, Clone)]
pub enum Time {
    Seconds(Seconds),
    Ticks(Ticks),
}

impl From<Seconds> for Time {
    fn from(s: Seconds) -> Time {
        Time::Seconds(s)
    }
}

impl From<Ticks> for Time {
    fn from(t: Ticks) -> Time {
        Time::Ticks(t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BPM(pub f32);

impl Eq for BPM {}

impl PartialOrd for BPM {
    fn partial_cmp(&self, other: &BPM) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for BPM {
    fn cmp(&self, other: &BPM) -> cmp::Ordering {
        self.partial_cmp(other).expect("Encountered NaN BPM")
    }
}

impl<RHS> ops::Mul<RHS> for BPM
where
    f32: ops::Mul<RHS, Output = f32>,
{
    type Output = BPM;
    fn mul(self, rhs: RHS) -> BPM {
        BPM(self.0.mul(rhs))
    }
}
impl<RHS> ops::Div<RHS> for BPM
where
    f32: ops::Div<RHS, Output = f32>,
{
    type Output = BPM;
    fn div(self, rhs: RHS) -> BPM {
        BPM(self.0.div(rhs))
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub time: Seconds,
    pub dur: Seconds,
    pub start_tick: Option<Ticks>,
    pub dur_ticks: Option<Ticks>,
    pub ampl: f32,
    pub pitch: Pitch,
}

#[derive(Debug, Clone)]
pub struct Aux {
    pub time: Seconds,
    pub data: String,
}

pub type NoteStream = Vec<Note>;
pub type AuxStream = Vec<Aux>;

#[derive(Debug, Clone)]
pub enum Stream {
    Note(NoteStream),
    Aux(AuxStream),
}

impl Stream {
    pub fn note_stream(&self) -> Option<&NoteStream> {
        match self {
            &Stream::Note(ref ns) => Some(ns),
            _ => None,
        }
    }
}

pub type Group = Vec<NoteStream>;

#[derive(Debug, Clone, Copy)]
pub struct BPMEntry {
    pub abstick: Ticks,
    pub bpm: BPM,
    pub realtime: Option<Seconds>,
}

#[derive(Debug, Clone)]
pub struct BPMTableInput {
    pub entries: Vec<BPMEntry>,
    pub resolution: f32,
}

impl From<BPMTableInput> for BPMTable {
    fn from(input: BPMTableInput) -> BPMTable {
        let mut range = input.entries.clone();
        range.sort_unstable_by_key(|&ent| ent.abstick);
        for ent in range.iter_mut() {
            ent.realtime = Some(Seconds(0.0));
        }
        for idx in 1..(range.len() - 1) {
            let tick = range[idx].abstick;
            let BPMEntry {
                abstick: ptick,
                bpm: pbpm,
                realtime: ptm,
            } = range[idx - 1];
            range[idx].realtime = Some(
                ptm.unwrap()
                    + Seconds((60.0 * ((tick - ptick) as f32)) / (pbpm * input.resolution).0),
            );
        }
        BPMTable {
            range: range,
            resolution: input.resolution,
        }
    }
}

pub struct BPMTable {
    range: Vec<BPMEntry>,
    resolution: f32,
}

impl BPMTable {
    pub fn to_seconds(&self, tm: Time) -> Seconds {
        match tm {
            Time::Seconds(s) => s,
            Time::Ticks(t) => match self.range.binary_search_by_key(&t, |&ent| ent.abstick) {
                Ok(idx) => self.range[idx].realtime.unwrap(),
                Err(idx) => {
                    let effidx = cmp::max(0, idx - 1);
                    let BPMEntry {
                        abstick: tick,
                        bpm,
                        realtime: sec,
                    } = self.range[effidx];
                    sec.unwrap() + Seconds((60.0 * ((t - tick) as f32)) / (bpm * self.resolution).0)
                }
            },
        }
    }

    pub fn to_seconds_time(&self, tm: Time) -> Time {
        Time::Seconds(self.to_seconds(tm))
    }

    pub fn to_ticks(&self, tm: Time) -> Ticks {
        match tm {
            Time::Ticks(t) => t,
            Time::Seconds(s) => match self
                .range
                .binary_search_by_key(&s, |&ent| ent.realtime.unwrap())
            {
                Ok(idx) => self.range[idx].abstick,
                Err(idx) => {
                    let effidx = cmp::max(0, idx - 1);
                    let BPMEntry {
                        abstick: tick,
                        bpm,
                        realtime: sec,
                    } = self.range[effidx];
                    tick + ((((s - sec.unwrap()).0 * bpm.0 * self.resolution) / 60.0) as Ticks)
                }
            },
        }
    }

    pub fn to_ticks_time(&self, tm: Time) -> Time {
        Time::Ticks(self.to_ticks(tm))
    }
}

#[derive(Default)]
pub struct IVMeta {
    pub bpms: Option<BPMTable>,
    pub args: Option<String>,
    pub app: Option<String>,
}

#[derive(Default)]
pub struct IV {
    pub default_group: Group,
    pub groups: HashMap<String, Group>,
    pub meta: IVMeta,
}

impl IV {
    /* fn iter_streams(&self) -> impl Iterator<Item=&NoteStream> {
        self.groups.values().chain(iter::once(&self.default_group)).flat_map(|x| x.iter())
    } */
}
