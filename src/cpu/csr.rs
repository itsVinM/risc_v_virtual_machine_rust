// CSR addresses (machine-mode subset)
pub const CSR_MSTATUS:  usize = 0x300;
pub const CSR_MISA:     usize = 0x301;
pub const CSR_MIE:      usize = 0x304;
pub const CSR_MTVEC:    usize = 0x305;
pub const CSR_MSCRATCH: usize = 0x340;
pub const CSR_MEPC:     usize = 0x341;
pub const CSR_MCAUSE:   usize = 0x342;
pub const CSR_MTVAL:    usize = 0x343;
pub const CSR_MIP:      usize = 0x344;
pub const CSR_CYCLE:    usize = 0xC00;
pub const CSR_TIME:     usize = 0xC01;
pub const CSR_INSTRET:  usize = 0xC02;

const NUM_CSRS: usize = 4096;

pub struct CsrFile {
    regs: [u64; NUM_CSRS],
}

impl CsrFile {
    pub fn new() -> Self {
        let mut s = Self { regs: [0u64; NUM_CSRS] };
        // RV64IMAC
        s.regs[CSR_MISA] = (2u64 << 62) | (1 << 0) | (1 << 8) | (1 << 12) | (1 << 2) | (1 << 0);
        s
    }

    pub fn read(&self, addr: usize) -> u64 {
        if addr < NUM_CSRS { self.regs[addr] } else { 0 }
    }

    pub fn write(&mut self, addr: usize, val: u64) {
        if addr < NUM_CSRS {
            // CYCLE/TIME/INSTRET are read-only shadows
            if matches!(addr, CSR_CYCLE | CSR_TIME | CSR_INSTRET) { return; }
            self.regs[addr] = val;
        }
    }

    pub fn read_write(&mut self, addr: usize, val: u64, op: CsrOp) -> u64 {
        let old = self.read(addr);
        let new = match op {
            CsrOp::Write => val,
            CsrOp::Set   => old | val,
            CsrOp::Clear => old & !val,
        };
        self.write(addr, new);
        old
    }

    pub fn mstatus(&self) -> u64 { self.regs[CSR_MSTATUS] }
    pub fn mie(&self)     -> u64 { self.regs[CSR_MIE] }
    pub fn mip(&self)     -> u64 { self.regs[CSR_MIP] }

    pub fn set_mip_timer(&mut self)   { self.regs[CSR_MIP] |=  (1 << 7); }
    pub fn clear_mip_timer(&mut self) { self.regs[CSR_MIP] &= !(1 << 7); }

    pub fn inc_cycle(&mut self)   { self.regs[CSR_CYCLE]   += 1; }
    pub fn inc_instret(&mut self) { self.regs[CSR_INSTRET] += 1; }
    pub fn set_time(&mut self, t: u64) { self.regs[CSR_TIME] = t; }
}

#[derive(Clone, Copy)]
pub enum CsrOp { Write, Set, Clear }
