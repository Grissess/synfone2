use std::io;
use std::collections::HashMap;
use std::borrow::Borrow;

use xml::reader;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;
use std::hash::Hash;
use std::cmp::Eq;
use std::str::FromStr;
use std::fmt::Display;
use failure::Error;
use super::*;

struct AttrMapping(HashMap<String, String>);

impl AttrMapping {
    pub fn make(attrs: Vec<OwnedAttribute>) -> AttrMapping {
        let mut output = HashMap::new();

        for attr in attrs {
            output.insert(attr.name.local_name.clone(), attr.value.clone());
        }

        AttrMapping(output)
    }

    pub fn get_str<'a, 'b, 'c: 'a, Q: Hash+Eq+Display+?Sized>(&'a self, key: &'b Q, default: &'c str) -> &'a str where String: Borrow<Q> {
        self.0.get(key).map(|x| &**x).unwrap_or(default)
    }

    pub fn req<V: FromStr, Q: Hash+Eq+Display+?Sized>(&self, key: &Q) -> Result<V, Error> where String: Borrow<Q>, V::Err: failure::Fail {
        match self.0.get(key){ 
            Some(x) => Ok(x.parse()?),
            None => bail!("{} not found in attrs", key)
        }
    }

    pub fn req_midi_pitch<Q: Hash+Eq+Display+?Sized>(&self, key: &Q) -> Result<Pitch, Error> where String: Borrow<Q> {
        Ok(Pitch::MIDI(self.req::<f32, Q>(key)?))
    }
}

fn parse_note(ev: XmlEvent, into: &mut Vec<Note>) -> Result<bool, Error> {
    match ev {
        XmlEvent::StartElement{name, attributes, ..} => {
            if name.local_name.as_ref() != "note" { bail!("malformed iv: non-note attr in note stream"); }
            let attrs = AttrMapping::make(attributes);
            into.push(Note {
                time: attrs.req("time")?,
                ampl: attrs.req("ampl")?,
                dur: attrs.req("dur")?,
                pitch: attrs.req_midi_pitch("pitch")?,
                start_tick: None,
                dur_ticks: None
            });
            Ok(false)
        },
        _ => Ok(true)
    }
}

pub fn read<R: io::Read>(source: R) -> Result<IV, Error> {
    let mut output: IV = Default::default();
    let mut event_reader = EventReader::new(source);

    #[derive(Debug)]
    enum ReadState<'a> {
        Idle,
        InStreams,
        InBPMs,
        InNoteStream(&'a mut NoteStream),
        InAuxStream(&'a mut AuxStream),
    }

    let mut state = ReadState::Idle;

    loop {
        match event_reader.next()? {
            XmlEvent::StartElement{name, attributes, ..} => {
                let attrmap = AttrMapping::make(attributes);

                match name.local_name.as_ref() {
                    "bpms" => { }
                    "streams" => {
                        match attrmap.get_str("type", "") {
                            "ns" => {
                                let mut notes = Vec::new();

                                loop {
                                    if !parse_note(event_reader.next()?, &mut notes)? { break; }
                                }

                            },
                            _ => unimplemented!()
                        }
                    },
                    _ => unimplemented!()
                }
            }
            XmlEvent::EndElement{name} => match (name.local_name.as_ref(), &state) {
                ("bpms", _) => { state = ReadState::Idle; },
                ("streams", _) => { state = ReadState::Idle; },
                _ => (),
            },
            _ => (),
        }
    }
        

    Ok(output)
}
