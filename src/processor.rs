use sdl2::{audio::AudioDevice, render::Canvas};

use crate::{font::{self, FONT}, SineWave};
use sdl2::Sdl;
use sdl2::pixels::Color;
use std::num::Wrapping;
use sdl2::rect::Point;
use sdl2::keyboard::Keycode;
use sdl2::event::Event;
use rand::Rng;

#[derive(PartialEq)]
enum ShiftTypes {
    AsX,
}

const SHIFT_TYPE: ShiftTypes = ShiftTypes::AsX;

const BACKGROUND_COLOR: Color = Color::RGB(0, 0, 0);
const DRAW_COLOR: Color = Color::RGB(255, 255, 255);


pub struct CPU<'a> {
    memory: [u8; 4096],
    vx: [Wrapping<u8>; 16],
    stack_register: Vec<u16>,
    pc: u16,
    index_register: u16,
    delay_timer: u8,
    sound_timer: u8,
    display: &'a mut Canvas<sdl2::video::Window>,
    audio_device: AudioDevice<SineWave>,
    pub event: sdl2::EventPump,
    display_array: [u64; 32],
}

impl CPU<'_> {
    pub fn new(canvas: &mut Canvas<sdl2::video::Window>, device: AudioDevice<SineWave>, sdl_context: Sdl) -> CPU {
        let mut ram: [u8; 4096] = [0; 4096];
        for i in 0..font::FONT.len() {
            ram[0x050 + i] = FONT[i];
        }
        CPU {
            memory: ram,
            vx: [Wrapping(0); 16],
            stack_register: vec![],
            pc: 0x200,
            index_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            display: canvas,
            audio_device: device,
            event: sdl_context.event_pump().unwrap(),
            display_array: [0; 32],
        }
    }

    pub fn load(&mut self, data: Vec<u8>) {
        for i in 0..data.len() {
            if 0x200 + i > 4095 {
                panic!("ROM too large!");
            }
            self.memory[0x200 + i] = *data.get(i).unwrap();
        }
    }

    pub fn update_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    fn fetch(&mut self) -> u16 {
        let instruction = ((self.memory[self.pc as usize] as u16) << 8) | (self.memory[self.pc as usize + 1] as u16);
        self.pc += 2;
        instruction
    }
    fn execute(&mut self, instruction: u16) {
        match (instruction & 0xF000) >> 12 {
            0x0 => match instruction {
                0x00E0 => self.clear_screen(),
                0x00EE => self.stack_return(),
                _ => println!("unknow 0 instruction {:#06x}", instruction)
            },
            0x1 => self.jump(instruction),
            0x2 => self.call_subroutine(instruction),
            0x3 => self.jump_if_val_is_equal(instruction),
            0x4 => self.jump_if_val_not_equal(instruction),
            0x5 => self.jump_if_reg_is_equal(instruction),
            0x6 => self.set_as_value(instruction),
            0x7 => self.add_as_value(instruction),
            0x8 => match instruction & 0x000F {
                0x0000 => self.set_as_register(instruction),
                0x0001 => self.or_register(instruction),
                0x0002 => self.and_register(instruction),
                0x0003 => self.xor_register(instruction),
                0x0004 => self.add_as_register(instruction),
                0x0005 => self.sub_vx_xy(instruction),
                0x0006 => self.shift_register_right(instruction),
                0x0007 => self.sub_vy_xx(instruction),
                0x000E => self.shift_register_left(instruction),
                _ => println!("unknow 8 instruction {:#06x}", instruction)
            },
            0x9 => self.jump_if_reg_not_equal(instruction),
            0xA => self.set_index(instruction),
            0xC => self.random(instruction),
            0xD => self.display_sprite(instruction),
            0xF => match instruction & 0x00FF {
                0x0007 => self.get_delay_timer(instruction),
                0x0015 => self.set_delay_timer(instruction),
                0x0018 => self.set_sound_timer(instruction),
                0x001E => self.add_to_index(instruction),
                0x000A => self.get_key(instruction),
                0x0029 => self.get_font_character(instruction),
                0x0033 => self.binary_to_decimal(instruction),
                0x0055 => self.save_registers(instruction),
                0x0065 => self.load_registers(instruction),
                _ => println!("unknow F instruction {:#06x}", instruction)
            },

            _ => println!("unknown instruction {:#06x}", instruction)
        }
    }

    fn get_pixel(&mut self, x: u32, y: u32) -> u8 {
        ((self.display_array[y as usize] & (1 << x)) >> x) as u8
    }

    fn set_pixel(&mut self, x: u32, y: u32, bit: u8) {
        if bit == 1 {
            self.display_array[y as usize] = self.display_array[y as usize] | (1 << x);
        }
        if bit == 0  {
            self.display_array[y as usize] = self.display_array[y as usize] & !(1 << x);
        }
    }

    fn update_display(&mut self) {
        self.display.set_draw_color(BACKGROUND_COLOR);
        self.display.clear();
        for row in 0..31 {
            for column in 0..63 {
                if (self.display_array[row] & (1 << column)) != 0 {
                    self.display.set_draw_color(DRAW_COLOR);
                    self.display.draw_point(Point::new(column, row as i32)).expect("Failed to draw point");
                }
            }
        }
        self.display.present();
    }

    pub fn run(&mut self) {
        let instruction = self.fetch();
        self.execute(instruction);
        if self.sound_timer > 0 {
            self.audio_device.resume();
        } else {
            self.audio_device.pause();
        }
    }

    
}

