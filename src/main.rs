use bytes::BytesMut;
use num_enum::TryFromPrimitive;
use std::fs;
struct Emulator {
    ram: BytesMut,
    rom: BytesMut,
    header: BytesMut,
    rom_path: String,
    cpu: Cpu,
}

struct Cpu {
    flags: StatusFlags,
    program_counter: u16,
    stack_pointer: u16,
    halted: bool,
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum Opcode {
    HLT = 0x02,

    PHA = 0x48,
    PLA = 0x68,
    BPL = 0x10,
    BMI = 0x30,
    BNE = 0xD0,
    BEQ = 0xF0,

    LDY_Immediate = 0xA0,
    LDX_Immediate = 0xA2,

    LDA_ZeroPage = 0xA5,
    LDA_Immediate = 0xA9,
    LDA_Absolute = 0xAD,

    STA_ZeroPage = 0x85,
    STA_Absolute = 0x8D,

    STX_ZeroPage = 0x86,
    STX_Absolute = 0x8E,

    STY_ZeroPage = 0x84,
    STY_Absolute = 0x8C,
}

struct StatusFlags {
    carry_flag: bool,
    zero_flag: bool,
    interrupt_disable_flag: bool,
    overflow_flag: bool,
    negative_flag: bool,
}

impl Emulator {
    fn new(ram: u16, rom: u16, rom_path: &str) -> Self {
        Emulator {
            ram: {
                let mut buf = BytesMut::with_capacity(ram as usize);
                buf.resize(ram as usize, 0xFF);
                buf
            },
            rom: BytesMut::zeroed(rom as usize),
            header: BytesMut::zeroed(16),
            rom_path: rom_path.to_string(),
            cpu: Cpu {
                //interrupt_disable_flag is the only one that is enabled by default
                flags: StatusFlags {
                    carry_flag: false,
                    zero_flag: false,
                    interrupt_disable_flag: true,
                    overflow_flag: false,
                    negative_flag: false,
                },
                program_counter: 0,
                stack_pointer: 0,
                halted: false,
                reg_a: 0,
                reg_x: 0,
                reg_y: 0,
            },
        }
    }

