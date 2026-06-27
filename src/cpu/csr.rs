pub const CSR_MSTATUS:    usize = 0x300;
pub const CSR_MISA:       usize = 0x301;
pub const CSR_MEDELEG:    usize = 0x302;
pub const CSR_MIDELEG:    usize = 0x303;
pub const CSR_MIE:        usize = 0x304;
pub const CSR_MTVEC:      usize = 0x305;
pub const CSR_MENVCFG:    usize = 0x30A;
pub const CSR_MEPC:       usize = 0x341;
pub const CSR_MCAUSE:     usize = 0x342;
pub const CSR_MTVAL:      usize = 0x343;
pub const CSR_MIP:        usize = 0x344;


pub const CSR_SSTATUS:    usize = 0x100;
pub const CSR_SIE:        usize = 0x104;
pub const CSR_STVEC:      usize = 0x105;
pub const CSR_STIMECMP:   usize = 0x14D;
pub const CSR_SEPC:       usize = 0x141;
pub const CSR_SCAUSE:     usize = 0x142;
pub const CSR_STVAL:      usize = 0x143;
pub const CSR_SIP:        usize = 0x144;
pub const CSR_SATP:       usize = 0x180;

pub const CSR_CYCLE:      usize = 0xC00;
pub const CSR_TIME:       usize = 0xC01;
pub const CSR_INSTRET:    usize = 0xC02;

const NUM_CSRS: usize = 4096;

pub const MENVCFG_STCE: u64 = 1 << 63;

pub const MSTATUS_MIE:  u64 = 1 << 3;
pub const MSTATUS_MPIE: u64 = 1 << 7;
pub const MSTATUS_MPP:  u64 = 0b11 << 11;
pub const MSTATUS_SPP:  u64 = 1 << 8;
pub const MSTATUS_SPIE: u64 = 1 << 5;
pub const MSTATUS_SIE:  u64 = 1 << 1;
pub const MSTATUS_SUM:  u64 = 1 << 18;
pub const MSTATUS_MXR:  u64 = 1 << 19;
pub const MSTATUS_FS:   u64 = 0b11 << 13;
pub const MSTATUS_XS:   u64 = 0b11 << 15;
pub const MSTATUS_VS:   u64 = 0b11 << 9;

pub const MIP_SSIP: u64 = 1 << 1;
pub const MIP_STIP: u64 = 1 << 5;
pub const MIP_SEIP: u64 = 1 << 9;
pub const MIP_MSIP: u64 = 1 << 3;
pub const MIP_MTIP: u64 = 1 << 7;
pub const MIP_MEIP: u64 = 1 << 11;

pub const SATP_PPN:   u64 = (1 << 44) - 1;
pub const SATP_MODE_BARE: u64 = 0;
pub const SATP_MODE_SV39: u64 = 8;

pub struct CsrFile {
    regs: [u64; NUM_CSRS],
    stimecmp: u64,
}

impl CsrFile {
    pub fn new() -> Self {
        let mut s = Self { regs: [0u64; NUM_CSRS], stimecmp: u64::MAX };
        s.regs[CSR_MISA] = (2u64 << 62) | (1 << 0) | (1 << 8) | (1 << 12);
        s.regs[CSR_MEDELEG] = (1 << 3) | (1 << 8) | (1 << 9) | (1 << 12) | (1 << 13) | (1 << 15);
        s.regs[CSR_MIDELEG] = MIP_SSIP | MIP_STIP | MIP_SEIP;
        s
    }

    pub fn read(&self, addr: usize, priv_level: Privilege) -> u64 {
        if addr >= NUM_CSRS { return 0; }
        match addr {
            CSR_SSTATUS => self.regs[CSR_MSTATUS] & 0x8000_0000_0000_0033,
            CSR_SIE     => self.regs[CSR_MIE] & self.regs[CSR_MIDELEG],
            CSR_SIP     => self.regs[CSR_MIP] & self.regs[CSR_MIDELEG],
            CSR_SATP if priv_level == Privilege::S => self.regs[CSR_SATP],
            CSR_TIME    => self.regs[CSR_CYCLE], // time = cycle count
            CSR_CYCLE | CSR_INSTRET => self.regs[addr],
            CSR_STIMECMP if priv_level <= Privilege::S => {
                if priv_level == Privilege::S && !self.sstc_enabled() {
                    0
                } else {
                    self.stimecmp
                }
            }
            _ => {
                if priv_level < Privilege::M && (0x300..=0x3FF).contains(&addr) {
                    return 0;
                }
                self.regs[addr]
            }
        }
    }

