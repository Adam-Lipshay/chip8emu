use sdl2::{render::Canvas, sys::Window};

use crate::font::{self, FONT};
use sdl2::pixels::Color;
use std::num::Wrapping;
use sdl2::rect::Point;


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
    display_array: [u64; 32],
}

impl CPU<'_> {
    pub fn new(canvas: &mut Canvas<sdl2::video::Window>) -> CPU {
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
            display_array: [0; 32],
        }
    }

    pub fn load(&mut self, data: Vec<u8>) {
        for i in 0..data.len() {
            if(0x200+i > 4095) {
                panic!("ROM too large!");
            }
            self.memory[0x200 + i] = *data.get(i).unwrap();
        }
    }

    pub fn update_timers(&mut self) {
        if(self.delay_timer > 0) {
            self.delay_timer -= 1;
        }
        if(self.sound_timer > 0) {
            self.sound_timer -= 1;
        }
    }

    pub fn start(&mut self) {
        self.pc = 0x200;
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
                _ => panic!("unknow 0 instruction {:#06x}", instruction)
            },
            0x1 => self.jump(instruction),
            0x6 => self.set_register(instruction),
            0x7 => self.add_register(instruction),
            0xA => self.set_index(instruction),
            0xD => self.display_sprite(instruction),

            _ => panic!("unknown instruction {:#06x}", instruction)
        }
    }

    fn get_pixel(&mut self, x: u32, y: u32) -> u8 {
        ((self.display_array[y as usize] & (1 << x)) >> x) as u8
    }

    fn set_pixel(&mut self, x: u32, y: u32, bit: u8) {
        if(bit == 1) {
            self.display_array[y as usize] = self.display_array[y as usize] | (1 << x);
        }
        if(bit == 0) {
            self.display_array[y as usize] = self.display_array[y as usize] & !(1 << x);
        }
    }

    fn update_display(&mut self) {
        self.display.set_draw_color(BACKGROUND_COLOR);
        self.display.clear();
        for row in 0..31 {
            for column in 0..63 {
                if((self.display_array[row] & (1 << column)) != 0) {
                    self.display.set_draw_color(DRAW_COLOR);
                    self.display.draw_point(Point::new(column, row as i32));
                }
            }
        }
        self.display.present();
    }

    pub fn run(&mut self) {
        let instruction = self.fetch();
        self.execute(instruction);
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

    fn set_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = Wrapping((instruction & 0x00FF) as u8);
    }

    fn add_register(&mut self, instruction: u16) {
        self.vx[((instruction & 0x0F00) >> 8) as usize] = self.vx[((instruction & 0x0F00) >> 8) as usize] + Wrapping((instruction & 0x00FF) as u8);
    }

    fn set_index(&mut self, instruction: u16) {
        self.index_register = instruction & 0x0FFF;
    }

    fn display_sprite(&mut self, instruction: u16) {
        let mut x = self.vx[((instruction & 0x0F00) >> 8) as usize].0 % 64;
        let mut y = self.vx[((instruction & 0x00F0) >> 4) as usize].0 % 32;
        self.vx[0xF as usize] = Wrapping(0);
        let n = instruction & 0x000F;

        for byte in 0..n {
            let row = self.memory[(self.index_register + byte) as usize];

            for bit in 0..8 {
                let cx = x + bit;
                let cy = y as u16 + byte;
                let cpixel = self.get_pixel(cx as u32, cy as u32);
                let row_pixel = ((row & (1 << 7 - bit)) >> 7 - bit);
                self.set_pixel(cx as u32, cy as u32, cpixel ^ row_pixel);

                if(cpixel == 1 && row_pixel == 1) {
                    self.vx[0xF as usize] = Wrapping(1);
                }
            }

        }

        self.update_display();
    }
}