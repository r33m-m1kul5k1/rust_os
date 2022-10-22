const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;
const ADDRESS: usize = 0xb8000;
const PRINTABLE_ASCII: u8 = 0x20;
const PRINTABLE_ASCII_CMP: u8 = 0x7e;
const NOT_IN_ASCII_RANGE: u8 = 0xfe;

use core::fmt;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // By deriving the Copy, Clone, Debug, PartialEq, and Eq traits, we enable copy semantics for the type and make it printable and comparable.
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8); // ColorCode struct contains the full color byte, containing foreground and background color. 

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

// This struct will write to the screen
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.buffer.chars[row][col] = ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                };
                self.column_position += 1;
            }
        }
    }
    
    fn new_line(&mut self) {
        // only for the case that we need to print byte code, which is uneeded in our OS.
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                PRINTABLE_ASCII..=PRINTABLE_ASCII_CMP | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(NOT_IN_ASCII_RANGE),
            }

        }
    }

    pub fn set_color_code(&mut self, color_code: ColorCode) {
        self.color_code = color_code;
    }
    
}

impl fmt::Write for Writer {
    
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

  


pub fn print(string: &str, foreground: Color, background: Color) {
    let mut writer = Writer {
        column_position: 0,
        color_code: ColorCode::new(foreground, background),
        buffer: unsafe { &mut *(ADDRESS as *mut Buffer) },
    };

    writer.write_string(string);

}

