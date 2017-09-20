use std::mem;
use std::time::Duration;
use super::*;

use ::byteorder::{ByteOrder, NetworkEndian};

const OBLIGATE_POLYPHONE: u32 = 0xffffffff;

pub enum Command {
    KeepAlive,
    Ping{data: [u8; 32]},
    Quit,
    Play{sec: u32, usec: u32, freq: u32, amp: f32, voice: u32},
    Caps{voices: u32, tp: [u8; 4], ident: [u8; 24]},
    PCM{samples: [i16; 16]},
    Unknown{data: [u8; 36]},
}

impl Command {
    const SIZE: usize = 36;

    fn duration(&self) -> Option<Duration> {
        match *self {
            Command::Play{sec, usec, ..} => Some(Duration::new(sec as u64, usec * 1000)),
            _ => None,
        }
    }

    fn pitch(&self) -> Option<Pitch> {
        match *self {
            Command::Play{freq, ..} => Some(Pitch::Freq(freq as f32)),
            _ => None,
        }
    }
}

impl<'a> From<&'a [u8; 36]> for Command {
    fn from(packet: &'a [u8; 36]) -> Command {
        let mut fields_u32: [u32; 9] = unsafe { mem::uninitialized() };
        let mut fields_f32: [f32; 9] = unsafe { mem::uninitialized() };
        NetworkEndian::read_u32_into(packet, &mut fields_u32);
        unsafe { NetworkEndian::read_f32_into_unchecked(packet, &mut fields_f32); }

        match fields_u32[0] {
            0 => Command::KeepAlive,
            1 => {
                let mut data: [u8; 32] = unsafe { mem::uninitialized() };
                data.copy_from_slice(&packet[4..]);
                Command::Ping{data: data}
            }
            _ => {
                let mut data: [u8; 36] = unsafe { mem::uninitialized() };
                data.copy_from_slice(packet);
                Command::Unknown{data: data}
            }
        }
    }
}
