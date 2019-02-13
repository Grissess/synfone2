use std::{io, env, thread, iter, time, mem};
use std::io::*;
use std::fs::*;
use std::net::*;
use std::sync::*;
use std::collections::VecDeque;

extern crate synfone;
extern crate portaudio;
// #[macro_use]
// extern crate glium;
// use glium::{glutin, Surface};
// use glium::index::PrimitiveType;
// extern crate palette;
// use palette::IntoColor;
use portaudio as pa;
use synfone::*;
use synfone::synth::*;
use synfone::lang::*;
use synfone::proto::*;
use synfone::client::*;

const GFX: bool = false;

fn main() {
    let env = Environment::default();

    let mut genfile = File::open(env::args_os().nth(1).expect("Need first argument to be a file with a generator vector")).expect("Failed to open file");
    let mut genstr = String::new();
    genfile.read_to_string(&mut genstr);

    let gens = Parser::new(Tokenizer::new(genstr.chars()), env.clone()).expect("Failed to get first token").parse_gen_vec().expect("Failed to compile generators");
    let sock = UdpSocket::bind("0.0.0.0:13676").expect("Failed to bind socket");

    eprintln!("Parsed {} generator definitions", gens.len());

    let mut client = Arc::new(Mutex::new(Client::new(sock.try_clone().expect("Failed to clone socket"), gens, env.clone()).expect("Failed to create client")));
    let mut last_buffer = Arc::new(Mutex::new(<VecDeque<Sample>>::with_capacity(env.default_buffer_size * 9)));
    let last_buffer_lim = env.default_buffer_size * 8;
    last_buffer.lock().expect("Failed to init shared buffer").append(&mut iter::repeat(0.0f32).take(last_buffer_lim).collect());

    let pa_inst = pa::PortAudio::new().expect("Failed to create PortAudio interface");
    let settings = pa_inst.default_output_stream_settings(1, env.sample_rate as f64, env.default_buffer_size as u32).expect("Failed to instantiate stream settings");
    let mut stream;
    {
        let client = client.clone();
        let last_buffer = last_buffer.clone();
        let mut ring: VecDeque<Sample> = VecDeque::new();
        ring.reserve_exact(2 * env.default_buffer_size);
        stream = pa_inst.open_non_blocking_stream(settings, move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            while frames > ring.len() {
                let mut cli = client.lock().unwrap();
                cli.next_frames();
                {
                    let mut buf = last_buffer.lock().expect("Failed to acquire shared buffer in audio callback");
                    buf.append(&mut cli.buffer().samples.iter().map(|&x| x).collect());
                    let len = buf.len();
                    if len > last_buffer_lim {
                        buf.drain(..(len - last_buffer_lim));
                    }
                }
                ring.append(&mut cli.buffer().iter().map(|&x| x).collect());
            }
            let samps = ring.drain(..frames).collect::<Vec<f32>>();
            buffer.copy_from_slice(&samps);
            pa::Continue
        }).expect("Failed to create stream");
    }


    eprintln!("Starting.");

    stream.start().expect("Failed to start stream");

    eprintln!("Audio stream started.");

    let net_thread = {
        let client = client.clone();
        let net_thread = thread::spawn(move || {
            let mut buffer: [u8; Command::SIZE] = [0u8; Command::SIZE];
            loop {
                let (bytes, sender) = sock.recv_from(&mut buffer).unwrap();
                if bytes < Command::SIZE {
                    continue;
                }

                let cmd = Command::from(&buffer);
                {
                    let mut cli = client.lock().unwrap();
                    if !cli.handle_command(cmd, sender) {
                        break;
                    }
                }
            }
        });
        net_thread
    };

    eprintln!("Network thread started.");
    
    net_thread.join().expect("Network thread panicked");

    eprintln!("Exiting.");
}
