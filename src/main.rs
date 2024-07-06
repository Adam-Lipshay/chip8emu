mod processor;
mod font;

use std::env::{self};

use sdl2::pixels::Color;
use sdl2::event::Event;
use std::time::Duration;
use sdl2::audio::{AudioCallback, AudioSpecDesired};
use std::f32::consts::PI;

const SLEEP_TIME: u64 = 2;

struct SineWave {
    phase: f32,
    frequency: f32,
    volume: f32,
}

impl AudioCallback for SineWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        const SAMPLE_RATE: f32 = 44100.0;
        let angular_frequency = 2.0 * PI * self.frequency / SAMPLE_RATE;
        
        for x in out.iter_mut() {
            *x = self.volume * (self.phase * angular_frequency).sin();
            self.phase += 1.0;
        }
    }
}

pub fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Please input a path to a ROM file.");
    }
    let rom = std::fs::read(&args[1]).expect("Unable to load ROM");
    

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("CHIP-8 Emu", 1280, 640)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    
    canvas.set_scale(20.0, 20.0).expect("Failed to set scale");
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let audio_subsystem = sdl_context.audio().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    let device = audio_subsystem.open_playback(None, &desired_spec, |_spec| {
        SineWave {
            phase: 0.0,
            frequency: 440.0,
            volume: 0.5,
        }
    }).unwrap();

    let mut cpu = processor::CPU::new(&mut canvas, device, sdl_context);
    cpu.load(rom);

    'running: loop {

        for event in cpu.event.poll_iter() {
            if let Event::Quit { .. } = event {
                break 'running;
            };
        }
        
        cpu.run();

        ::std::thread::sleep(Duration::from_millis(SLEEP_TIME));
    }
}
