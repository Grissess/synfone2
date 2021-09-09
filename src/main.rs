use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::net::*;
use std::sync::*;
use std::{env, ffi, iter, thread};

use cpal::SampleRate;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};

use synfone::client::*;
use synfone::lang::*;
use synfone::proto::*;
use synfone::synth::*;
use synfone::*;

fn main() -> Result<(), std::io::Error> {
    let cmd = env::args_os().nth(1).expect("Please pass a command as the first argument; use `help` as a command for more information.");
    let cmds = cmd.into_string().expect("Couldn't parse command");

    let new_args: Vec<ffi::OsString> = env::args_os().skip(1).collect();

    match &*cmds {
        "help" => eprintln!("TODO! Commands are help, client."),
        "client" => main_client(new_args)?,
        _ => eprintln!("Unknown command; `help` for help."),
    }
    Ok(())
}

fn main_client(args: Vec<ffi::OsString>) -> Result<(), std::io::Error> {
    let host = if let Ok(host) = cpal::host_from_id(cpal::HostId::Jack) {
        host
    } else {
        cpal::default_host()
    };
    let device = host.default_output_device().expect("no default host audio device!");
    let mut conf_ranges = device.supported_output_configs().expect("could not query audio device capabilities -- audio device disconnected?");
    let conf_range = conf_ranges.next().expect("audio device has no configurations!");
    let desired_sample_rate = conf_range.max_sample_rate().0;
    //let desired_sample_rate = u32::clamp(44100, conf_range.min_sample_rate().0, conf_range.max_sample_rate().0);
    let conf = conf_range.with_sample_rate(SampleRate(desired_sample_rate)).config();

    println!("playing at sample rate {}", conf.sample_rate.0);
    let env = Environment {
        sample_rate: conf.sample_rate.0 as f32,
        default_buffer_size: 64,
    };

    let mut genfile = File::open(
        args.iter()
            .nth(1)
            .expect("Need first argument to be a file with a generator vector"),
    )
    .expect("Failed to open file");
    let mut genstr = String::new();
    genfile.read_to_string(&mut genstr)?;

    let gens = Parser::new(Tokenizer::new(genstr.chars()), env.clone())
        .expect("Failed to get first token")
        .parse_gen_vec()
        .expect("Failed to compile generators");
    let sock = UdpSocket::bind("0.0.0.0:13676").expect("Failed to bind socket");

    eprintln!("Parsed {} generator definitions", gens.len());

    let client = Arc::new(Mutex::new(
        Client::new(
            sock.try_clone().expect("Failed to clone socket"),
            gens,
            env.clone(),
        )
        .expect("Failed to create client"),
    ));
    let last_buffer = Arc::new(Mutex::new(<VecDeque<Sample>>::with_capacity(
        env.default_buffer_size * 9,
    )));
    let last_buffer_lim = env.default_buffer_size * 8;
    last_buffer
        .lock()
        .expect("Failed to init shared buffer")
        .append(&mut iter::repeat(0.0f32).take(last_buffer_lim).collect());

    let stream;
    {
      let client = client.clone();
      let last_buffer = last_buffer.clone();
      let mut ring: VecDeque<Sample> = VecDeque::new();
      ring.reserve_exact(2 * env.default_buffer_size);
      stream = device.build_output_stream(
          &conf,
          move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
              let frames = data.len();
              while frames > ring.len() {
                  let mut cli = client.lock().unwrap();
                  cli.next_frames();
                  {
                      let mut buf = last_buffer
                          .lock()
                          .expect("Failed to acquire shared buffer in audio callback");
                      buf.append(&mut cli.buffer().samples.iter().map(|&x| x).collect());
                      let len = buf.len();
                      if len > last_buffer_lim {
                          buf.drain(..(len - last_buffer_lim));
                      }
                  }
                  ring.append(&mut cli.buffer().iter().map(|&x| x).collect());
              }
              let mut drain = ring.drain(..frames);
              let mut min = 1.0;
              let mut max = 0.0;
              for i in 0..frames {
                let frame = drain.next().unwrap();
                min = f32::min(min, frame);
                max = f32::max(min, frame);
                data[i] = frame;
              }
              println!("played {} frames, min {}, max {}", frames, min, max);
          },
          move |err| {
            println!("audio stream error: {}", err);
          }
      ).expect("could not create audio stream!");
    }

    eprintln!("Starting.");

    stream.play().expect("Failed to start stream");

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
                    let client = client.clone();
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

    Ok(())
}
