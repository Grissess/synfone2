pub type Sample = f32;

#[derive(Debug, Clone)]
pub enum Pitch {
    Freq(f32),
    MIDI(f32),
}

impl Pitch {
    pub fn to_midi(&self) -> f32 {
        match *self {
            Pitch::MIDI(x) => x,
            Pitch::Freq(x) => 12.0 * (x / 440.0).log2() + 69.0,
        }
    }
    pub fn to_midi_pitch(&self) -> Pitch {
        Pitch::MIDI(self.to_midi())
    }

    pub fn to_freq(&self) -> f32 {
        match *self {
            Pitch::MIDI(x) => 440.0 * (2.0f32).powf((x - 69.0) / 12.0),
            Pitch::Freq(x) => x,
        }
    }
    pub fn to_freq_pitch(&self) -> Pitch {
        Pitch::Freq(self.to_freq())
    }
}