impl CPU<'_> {
    fn  clear_screen (&mut self) {
        self.display_array = [0; 32];
        self.display.set_draw_color(BACKGROUND_COLOR);
        self.display.clear();
        self.display.present();
    }

    fn jump(&mut self, instruction: u16) {
        self.pc = instruction & 0x0FFF;
    }

    fn set_as_value(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping((instruction & 0x00FF) as u8);
    }

    fn add_as_value(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] + Wrapping((instruction & 0x00FF) as u8);
    }

    fn set_index(&mut self, instruction: u16) {
        self.index_register = instruction & 0x0FFF;
    }

    fn display_sprite(&mut self, instruction: u16) {
        let x = self.vx[((instruction & 0x0F00) >> 8) as usize].0 % 64;
        let y = self.vx[((instruction & 0x00F0) >> 4) as usize].0 % 32;
        self.vx[0xF as usize] = Wrapping(0);
        let n = instruction & 0x000F;

        for byte in 0..n {
            let row = self.memory[(self.index_register + byte) as usize];

            for bit in 0..8 {
                let cx = x + bit;
                if cx > 63 {
                    break;
                }
                let cy = y as u16 + byte;
                let cpixel = self.get_pixel(cx as u32, cy as u32);
                let row_pixel = (row & (1 << 7 - bit)) >> 7 - bit;
                self.set_pixel(cx as u32, cy as u32, cpixel ^ row_pixel);

                if cpixel == 1 && row_pixel == 1 {
                    self.vx[0xF as usize] = Wrapping(1);
                }
            }

        }

        self.update_display();
    }

    fn call_subroutine(&mut self, instruction: u16) {
        self.stack_register.push(self.pc);
        self.pc = instruction & (0x0FFF);
    }

    fn stack_return(&mut self) {
        self.pc = match self.stack_register.pop() {
            Some(address) => address,
            None => panic!("called return with empty stack"),
        };
    }

    fn jump_if_val_is_equal(&mut self, instruction: u16) {
        if self.vx[((instruction & 0x0F00) >> 8) as usize].0 == (instruction & 0x00FF) as u8 {
            self.pc += 2;
        }
    }

    fn jump_if_val_not_equal(&mut self, instruction: u16) {
        if self.vx[((instruction & 0x0F00) >> 8) as usize].0 != (instruction & 0x00FF) as u8 {
            self.pc += 2;
        }
    }

    fn jump_if_reg_is_equal(&mut self, instruction: u16) {
        if self.vx[((instruction & 0x0F00) >> 8) as usize] == self.vx[((instruction & 0x00F0) >> 4) as usize] {
            self.pc += 2;
        }
    }

    fn jump_if_reg_not_equal(&mut self, instruction: u16) {
        if self.vx[((instruction & 0x0F00) >> 8) as usize] != self.vx[((instruction & 0x00F0) >> 4) as usize] {
            self.pc += 2;
        }
    }

    fn set_as_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x00F0) >> 4) as usize];
    }

    fn or_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] | self.vx[((instruction & 0x00F0) >> 4) as usize];
    }

    fn and_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] & self.vx[((instruction & 0x00F0) >> 4) as usize];
    }

    fn xor_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] ^ self.vx[((instruction & 0x00F0) >> 4) as usize];
    }

    fn add_as_register(&mut self, instruction: u16) {
        let x = self.vx[((instruction & 0x0F00) >> 8) as usize];
        let y = self.vx[((instruction & 0x00F0) >> 4) as usize];
        self.vx[((instruction & 0x0F00) >> 8) as usize] = x + y;
        if (x.0 as u16 + y.0 as u16) > u8::MAX as u16 {
            self.vx[0xF as usize] = Wrapping(1);
        }
    }

    fn sub_vx_xy(&mut self, instruction: u16) {
        let x = self.vx[((instruction & 0x0F00) >> 8) as usize];
        let y = self.vx[((instruction & 0x00F0) >> 4) as usize];
        self.vx[((instruction & 0x0F00) >> 8) as usize] = x - y;
        if x > y {
            self.vx[0xF as usize] = Wrapping(1);
        } else {
            self.vx[0xF as usize] = Wrapping(0);
        }
    }

    fn sub_vy_xx(&mut self, instruction: u16) {
        let x = self.vx[((instruction & 0x0F00) >> 8) as usize];
        let y = self.vx[((instruction & 0x00F0) >> 4) as usize];
        self.vx[((instruction & 0x0F00) >> 8) as usize] = y - x;
        if y > x {
            self.vx[0xF as usize] = Wrapping(1);
        } else {
            self.vx[0xF as usize] = Wrapping(0);
        }
    }

    fn shift_register_right(&mut self, instruction: u16) {
        if SHIFT_TYPE == ShiftTypes::AsX {
            if self.vx[((instruction & 0x0F00) >> 8) as usize].0 & 1 == 1 {
                self.vx[0xF as usize] = Wrapping(1);
            } else {
                self.vx[0xF as usize] = Wrapping(0);
            }
            self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(self.vx[((instruction & 0x0F00) >> 8) as usize].0 >> 1);
        } else {
            if self.vx[((instruction & 0x00F0) >> 4) as usize].0 & 1 == 1 {
                self.vx[0xF as usize] = Wrapping(1);
            } else {
                self.vx[0xF as usize] = Wrapping(0);
            }
            self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(self.vx[((instruction & 0x00F0) >> 4) as usize].0 >> 1);
        }
    }

    fn shift_register_left(&mut self, instruction: u16) {
        if SHIFT_TYPE == ShiftTypes::AsX {
            if self.vx[((instruction & 0x0F00) >> 8) as usize].0 & (1 << 7) == 128 {
                self.vx[0xF as usize] = Wrapping(1);
            } else {
                self.vx[0xF as usize] = Wrapping(0);
            }
            self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(self.vx[((instruction & 0x0F00) >> 8) as usize].0 << 1);
        } else {
            if self.vx[((instruction & 0x00F0) >> 4) as usize].0 & (1 << 7) == 128 {
                self.vx[0xF as usize] = Wrapping(1);
            } else {
                self.vx[0xF as usize] = Wrapping(0);
            }
            self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(self.vx[((instruction & 0x00F0) >> 4) as usize].0 << 1);
        }
    }

    fn random(&mut self, instruction: u16) {
        let random_number: u8 = rand::thread_rng().gen();
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] & Wrapping(random_number);
    }

    fn get_delay_timer(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(self.delay_timer);
    }

    fn set_delay_timer(&mut self, instruction: u16) {
        self.delay_timer = self.vx[((instruction & 0x0F00) >> 8) as usize].0;
    }

    fn set_sound_timer(&mut self, instruction: u16) {
        self.sound_timer = self.vx[((instruction & 0x0F00) >> 8) as usize].0;
    }

    fn add_to_index(&mut self, instruction: u16) {
        self.index_register += self.vx[((instruction & 0x0F00) >> 8) as usize].0 as u16;
    }

    fn get_key(&mut self, instruction: u16) {
        'wait_key: loop {
            for event in self.event.poll_iter() {
                match event {
                    Event::KeyDown { keycode: Some(Keycode::NUM_1), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x1);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::NUM_2), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x2);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::NUM_3), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x3);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::NUM_4), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xC);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x4);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x5);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::E), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x6);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xD);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::A), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x7);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x8);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::D), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x9);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::F), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xE);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::Z), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xA);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::X), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0x0);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xB);
                        break 'wait_key
                    },
                    Event::KeyDown { keycode: Some(Keycode::V), .. } => {
                        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping(0xF);
                        break 'wait_key
                    },
                    _ => {}
                }
            }
        }
        
    }

    fn get_font_character(&mut self, instruction: u16) {
        self.index_register = (self.vx[((instruction & 0x0F00) >> 8) as usize].0 as u16)*5;
    }

    fn binary_to_decimal(&mut self, instruction: u16) {
        let number = self.vx[((instruction & 0x0F00) >> 8) as usize].0;
        
        self.memory[self.index_register as usize] = number / 100;
        self.memory[self.index_register as usize + 1] = (number / 10) % 10;
        self.memory[self.index_register as usize + 2] = number % 10;
        println!("{} {} {}", self.memory[self.index_register as usize], self.memory[self.index_register as usize + 1],self.memory[self.index_register as usize + 2])
    }

    fn save_registers(&mut self, instruction: u16) {
        let n = (instruction & 0x0F00) >> 8;
        for i in 0..n + 1 {
            self.memory[(self.index_register + i) as usize] = self.vx[i as usize].0;
        }
    }

    fn load_registers(&mut self, instruction: u16) {
        let n = (instruction & 0x0F00) >> 8;
        for i in 0..n + 1{
            self.vx[i as usize] = Wrapping(self.memory[(self.index_register + i) as usize]);
        }
    }
}