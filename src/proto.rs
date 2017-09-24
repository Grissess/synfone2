use std::mem;
use std::time::Duration;
use super::*;

use ::byteorder::{ByteOrder, NetworkEndian};

#[allow(dead_code)]
const OBLIGATE_POLYPHONE: u32 = 0xffffffff;

pub enum Command {
    KeepAlive,
    Ping{data: [u8; 32]},
    Quit,
    Play{sec: u32, usec: u32, freq: u32, amp: f32, voice: u32},
    Caps{voices: u32, tp: [u8; 4], ident: [u8; 24]},
    PCM{samples: [i16; 16]},
    Unknown{data: [u8; Command::SIZE]},
}

impl Command {
    const SIZE: usize = 36;

    pub fn duration(&self) -> Option<Duration> {
        match *self {
            Command::Play{sec, usec, ..} => Some(Duration::new(sec as u64, usec * 1000)),
            _ => None,
        }
    }

    pub fn pitch(&self) -> Option<Pitch> {
        match *self {
            Command::Play{freq, ..} => Some(Pitch::Freq(freq as f32)),
            _ => None,
        }
    }
}

impl<'a> From<&'a [u8; Command::SIZE]> for Command {
    fn from(packet: &'a [u8; Command::SIZE]) -> Command {
        let mut fields_u32: [u32; Command::SIZE / 4] = unsafe { mem::uninitialized() };
        let mut fields_f32: [f32; Command::SIZE / 4] = unsafe { mem::uninitialized() };
        NetworkEndian::read_u32_into(packet, &mut fields_u32);
        unsafe { NetworkEndian::read_f32_into_unchecked(packet, &mut fields_f32); }

        match fields_u32[0] {
            0 => Command::KeepAlive,
            1 => {
                let mut data: [u8; 32] = unsafe { mem::uninitialized() };
                data.copy_from_slice(&packet[4..]);
                Command::Ping{data: data}
            }
            2 => Command::Quit,
            3 => Command::Play{
                sec: fields_u32[1],
                usec: fields_u32[2],
                freq: fields_u32[3],
                amp: fields_f32[4],
                voice: fields_u32[5],
            },
            4 => {
                let mut tp: [u8; 4] = unsafe { mem::uninitialized() };
                let mut ident: [u8; 24] = unsafe { mem::uninitialized() };
                tp.copy_from_slice(&packet[8..12]);
                ident.copy_from_slice(&packet[12..]);
                Command::Caps{
                    voices: fields_u32[1],
                    tp: tp,
                    ident: ident,
                }
            },
            5 => {
                let mut samples: [i16; 16] = unsafe { mem::uninitialized() };
                ::byteorder::LittleEndian::read_i16_into(&packet[4..], &mut samples);
                Command::PCM{samples: samples}
            },
            _ => {
                let mut data: [u8; Command::SIZE] = unsafe { mem::uninitialized() };
                data.copy_from_slice(packet);
                Command::Unknown{data: data}
            }
        }
    }
}

impl<'a> From<&'a Command> for [u8; Command::SIZE] {
    fn from(cmd: &'a Command) -> [u8; Command::SIZE] {
        let mut ret: [u8; Command::SIZE] = [0u8; Command::SIZE];

        match *cmd {
            Command::KeepAlive => NetworkEndian::write_u32(&mut ret[..4], 0),
            Command::Ping{data} => {
                NetworkEndian::write_u32(&mut ret[..4], 1);
                (&mut ret[4..]).copy_from_slice(&data);
            },
            Command::Quit => NetworkEndian::write_u32(&mut ret[..4], 2),
            Command::Play{sec, usec, freq, amp, voice} => {
                NetworkEndian::write_u32_into(&[3u32, sec, usec, freq], &mut ret[..16]);
                NetworkEndian::write_f32(&mut ret[16..20], amp);
                NetworkEndian::write_u32(&mut ret[20..24], voice);
            },
            Command::Caps{voices, tp, ident} => {
                NetworkEndian::write_u32_into(&[4u32, voices], &mut ret[..8]);
                (&mut ret[8..12]).copy_from_slice(&tp);
                (&mut ret[12..]).copy_from_slice(&ident);
            },
            Command::PCM{samples} => {
                NetworkEndian::write_u32(&mut ret[..4], 5);
                NetworkEndian::write_i16_into(&samples, &mut ret[4..]);
            },
            Command::Unknown{data} => {
                ret.copy_from_slice(&data);
            },
        };

        ret
    }
}