    pub fn write(&mut self, addr: usize, val: u64, priv_level: Privilege) {
        if addr >= NUM_CSRS { return; }
        match addr {
            CSR_CYCLE | CSR_TIME | CSR_INSTRET => {}
            CSR_SSTATUS => {
                let mask = MSTATUS_SPP | MSTATUS_SPIE | MSTATUS_SIE
                         | MSTATUS_FS | MSTATUS_XS | MSTATUS_VS
                         | MSTATUS_SUM | MSTATUS_MXR;
                self.regs[CSR_MSTATUS] = (self.regs[CSR_MSTATUS] & !mask) | (val & mask);
            }
            CSR_SIE => {
                let mask = self.regs[CSR_MIDELEG] & (MIP_SSIP | MIP_STIP | MIP_SEIP);
                let delegated = val & mask;
                self.regs[CSR_MIE] = (self.regs[CSR_MIE] & !mask) | delegated;
            }
            CSR_SIP => {
                let mask = self.regs[CSR_MIDELEG] & (MIP_SSIP | MIP_STIP | MIP_SEIP);
                let delegated = val & mask;
                self.regs[CSR_MIP] = (self.regs[CSR_MIP] & !mask) | delegated;
            }
            CSR_SATP if priv_level == Privilege::S => {
                self.regs[CSR_SATP] = val;
            }
            CSR_STIMECMP if priv_level <= Privilege::S => {
                if priv_level == Privilege::S && !self.sstc_enabled() {
                    return;
                }
                self.stimecmp = val;
            }
            _ => {
                if (0x300..=0x3FF).contains(&addr) && priv_level != Privilege::M { return; }
                self.regs[addr] = val;
            }
        }
    }

    pub fn read_write(&mut self, addr: usize, val: u64, op: CsrOp, priv_level: Privilege) -> u64 {
        let old = self.read(addr, priv_level);
        let new = match op {
            CsrOp::Write => val,
            CsrOp::Set   => old | val,
            CsrOp::Clear => old & !val,
        };
        self.write(addr, new, priv_level);
        old
    }

    pub fn mstatus(&self) -> u64 { self.regs[CSR_MSTATUS] }
    pub fn mie(&self)     -> u64 { self.regs[CSR_MIE] }
    pub fn mip(&self)     -> u64 { self.regs[CSR_MIP] }

    pub fn set_mip_timer(&mut self)   { self.regs[CSR_MIP] |= MIP_MTIP; }
    pub fn clear_mip_timer(&mut self) { self.regs[CSR_MIP] &= !MIP_MTIP; }
    pub fn set_mip_stimer(&mut self)  { self.regs[CSR_MIP] |= MIP_STIP; }
    pub fn clear_mip_stimer(&mut self) { self.regs[CSR_MIP] &= !MIP_STIP; }
    pub fn set_mip_seip(&mut self)   { self.regs[CSR_MIP] |= MIP_SEIP; }
    pub fn clear_mip_seip(&mut self) { self.regs[CSR_MIP] &= !MIP_SEIP; }
    pub fn set_mip_ssip(&mut self)   { self.regs[CSR_MIP] |= MIP_SSIP; }
    pub fn clear_mip_ssip(&mut self) { self.regs[CSR_MIP] &= !MIP_SSIP; }

    pub fn inc_cycle(&mut self)   { self.regs[CSR_CYCLE]   += 1; }
    pub fn inc_instret(&mut self) { self.regs[CSR_INSTRET] += 1; }

    pub fn medeleg(&self) -> u64 { self.regs[CSR_MEDELEG] }
    pub fn mideleg(&self) -> u64 { self.regs[CSR_MIDELEG] }

    pub fn sstc_enabled(&self) -> bool {
        (self.regs[CSR_MENVCFG] & MENVCFG_STCE) != 0
    }

    pub fn stimecmp(&self) -> u64 { self.stimecmp }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CsrOp { Write, Set, Clear }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Debug)]
pub enum Privilege {
    U = 0,
    S = 1,
    M = 3,
}

impl Privilege {
    pub fn bits(self) -> u64 {
        match self {
            Privilege::U => 0,
            Privilege::S => 1,
            Privilege::M => 3,
        }
    }
}

pub fn csr_access_ok(addr: usize, priv_level: Privilege) -> bool {
    if priv_level == Privilege::M { return true; }
    match addr {
        0x100..=0x1FF => priv_level >= Privilege::S,
        0x300..=0x3FF => false,
        0xC00..=0xC1F => true,
        0xC80..=0xC9F => true,
        _ => false,
    }
}
