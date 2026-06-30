use crate::sbi;
use core::fmt::{self, Write};

const UART_BASE: *mut u8 = 0x1000_0000 as *mut u8;

fn uart_write_byte(c: u8) {
    unsafe {
        core::ptr::write_volatile(UART_BASE, c);
    }
}

pub fn putchar(c: u8) {
    match c {
        b'\n' => {
            sbi::legacy_console_putchar(b'\r');
            sbi::legacy_console_putchar(b'\n');
        }
        _ => {
            sbi::legacy_console_putchar(c);
        }
    }
}

pub fn getchar() -> Option<u8> {
    let c = sbi::legacy_console_getchar();
    if c < 0 {
        None
    } else {
        Some(c as u8)
    }
}

pub fn puts(s: &str) {
    for &b in s.as_bytes() {
        putchar(b);
    }
}

struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        puts(s);
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    ConsoleWriter.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