    fn read(&self, address: u16) -> u8 {
        if address < 0x800 {
            self.ram[address as usize]
        } else if address >= 0x8000 {
            self.rom[(address - 0x8000) as usize]
        } else {
            0
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        if address < 0x800 {
            self.ram[address as usize] = value;
        } else if address >= 0x8000 {
            todo!();
        } else {
            todo!();
        }
    }

    fn push(&mut self, value: u8) {
        self.write(self.cpu.stack_pointer, value);
        self.cpu.stack_pointer -= 1;
    }

    fn pull(&mut self) -> u8 {
        self.cpu.stack_pointer += 1;
        let stack_byte = self.read(0x100 + self.cpu.stack_pointer);
        stack_byte
    }

    fn reset(mut self) {
        self.header = BytesMut::from(&fs::read(&self.rom_path).unwrap()[..]); // load rom file in memory 
        self.rom.copy_from_slice(&self.header[16..]); // extract the header
        let pcl = self.read(0xFFFC);
        let pch = self.read(0xFFFD);
        self.cpu.program_counter = ((pch as u16) * 0x100) + pcl as u16;
        self.run();
        //println!("a : 0x{:02x}\nx : 0x{:02x} \ny : 0x{:02x}", self.cpu.reg_a, self.cpu.reg_x, self.cpu.reg_y);
        println!("{:02x}", self.ram);
    }

    fn run(&mut self) {
        while !self.cpu.halted {
            self.emulate_cpu();
        }
    }

    fn emulate_cpu(&mut self) {
        use Opcode::*;
        let opcode = Opcode::try_from(self.read(self.cpu.program_counter)).unwrap();
        self.cpu.program_counter += 1;
        let mut cycles: usize = 0;
        match opcode {
            HLT => self.cpu.halted = true,
            LDY_Immediate => {
                self.cpu.reg_y = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                cycles = 2;
            }
            LDX_Immediate => {
                self.cpu.reg_x = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                cycles = 2;
            }
            LDA_Immediate => {
                self.cpu.reg_a = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                self.cpu.flags.zero_flag = self.cpu.reg_a == 0;
                self.cpu.flags.negative_flag = self.cpu.reg_a > 127;
                cycles = 2;
            }
            LDA_ZeroPage => {
                let operand = self.read(self.cpu.program_counter);
                self.cpu.reg_a = self.read(operand as u16);
                self.cpu.program_counter += 1;
                cycles = 3;
            }
            LDA_Absolute => {
                let operand_l = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                let operand_h = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                self.cpu.reg_a = self.read((operand_h as u16) * 256 + operand_l as u16);
                cycles = 4;
            }
            STY_ZeroPage => {
                let operand = self.read(self.cpu.program_counter);
                self.write(operand as u16, self.cpu.reg_y);
                self.cpu.program_counter += 1;
                cycles = 3;
            }
            STX_ZeroPage => {
                let operand = self.read(self.cpu.program_counter);
                self.write(operand as u16, self.cpu.reg_x);
                self.cpu.program_counter += 1;
                cycles = 3;
            }
            STA_ZeroPage => {
                let operand = self.read(self.cpu.program_counter);
                self.write(operand as u16, self.cpu.reg_a);
                self.cpu.program_counter += 1;
                cycles = 3;
            }
            STY_Absolute => {
                let operand_l = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                let operand_h = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                self.write((operand_h as u16) * 256 + operand_l as u16, self.cpu.reg_y);
                cycles = 4;
            }
            STX_Absolute => {
                let operand_l = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                let operand_h = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                self.write((operand_h as u16) * 256 + operand_l as u16, self.cpu.reg_x);
                cycles = 4;
            }
            STA_Absolute => {
                let operand_l = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                let operand_h = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                self.write((operand_h as u16) * 256 + operand_l as u16, self.cpu.reg_a);
                cycles = 4;
            }
            PHA => {
                self.push(self.cpu.reg_a);
                cycles = 3;
            }
            PLA => {
                self.cpu.reg_a = self.pull();
                self.cpu.flags.zero_flag = self.cpu.reg_a == 0;
                self.cpu.flags.negative_flag = self.cpu.reg_a >= 0x80;
                cycles = 4;
            }
            BNE => {
                let operand = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                if !self.cpu.flags.zero_flag {
                    let mut jump_counter = operand as i32;
                    if jump_counter > 127 {
                        jump_counter -= 256;
                    }
                    self.cpu.program_counter = self.cpu.program_counter + jump_counter as u16;
                    cycles = 3;
                } else {
                    cycles = 2;
                }
            }
            BEQ => {
                let operand = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                if self.cpu.flags.zero_flag {
                    let mut jump_counter = operand as i32;
                    if jump_counter > 127 {
                        jump_counter -= 256;
                    }
                    self.cpu.program_counter = self.cpu.program_counter + jump_counter as u16;
                    cycles = 3;
                } else {
                    cycles = 2;
                }
            }
            BPL => {
                let operand = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                if !self.cpu.flags.negative_flag {
                    let mut jump_counter = operand as i32;
                    if jump_counter > 127 {
                        jump_counter -= 256;
                    }
                    self.cpu.program_counter = self.cpu.program_counter + jump_counter as u16;
                    cycles = 3;
                } else {
                    cycles = 2;
                }
            }
            BMI => {
                let operand = self.read(self.cpu.program_counter);
                self.cpu.program_counter += 1;
                if self.cpu.flags.negative_flag {
                    let mut jump_counter = operand as i32;
                    if jump_counter > 127 {
                        jump_counter -= 256;
                    }
                    self.cpu.program_counter = self.cpu.program_counter + jump_counter as u16;
                    cycles = 3;
                } else {
                    cycles = 2;
                }
            }
            _ => todo!(),
        }
    }
}

fn main() {
    let emulator = Emulator::new(
        0x800,
        0x8000,
        "/home/este/rust/ntsc-nes/__PatreonRoms/3_Branches.nes",
    );
    emulator.reset();
}
