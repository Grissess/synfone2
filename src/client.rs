use std::net::{UdpSocket};
use std::{io, mem};

use synth::*;
use proto::*;

pub struct Voice {
    pub gen: GenBox,
    pub params: Parameters,
}

pub struct Client {
    pub socket: UdpSocket,
    pub voices: Vec<Voice>,
    pub env: Environment,
    pub frames: usize,
    pub buf: SampleBuffer,
}

macro_rules! dprintln {
    ( $( $x:expr ),* ) => { eprintln!( $( $x ),* ) }
}

impl Client {
    pub fn new(socket: UdpSocket, gens: Vec<GenBox>, env: Environment) -> io::Result<Client> {
        socket.set_nonblocking(true)?;
        let buf = SampleBuffer::new(env.default_buffer_size);
        let voices = gens.into_iter().map(|g| Voice { gen: g, params: Parameters { env: env.clone(), ..Default::default() } }).collect();
        Ok(Client {
            socket: socket,
            voices: voices,
            env: env,
            frames: 0,
            buf: buf,
        })
    }

    pub fn pump(&mut self, out_buffer: &mut Vec<u8>) -> bool {
        if self.voices.len() == 0 {
            return false;
        }

        let mut buffer: [u8; Command::SIZE] = unsafe { mem::uninitialized() };

        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((bytes, sender)) => {
                    if bytes != Command::SIZE {
                        dprintln!("Dropping packet: wrong number of bytes (got {}, expected {})", bytes, Command::SIZE);
                        continue;
                    }

                    let cmd = Command::from(&buffer);
                    dprintln!("Packet {:?} from {:?}", (&buffer as &[u8]), sender);
                    match cmd {
                        Command::KeepAlive => {},
                        Command::Ping{..} => {
                            self.socket.send_to(&buffer, sender);
                        },
                        Command::Quit => {
                            return false;
                        },
                        Command::Play{voice, freq, amp, ..} => {
                            if (voice as usize) >= self.voices.len() {
                                dprintln!("Dropping packet: tried to send to voice {} >= number of voices {}", voice, self.voices.len());
                                continue;
                            }
                            let dur = cmd.duration().unwrap();
                            let frac_secs = (dur.as_secs() as f32) + (dur.subsec_nanos() as f32) / 1.0e9;
                            let frames = frac_secs * (self.env.sample_rate as f32);

                            dprintln!("Playing on voice {} freq {} amp {} from frame {} until frame {}", voice, freq, amp, self.frames, (self.frames as f32) + frames);

                            let mut vars = &mut self.voices[voice as usize].params.vars;
                            *vars.entry("v_deadline".to_string()).or_insert_with(Default::default) = (self.frames as f32) + frames;
                            *vars.entry("v_freq".to_string()).or_insert_with(Default::default) = freq as f32;
                            *vars.entry("v_amp".to_string()).or_insert_with(Default::default) = amp;
                        },
                        Command::Caps{..} => {
                            let reply = Command::Caps {
                                voices: self.voices.len() as u32,
                                tp: ['S' as u8, 'Y' as u8, 'N' as u8, 'F' as u8],
                                ident: [0u8; 24],
                            };
                            let mut reply_buffer: [u8; Command::SIZE] = [0u8; Command::SIZE];
                            reply.write_into(&mut reply_buffer);
                            self.socket.send_to(&reply_buffer, sender);
                        },
                        Command::PCM{..} => { /* TODO */ },
                        Command::Unknown{data} => {
                            dprintln!("Dropping packet: unknown data {:?}", (&data as &[u8]));
                        },
                    }
                },
                Err(err) => {
                    if err.kind() == io::ErrorKind::WouldBlock {
                        break;
                    }
                    return false;
                },
            }
        }

        for voice in self.voices.iter_mut() {
            *voice.params.vars.entry("v_frame".to_string()).or_insert_with(Default::default) = self.frames as f32;
        }

        let (first, next) = self.voices.split_at_mut(1);
        self.buf.update_from(first[0].gen.eval(&first[0].params));

        for voice in next {
            self.buf.sum_into(voice.gen.eval(&voice.params));
        }

        let current = out_buffer.len();
        out_buffer.reserve_exact(self.buf.size() - current);
        unsafe { out_buffer.set_len(self.buf.size()); }
        self.buf.write_bytes(out_buffer);
        self.frames += self.buf.len();
        return true;
    }
}
