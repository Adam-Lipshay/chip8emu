mod processor;
mod font;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::{SystemTime, Duration};

const SLEEP_TIME: u64 = 2;


pub fn main() {
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

    let mut cpu = processor::CPU::new(&mut canvas);
    let rom = std::fs::read("ROMs/IBM.ch8").unwrap();
    cpu.load(rom);

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut now = SystemTime::now();
    'running: loop {

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        
        if now.elapsed().unwrap().as_millis() >= 1000 {
            cpu.update_timers();
            now = SystemTime::now();
        }

        cpu.run();

        ::std::thread::sleep(Duration::from_millis(SLEEP_TIME));
    }
}
