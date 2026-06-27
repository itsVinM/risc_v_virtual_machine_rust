use crate::traps::TrapCause;

pub const UART_BASE: u64 = 0x1000_0000;
pub const UART_END:  u64 = 0x1000_00FF;

const THR: u64 = 0;
const LSR: u64 = 5;
const LSR_TX_EMPTY: u8 = 1 << 5;

pub struct Uart {
    out_buf: [u8; 4096],
    out_len: usize,
}

impl Uart {
    pub fn new() -> Self {
        Self { out_buf: [0; 4096], out_len: 0 }
    }

    pub fn read8(&self, offset: u64) -> Result<u8, TrapCause> {
        match offset {
            LSR => Ok(LSR_TX_EMPTY),
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
        if offset == THR {
            if val == b'\n' && self.out_len + 1 < self.out_buf.len() {
                self.out_buf[self.out_len] = val;
                self.out_len += 1;
            } else if val == b'\r' {
            } else if self.out_len < self.out_buf.len() {
                self.out_buf[self.out_len] = val;
                self.out_len += 1;
            }
        }
        Ok(())
    }

}
