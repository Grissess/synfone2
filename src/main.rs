use std::{io, env, thread, iter, time, mem};
use std::io::*;
use std::fs::*;
use std::net::*;
use std::sync::*;
use std::collections::VecDeque;

extern crate synfone;
extern crate portaudio;
#[macro_use]
extern crate glium;
use glium::{glutin, Surface};
use glium::index::PrimitiveType;
extern crate palette;
use palette::IntoColor;
use portaudio as pa;
use synfone::*;
use synfone::synth::*;
use synfone::lang::*;
use synfone::proto::*;
use synfone::client::*;

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

    {
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
    }

    eprintln!("Network thread started.");

    //net_thread.join().expect("Network thread panicked");
    
    {
        let last_buffer = last_buffer.clone();

        let mut events_loop = glutin::EventsLoop::new();
        let window_bld = glutin::WindowBuilder::new().with_fullscreen(glutin::get_primary_monitor());
        let context_bld = glutin::ContextBuilder::new().with_gl_profile(glutin::GlProfile::Core);
        let display = glium::Display::new(window_bld, context_bld, &events_loop).expect("Failed to create display");

        eprintln!("OpenGL init, version {:?}", display.get_opengl_version());

        #[derive(Copy,Clone)]
        struct Vertex1dx {
            x: f32,
        }

        implement_vertex!(Vertex1dx, x);

        #[derive(Copy,Clone)]
        struct Vertex1dy {
            y: f32,
        }

        implement_vertex!(Vertex1dy, y);

        #[derive(Copy,Clone)]
        struct TexVertex2d {
            position: [f32; 2],
            uv: [f32; 2],
        }

        implement_vertex!(TexVertex2d, position, uv);

        let rect_vertices = glium::VertexBuffer::new(&display, &[
            TexVertex2d { position: [-1.0, -1.0], uv: [0.0, 0.0] },
            TexVertex2d { position: [1.0, -1.0], uv: [1.0, 0.0] },
            TexVertex2d { position: [1.0, 1.0], uv: [1.0, 1.0] },
            TexVertex2d { position: [-1.0, 1.0], uv: [0.0, 1.0] },
        ]).expect("Failed to create vertex buffer");

        let rect_indices = glium::IndexBuffer::new(&display, PrimitiveType::TrianglesList, &[0u16, 1, 2, 0, 2, 3]).expect("Failed to create index buffer");

        let graph_program = glium::program::Program::from_source(&display,
            "#version 430

            in vec2 position;
            in vec2 uv;

            out vec2 vUV;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                vUV = uv;
            }",
            "#version 430

            in vec2 vUV;

            out vec4 f_color;

            uniform sampler2D tex;

            void main() {
                f_color = texture(tex, vUV);
            }",
            None,
        ).expect("Failed to create graph program");

        let bg_program = glium::program::Program::from_source(&display,
            "#version 430

            in vec2 position;
            in vec2 uv;

            out vec2 vUV;

            void main() {
                gl_Position = vec4(position, 0.0, 1.0);
                vUV = uv;
            }",
            "#version 430

            layout (std430, binding = 1) buffer sbVoices {
                float voices[];
            };

            in vec2 vUV;

            out vec4 f_color;

            uniform float freq_low = 40.0, freq_high = 95.0;

            vec3 hsv2rgb(vec3 c)
            {
                vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
                vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
                return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
            }

            void main() {
                int n_voice = voices.length() / 2;
                int voice = clamp(int(vUV.x * n_voice), 0, n_voice - 1);
                float pitch = voices[voice * 2];
                float amp = voices[voice * 2 + 1];
                f_color = amp * vec4(hsv2rgb(vec3(clamp((pitch - freq_low) / (freq_high - freq_low), 0.0, 1.0), 1.0, amp)), 1.0);
            }",
            None,
        ).expect("Failed to create background program");

        let scope_program = glium::program::Program::from_source(&display,
            "#version 430

            in float x;
            in float y;

            void main() {
                gl_Position = vec4(x, y, 0.0, 1.0);
            }",
            "#version 430

            out vec4 f_color;

            void main() {
                f_color = vec4(0.0, 1.0, 0.0, 1.0);
            }",
            None,
        ).expect("Failed to create scope program");

        let params = glium::DrawParameters {
            blend: glium::draw_parameters::Blend::alpha_blending(),
            ..Default::default()
        };

        let (width, height) = display.get_framebuffer_dimensions();
        eprintln!("Allocating data with dimensionality {}, {}", width, height);
        let tex_data = glium::texture::RawImage2d::from_raw_rgba(iter::repeat(0u8).take((width * height * 4) as usize).collect(), (width, height));
        let tex_src = glium::texture::Texture2d::with_format(&display, tex_data, glium::texture::UncompressedFloatFormat::F32F32F32F32, glium::texture::MipmapsOption::NoMipmap).expect("Failed to create source texture");
        let tex_data = glium::texture::RawImage2d::from_raw_rgba(iter::repeat(0u8).take((width * height * 4) as usize).collect(), (width, height));
        let tex_dst = glium::texture::Texture2d::with_format(&display, tex_data, glium::texture::UncompressedFloatFormat::F32F32F32F32, glium::texture::MipmapsOption::NoMipmap).expect("Failed to create source texture");
        let mut fb_src = tex_src.as_surface();
        let mut fb_dst = tex_dst.as_surface();
        let bar_height = height / 128;

        let mut voice_ssbo = <glium::buffer::Buffer<[f32]>>::empty_unsized(&display,
            glium::buffer::BufferType::ShaderStorageBuffer,
            2 * mem::size_of::<f32>() * client.lock().unwrap().voices.len(),
            glium::buffer::BufferMode::Persistent,
        ).expect("Failed to create voice buffer");

        let mut sample_vbo_x = glium::VertexBuffer::new(&display,
            &(0..last_buffer_lim).into_iter().map(|i| Vertex1dx { x: 2.0 * ((i as f32) / ((last_buffer_lim - 1) as f32)) - 1.0 }).collect::<Vec<_>>()[..],
        ).expect("Failed to create sample X buffer");
        let mut sample_vbo_y = glium::VertexBuffer::persistent(&display,
            &(0..last_buffer_lim).into_iter().map(|_| Vertex1dy { y: 0.0 }).collect::<Vec<_>>()[..],
        ).expect("Failed to create sample Y buffer");

        let mut should_break = false;

        loop {
            events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, .. } => match event {
                        glutin::WindowEvent::Closed => should_break = true,
                        glutin::WindowEvent::KeyboardInput {
                            input: glutin::KeyboardInput {
                                virtual_keycode: Some(glutin::VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => should_break = true,
                        _ => (),
                    },
                    _ => (),
                }
            });

            if should_break { break; }

            fb_dst.fill(&fb_src, glium::uniforms::MagnifySamplerFilter::Nearest);

            fb_dst.clear_color(0.0, 0.0, 0.0, 0.0);
            fb_src.blit_color(
                &glium::Rect { left: 1, bottom: 0, width: (width - 1), height: height },
                &fb_dst,
                &glium::BlitTarget { left: 0, bottom: 0, width: (width - 1) as i32, height: height as i32 },
                glium::uniforms::MagnifySamplerFilter::Nearest,
            );

            let mut voice_params: Vec<(f32, f32)> = Vec::new();

            {
                let client = client.lock().unwrap();
                let len = client.voices.len();
                for (idx, voice) in client.voices.iter().enumerate() {
                    let freq = *voice.params.vars.get("v_freq").unwrap_or(&0.0);
                    let amp = *voice.params.vars.get("v_amp").unwrap_or(&0.0);
                    let deadline = *voice.params.vars.get("v_deadline").unwrap_or(&std::f32::INFINITY);
                    if deadline > (client.frames as f32) {
                        if freq > 0.0 && amp > 0.0 {
                            voice_params.push((Pitch::Freq(freq).to_midi(), amp));
                            let col = palette::Hsl::new(
                                palette::RgbHue::from_radians((idx as f64) * 2.0 * std::f64::consts::PI / (len as f64)),
                                1.0,
                                0.5 * (amp as f64),
                            ).into_rgb();
                            let bar_data = glium::texture::RawImage2d::from_raw_rgba(
                                [
                                    (col.red * 255.0) as u8,
                                    (col.green * 255.0) as u8,
                                    (col.blue * 255.0) as u8,
                                    (amp * 255.0) as u8,
                                ].into_iter().cycle().take((bar_height * 4) as usize).map(|&x| x).collect(),
                                (1, bar_height),
                            );
                            tex_dst.write(glium::Rect {
                                left: width - 1,
                                bottom: ((height as f32) * (Pitch::Freq(freq).to_midi() / 127.0)) as u32,
                                width: 1,
                                height: bar_height
                            }, bar_data);
                        } else {
                            voice_params.push((0.0, 0.0));
                        }
                    } else {
                        voice_params.push((0.0, 0.0));
                    }
                }
            }

            let flat_buffer: Vec<f32> = voice_params.into_iter().flat_map(|pair| vec![pair.0, pair.1]).collect();

            voice_ssbo.slice_mut(..flat_buffer.len()).expect("Failed to view into buffer slice").write(
                &flat_buffer[..],
            );

            sample_vbo_y.write(&last_buffer.lock().expect("Failed to read shared buffer in gfx").iter().map(|&y| Vertex1dy { y }).collect::<Vec<_>>()[..]);

            {
                let uniforms = uniform! {
                    tex: &tex_dst,
                    sbVoices: &voice_ssbo,
                };
                let mut target = display.draw();
                target.clear_color(0.0, 0.0, 0.0, 0.0);
                target.draw(&rect_vertices, &rect_indices, &bg_program, &uniforms, &params).expect("Failed to draw");
                target.draw(&rect_vertices, &rect_indices, &graph_program, &uniforms, &params).expect("Failed to draw");
                target.draw((&sample_vbo_x, &sample_vbo_y), glium::index::NoIndices(glium::index::PrimitiveType::LineStrip), &scope_program, &uniforms, &params).expect("Failed to draw");
                target.finish().expect("Failed to submit draw commands");
            }

            //display.swap_buffers().expect("Failed to swap buffers");
        }
    }

    eprintln!("Exiting.");
}
