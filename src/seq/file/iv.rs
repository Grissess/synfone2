use std::io;
use std::collections::HashMap;
use std::borrow::Borrow;

use xml::reader;
use xml::reader::{EventReader, XmlEvent};
use xml::attribute::OwnedAttribute;

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

    pub fn get_str<'a, 'b, 'c: 'a, Q>(&'a self, key: &'b Q, default: &'c str) -> &'a str where String: Borrow<Q> {
        self.0.get(key).or(default)
    }
}

pub fn read<R: io::Read>(source: R) -> reader::Result<IV> {
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

    for ev in event_reader {
        match ev? {
            XmlEvent::StartElement{name, attributes, ..} => {
                let attrmap = AttrMapping::make(attributes);

                match (name.local_name.as_ref(), &state) {
                    ("bpms", &ReadState::Idle) => { state = ReadState::InBPMs; },
                    ("bpm", &ReadState::InBPMs) => {
                        let entry = BPMEntry {
                            abstick: 0,
                            bpm: BPM(0.0),
                            realtime: Some(Seconds(0.0)),
                        };
                    },
                    ("streams", &ReadState::Idle) => { state = ReadState::InStreams; },
                    _ => (),
                }
            },
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
