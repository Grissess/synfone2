use std::io;
use std::io::*;
use std::net::*;
use std::sync::*;
use std::collections::VecDeque;

extern crate synfone;
extern crate portaudio;
use portaudio as pa;
use synfone::*;
use synfone::synth::*;
use synfone::lang::*;
use synfone::proto::*;
use synfone::client::*;

const GEN: &'static str = "mul(saw(param('v_freq', 500)), ifelse(rel(param('v_frame'), '<', param('v_deadline')), param('v_amp'), 0.0))";

fn main() {
    let env = Environment::default();

    let mut gens = Vec::new();
    for _i in 0..25 {
        let gen = Parser::new(Tokenizer::new(GEN.chars())).expect("Failed to get first token").parse().expect("Failed to compile generator");
        gens.push(gen);
    }
    let sock = UdpSocket::bind("0.0.0.0:13676").expect("Failed to bind socket");

    let mut client = Arc::new(Mutex::new(Client::new(sock.try_clone().expect("Failed to clone socket"), gens, env.clone()).expect("Failed to create client")));

    let pa_inst = pa::PortAudio::new().expect("Failed to create PortAudio interface");
    let settings = pa_inst.default_output_stream_settings(1, env.sample_rate as f64, env.default_buffer_size as u32).expect("Failed to instantiate stream settings");
    let mut stream;
    {
        let client = client.clone();
        let mut ring: VecDeque<Sample> = VecDeque::new();
        ring.reserve_exact(2 * env.default_buffer_size);
        stream = pa_inst.open_non_blocking_stream(settings, move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
            while frames > ring.len() {
                let mut cli = client.lock().unwrap();
                cli.next_frames();
                ring.append(&mut cli.buffer().iter().map(|&x| x).collect());
            }
            let samps = ring.drain(..frames).collect::<Vec<f32>>();
            buffer.copy_from_slice(&samps);
            pa::Continue
        }).expect("Failed to create stream");
    }


    eprintln!("Starting.");

    stream.start().expect("Failed to start stream");

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

    eprintln!("Exiting.");
}
