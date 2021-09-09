use super::{Note, NoteStream, IV};

pub fn coalesce<'a, I: Iterator<Item = &'a NoteStream>>(stream_iter: I) -> NoteStream {
    let mut output = NoteStream::new();

    for ns in stream_iter {
        output.extend(ns.iter().cloned());
    }

    output
}

pub struct SchedParams {
    pub epsilon: f32,
}

impl Default for SchedParams {
    fn default() -> SchedParams {
        SchedParams { epsilon: 0.0 }
    }
}

pub fn schedule<'a, 'b: 'a, I: Iterator<Item = &'a Note>, F: FnMut(&'a Note) -> Option<&'b str>>(
    notes: I,
    mut classifier: F,
    params: &SchedParams,
) -> IV {
    let mut output: IV = Default::default();

    for note in notes {
        let grp_name = classifier(note);
        let grp = if let Some(name) = grp_name {
            if !output.groups.contains_key(name) {
                output.groups.insert(name.into(), Vec::new());
            }
            output.groups.get_mut(name).unwrap()
        } else {
            &mut output.default_group
        };

        let mut found: Option<usize> = None;
        for (idx, ns) in grp.iter().enumerate() {
            if ns.len() > 0 {
                let nt = &ns[ns.len() - 1];
                if note.time.0 < nt.time.0 + nt.dur.0 + params.epsilon {
                    continue;
                }
            }
            found = Some(idx);
            break;
        }

        if let Some(nidx) = found {
            grp[nidx].push(note.clone());
        } else {
            grp.push(vec![note.clone()]);
        }
    }

    output
}
