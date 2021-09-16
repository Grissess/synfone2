use std::io;
use std::convert::TryFrom;
use std::str::{from_utf8, Utf8Error};
use std::borrow::Borrow;

use crate::seq::{IV, Version, VersionDecodeError, IVMeta, BPMTable};

use quick_xml::events::{Event, BytesStart};


struct State<'s, B: io::BufRead> {
    iv: &'s mut IV,
    rdr: &'s mut quick_xml::Reader<B>,
}

#[derive(Debug)]
pub enum Error {
    QXML(quick_xml::Error),
    VersionDecodeError,
    UTF8(Utf8Error),
    Unexpected { scope: Scope, event: String },
}

#[derive(Debug, Clone, Copy, Hash)]
pub enum Scope {
    TopLevel,
}

impl From<quick_xml::Error> for Error {
    fn from(t: quick_xml::Error) -> Self {
        Error::QXML(t)
    }
}

impl From<VersionDecodeError> for Error {
    fn from(v: VersionDecodeError) -> Self {
        Error::VersionDecodeError
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::UTF8(e)
    }
}

pub fn read<R: io::Read>(source: R) -> Result<IV, Error> {
    let mut output: IV = Default::default();
    let mut reader = quick_xml::Reader::from_reader(
        io::BufReader::new(source)
    );
    let mut state = State {
        iv: &mut output,
        rdr: &mut reader,
    };
    
    read_toplevel(&mut state)?;

    Ok(output)
}

const IV_NAME: &[u8] = b"iv";
const IV_VERSION: &[u8] = b"version";
const IV_SOURCE: &[u8] = b"src";
fn read_toplevel<'s, B: io::BufRead>(state: &mut State<'s, B>) -> Result<(), Error> {
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        match state.rdr.read_event(&mut buffer)? {
            Event::Decl(_) => (),  // Don't care
            Event::Start(bs) => {
                match_iv(state, bs, false);
                break;
            },
            Event::Empty(bs) => {
                match_iv(state, bs, true);
                break;
            },
            ev => return Err(Error::Unexpected {
                scope: Scope::TopLevel,
                event: format!("{:?}", ev),
            }),
        }
    }
    Ok(())
}

fn match_iv<'s, 'a, B: io::BufRead>(state: &mut State<'s, B>, bs: BytesStart<'a>, empty: bool) -> Result<(), Error> {
    if bs.name() != IV_NAME {
        return Err(Error::Unexpected {
            scope: Scope::TopLevel,
            event: format!("start tag: {:?}", bs.name()),
        });
    }
    for attr in bs.attributes() {
        let attr = attr?;
        match attr.key {
            key if key == IV_VERSION => {
                let value = attr.unescaped_value()?;
                state.iv.version =
                    Version::try_from(value.borrow())?;
            },
            key if key == IV_SOURCE => {
                state.iv.source =
                    Some(from_utf8(
                            attr.unescaped_value()?.borrow()
                    )?.into());
            },
            _ => (),
        }
    }
    if !empty { read_iv(state)?; }
    Ok(())
}

const META_NAME: &[u8] = b"meta";
const STREAMS_NAME: &[u8] = b"streams";
fn read_iv<'s, B: io::BufRead>(state: &mut State<'s, B>) -> Result<(), Error> {
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        match state.rdr.read_event(&mut buffer)? {
            Event::Start(bs) => {
                match_in_iv(state, bs, false);
            },
            Event::Empty(bs) => {
                match_in_iv(state, bs, true);
            },
            Event::End(be) => {
                if be.name() == IV_NAME {
                    break;
                }
            },
            _ => (),
        }
    }
    Ok(())
}

fn read_until<'s, B: io::BufRead>(state: &mut State<'s, B>, name: &[u8]) -> Result<(), Error> {
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        match state.rdr.read_event(&mut buffer)? {
            Event::End(be) => {
                if be.name() == name {
                    return Ok(());
                }
            }
            _ => (),
        }
    }
}

fn match_in_iv<'s, 'a, B: io::BufRead>(state: &mut State<'s, B>, bs: BytesStart<'a>, empty: bool) -> Result<(), Error> {
    match bs.name() {
        nm if nm == META_NAME => {
            if !empty { read_meta(state)?; }
        },
        nm if nm == STREAMS_NAME => {
            if !empty { read_streams(state)?; }
        },
        nm => {
            if !empty { read_until(state, nm.borrow())?; }
        }
    }
    Ok(())
}

fn read_meta<'s, B: io::BufRead>(state: &mut State<'s, B>) -> Result<(), Error> {
    todo!()
}

fn read_streams<'s, B: io::BufRead>(state: &mut State<'s, B>) -> Result<(), Error> {
    todo!()
}
