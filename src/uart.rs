use crate::traps::TrapCause;

pub const UART_BASE: u64 = 0x1000_0000;
pub const UART_END:  u64 = 0x1000_00FF;

// 8250 UART register offsets (DLAB=0)
const THR: u64 = 0; // TX (write)
const IER: u64 = 1; // Interrupt Enable
// DLAB=1: DLL at 0, DLM at 1
const FCR: u64 = 2; // FIFO Control (write)
const LCR: u64 = 3; // Line Control
const MCR: u64 = 4; // Modem Control
const LSR: u64 = 5; // Line Status
const MSR: u64 = 6; // Modem Status

const LSR_TX_EMPTY: u8 = 1 << 5;
const LSR_TX_EMPTY_ALL: u8 = 1 << 6;

pub struct Uart {
    out_buf: [u8; 4096],
    out_len: usize,
    ier: u8,
    lcr: u8,
    mcr: u8,
    dll: u8,
    dlm: u8,
}

impl Uart {
    pub fn new() -> Self {
        Self { out_buf: [0; 4096], out_len: 0, ier: 0, lcr: 0, mcr: 0, dll: 0, dlm: 0 }
    }

    pub fn read8(&self, offset: u64) -> Result<u8, TrapCause> {
        match offset {
            0 if self.lcr & 0x80 != 0 => Ok(self.dll),
            1 if self.lcr & 0x80 != 0 => Ok(self.dlm),
            IER => Ok(self.ier),
            LCR => Ok(self.lcr),
            MCR => Ok(self.mcr),
            LSR => Ok(LSR_TX_EMPTY | LSR_TX_EMPTY_ALL),
            MSR => Ok(0xF0), // DCD+RI+DSR+CTS asserted
            _ => Ok(0),
        }
    }

    pub fn flush_output(&mut self) -> &str {
        if self.out_len == 0 { return ""; }
        let s = core::str::from_utf8(&self.out_buf[..self.out_len]).unwrap_or("");
        self.out_len = 0;
        s
    }

    pub fn write8(&mut self, offset: u64, val: u8) -> Result<(), TrapCause> {
        match offset {
            0 if self.lcr & 0x80 != 0 => self.dll = val,
            1 if self.lcr & 0x80 != 0 => self.dlm = val,
            THR => {
                if val != b'\r' && self.out_len < self.out_buf.len() {
                    self.out_buf[self.out_len] = val;
                    self.out_len += 1;
                }
            }
            IER => self.ier = val & 0x0F,
            FCR => {} // ignore FIFO control
            LCR => self.lcr = val,
            MCR => self.mcr = val & 0x1F,
            _ => {}
        }
        Ok(())
    }

}
