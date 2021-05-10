pub mod display;

use rand::{thread_rng, Rng};
use std::hint::unreachable_unchecked;

#[inline(always)]
pub fn get_opcode(mem: &[u8; 4096], addr: usize) -> u16 {
    (mem[addr] as u16) << 8 | mem[addr + 1] as u16
}

pub struct Chip8State {
    pub mem: Box<[u8; 4096]>,
    pub regs: [u8; 16],
    pub stack: [usize; 16],
    pub pc: usize, // Program counter
    pub i: usize,  // I-register
    pub sp: usize, // Stack pointer
    pub sound_timer: u8,
    pub delay_timer: u8
}

pub struct Chip8 {
    mem: Box<[u8; 4096]>,
    regs: [u8; 16],
    stack: [usize; 16],
    pc: usize, // Program counter
    i: usize,  // I-register
    sp: usize, // Stack pointer
    sound_timer: u8,
    delay_timer: u8,
    pub display: display::Display,
    key_wait_handler: &'static dyn Fn() -> u8,
    key_state_handler: &'static dyn Fn(u8) -> bool,
}

impl Chip8 {
    pub fn new<T, G>(
        key_wait_handler: Option<&'static (dyn Fn() -> u8 + 'static)>,
        key_state_handler: Option<&'static (dyn Fn(u8) -> bool + 'static)>
    ) -> Chip8 
    {
        let key_wait_handler = key_wait_handler.unwrap_or(&|| 0);
        let key_state_handler = key_state_handler.unwrap_or(&|k| false);
        Chip8 {
            mem: Box::new([0; 4096]),
            regs: [0; 16],
            stack: [0; 16],
            pc: 0,
            i: 0,
            sp: 0,
            sound_timer: 0,
            delay_timer: 0,
            display: display::Display::new(),
            key_wait_handler,
            key_state_handler,
        }
    }

    pub fn to_state(&self) -> Chip8State {
        Chip8State {
            mem: self.mem.clone(),
            regs: self.regs,
            stack: self.stack,
            pc: self.pc,
            i: self.i,
            sp: self.sp,
            sound_timer: self.sound_timer,
            delay_timer: self.delay_timer
        }
    }

    pub fn set_handlers(
        &mut self, 
        key_wait_handler: &'static (dyn Fn() -> u8 + 'static),
        key_state_handler: &'static (dyn std::ops::Fn(u8) -> bool + 'static)
    ) {
        self.key_wait_handler = key_wait_handler;
        self.key_state_handler = key_state_handler
    }

    pub fn get_regs(&self) -> [u8; 16] {
        self.regs
    }

    pub fn get_i(&self) -> usize {
        self.i
    }

    pub fn get_sound_timer(&self) -> u8 {
        self.sound_timer
    }

    pub fn get_delay_timer(&self) -> u8 {
        self.delay_timer
    }

    pub fn is_sound_playing(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn get_memory(&self, addr: usize) -> u8 {
        self.mem[addr]
    }

    pub fn get_opcode(&self, addr: usize) -> u16 {
        get_opcode(&self.mem, addr)
    }

    pub fn get_pc(&self) -> usize {
        self.pc
    }

    /// The at parameter should almost always be 0x200. It's here for compatability with ETI 660 programs (starting with 0x600).
    /// Panics if at is less than 240, where the default font lies.
    pub fn load(&mut self, at: usize, program: &[u8], font: Option<[u8; 240]>) {
        if at < 240 {
            panic!("First 240 bytes are the default font, so can't load here.")
        }
        for (i, b) in program.iter().enumerate() {
            self.mem[at + i] = *b;
        }
        let font = match font {
            None => display::DEFAULT_FONT,
            Some(f) => f,
        };
        for (i, c) in font.iter().enumerate() {
            self.mem[i] = *c
        }
        self.pc = at;
    }

    fn stack_push(&mut self, v: usize) {
        self.sp += 1;
        self.stack[self.sp] = v
    }

    fn stack_pop(&mut self) -> usize {
        self.sp -= 1;
        self.stack[self.sp + 1]
    }

    pub fn timers_tick(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1
        }
    }

    pub fn cpu_tick(&mut self) -> Result<(), &'static str> {
        self.run_opcode((self.mem[self.pc] as u16) << 8 | self.mem[self.pc + 1] as u16)
    }

    fn run_opcode(&mut self, opcode: u16) -> Result<(), &'static str> {
        // if self.debug {eprintln!("{:04x?}:{:04x?}", self.pc, opcode)};
        let x = || ((opcode & 0x0F00) >> 8) as usize;
        let y = || ((opcode & 0x00F0) >> 4) as usize;
        let n = || opcode & 0x000F;
        let kk = || opcode & 0x00FF;
        let nnn = || opcode & 0x0FFF;

        match (opcode & 0xF000) >> 12 {
            // Instructions that mess with the program counter are returning after that so it wouldn't be incremented after.
            0x0 => match opcode {
                0x00C0..=0x00CF => self.display.scroll_down(n() as u32),
                0x00E0 => self.display.clear(),
                0x00EE => self.pc = self.stack_pop(),
                0x00FB => self.display.scroll_side(4),
                0x00FC => self.display.scroll_side(-4),
                0x00FD => return Err("Program exited"),
                0x00FE => self.display.low_res_mode(),
                0x00FF => self.display.hi_res_mode(),
                _ => {}
            },
            0x1 => {
                self.pc = nnn() as usize;
                return Ok(());
            }
            0x2 => {
                self.stack_push(self.pc);
                self.pc = nnn() as usize;
                return Ok(());
            }
            0x3 => {
                if self.regs[x()] == kk() as u8 {
                    self.pc += 4;
                    return Ok(());
                }
            }
            0x4 => {
                if self.regs[x()] != kk() as u8 {
                    self.pc += 4;
                    return Ok(());
                }
            }
            0x5 => {
                if self.regs[x()] == self.regs[y()] as u8 {
                    self.pc += 4;
                    return Ok(());
                }
            }
            0x6 => self.regs[x()] = kk() as u8,
            0x7 => {
                let (v, _) = self.regs[x()].overflowing_add(kk() as u8);
                self.regs[x()] = v
            }
            0x8 => match opcode & 0x000F {
                0x0 => self.regs[x()] = self.regs[y()],
                0x1 => self.regs[x()] |= self.regs[y()],
                0x2 => self.regs[x()] &= self.regs[y()],
                0x3 => self.regs[x()] ^= self.regs[y()],
                0x4 => {
                    let (v, carry) = self.regs[x()].overflowing_add(self.regs[y()]);
                    self.regs[0xF] = carry as u8;
                    self.regs[x()] = v;
                }
                0x5 => {
                    let (v, borrow) = self.regs[x()].overflowing_sub(self.regs[y()]);
                    self.regs[0xF] = !borrow as u8;
                    self.regs[x()] = v;
                }
                0x6 => {
                    let (v, carry) = self.regs[y()].overflowing_shr(1);
                    self.regs[x()] = v;
                    self.regs[0xF] = carry as u8;
                }
                0x7 => {
                    let (v, borrow) = self.regs[y()].overflowing_add(self.regs[x()]);
                    self.regs[0xF] = !borrow as u8;
                    self.regs[x()] = v;
                }
                0xE => {
                    let (v, carry) = self.regs[y()].overflowing_shl(1);
                    self.regs[x()] = v;
                    self.regs[0xF] = carry as u8;
                }
                _ => return Err("Invalid opcode"),
            },
            0x9 => {
                if self.regs[x()] != self.regs[y()] as u8 {
                    self.pc += 4;
                    return Ok(());
                }
            }
            0xA => self.i = nnn() as usize,
            0xB => {
                self.pc = nnn() as usize + self.regs[0] as usize;
                return Ok(());
            }
            0xC => self.regs[x()] = thread_rng().gen::<u8>() & kk() as u8,
            0xD => {
                let mut erased = false;
                if n() == 0 && self.display.hi_res() {
                    for j in 0..16 {
                        erased |= self.display.write(
                            self.mem[self.i + j * 2],
                            self.regs[x()] as usize,
                            self.regs[y()] as usize + j as usize,
                        );
                        erased |= self.display.write(
                            self.mem[self.i + j * 2 + 1],
                            self.regs[x()] as usize + 8,
                            self.regs[y()] as usize + j as usize,
                        )
                    }
                } else {
                    for j in 0..n() {
                        erased |= self.display.write(
                            self.mem[self.i + j as usize],
                            self.regs[x()] as usize,
                            self.regs[y()] as usize + j as usize,
                        )
                    }
                }
                self.regs[0xF] = erased as u8
            }
            0xE => match opcode & 0x00FF {
                0x9E => {
                    if (self.key_state_handler)(self.regs[x()]) {
                        self.pc += 4;
                        return Ok(());
                    }
                }
                0xA1 => {
                    if !(self.key_state_handler)(self.regs[x()]) {
                        self.pc += 4;
                        return Ok(());
                    }
                }
                _ => return Err("Invalid opcode"),
            },
            0xF => match opcode & 0x00FF {
                0x07 => self.regs[x()] = self.delay_timer,
                0x0A => self.regs[x()] = (self.key_wait_handler)(),
                0x15 => self.delay_timer = self.regs[x()],
                0x18 => self.sound_timer = self.regs[x()],
                0x1E => {
                    let (v, _) = self.i.overflowing_add(self.regs[x()] as usize);
                    self.i = v
                }
                0x29 => self.i = self.regs[x()] as usize * 5,
                0x30 => self.i = self.regs[x()] as usize * 10 + 40,
                0x33 => {
                    let vx = self.regs[x()];
                    self.mem[self.i] = vx / 100;
                    self.mem[self.i + 1] = vx % 100 / 10;
                    self.mem[self.i + 2] = vx % 100 % 10;
                }
                0x55 => {
                    for j in 0..=x() {
                        self.mem[self.i + j] = self.regs[j]
                    }
                }
                0x65 => {
                    for j in 0..=x() {
                        self.regs[j] = self.mem[self.i + j]
                    }
                }
                _ => return Err("Invalid opcode"),
            },
            _ => unsafe { unreachable_unchecked() },
        }
        self.pc += 2;
        Ok(())
    }
}
