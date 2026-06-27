pub mod csr;
pub mod decoder;
pub mod executor;

use crate::mmu::Mmu as Bus;
use crate::traps::{TrapCause, pending_interrupt};
use crate::cpu::csr::{
    CsrFile, Privilege,
    CSR_MSTATUS, CSR_MEPC, CSR_MCAUSE, CSR_MTVAL, CSR_MTVEC,
    CSR_SEPC, CSR_STVEC, CSR_SCAUSE, CSR_STVAL,
    MSTATUS_MIE, MSTATUS_MPIE, MSTATUS_MPP,
    MSTATUS_SIE, MSTATUS_SPIE, MSTATUS_SPP,
};
use decoder::decode;
use executor::execute;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Ok,
    Halted,
    Trap(TrapCause),
}

pub struct Cpu {
    pub regs: [u64; 32],
    pub pc:   u64,
    pub csr:  CsrFile,
    pub halted: bool,
    pub priv_level: Privilege,
}

impl Cpu {
    pub fn new(entry: u64) -> Self {
        let mut regs = [0u64; 32];
        regs[2] = crate::mmu::DRAM_END;
        Self { regs, pc: entry, csr: CsrFile::new(), halted: false, priv_level: Privilege::M }
    }

    pub fn step(&mut self, bus: &mut Bus) -> StepResult {
        if self.halted { return StepResult::Halted; }

        bus.tick();
        self.csr.inc_cycle();

        // Check pending interrupts — both M and S level
        if let Some((irq, delegated)) = pending_interrupt(
            self.csr.mstatus(), self.csr.mie(), self.csr.mip(),
            self.priv_level, self.csr.mideleg(),
        ) {
            if delegated && self.priv_level < Privilege::M {
                // Delegate to S-mode
                self.take_s_trap(irq, self.pc, 0);
            } else {
                // Take in M-mode
                self.take_trap(irq, self.pc, 0);
            }
            return StepResult::Ok;
        }

        // Fetch with virtual memory translation
        let fetch_pa = translate_fetch(self.pc, self.priv_level, bus, self.csr.read(csr::CSR_SATP, Privilege::M))
            .unwrap_or(self.pc);

        let raw = match bus.read32(fetch_pa) {
            Ok(v)  => v,
            Err(_e) => {
                let cause = if self.priv_level >= Privilege::S {
                    TrapCause::InstructionAccessFault
                } else {
                    TrapCause::InstructionPageFault
                };
                self.take_trap(cause, self.pc, self.pc);
                return StepResult::Trap(cause);
            }
        };

        let result = execute(decode(raw), self.pc, &mut self.regs, &mut self.csr, bus, self.priv_level);
        self.csr.inc_instret();

        if result.halt {
            self.halted = true;
            return StepResult::Halted;
        }

        if let Some(trap) = result.trap {
            // Don't redirect pc for ecalls — caller (vm_tick) handles them directly
            match trap {
                TrapCause::EcallFromU | TrapCause::EcallFromS | TrapCause::EcallFromM => {}
                _ => {
                    let medeleg = self.csr.medeleg();
                    let exc_code = trap.exception_code();
                    let delegate = self.priv_level <= Privilege::S
                        && (medeleg & (1 << exc_code)) != 0;
                    if delegate && self.priv_level < Privilege::M {
                        self.take_s_trap(trap, self.pc, 0);
                    } else {
                        self.take_trap(trap, self.pc, 0);
                    }
                }
            }
            return StepResult::Trap(trap);
        }

        if let Some(new) = result.new_priv { self.priv_level = new; }
        self.regs[0] = 0;
        self.pc = result.next_pc;
        StepResult::Ok
    }

    fn take_trap(&mut self, cause: TrapCause, pc: u64, tval: u64) {
        let code = cause.code();
        self.csr.write(CSR_MEPC, pc, Privilege::M);
        self.csr.write(CSR_MCAUSE, code, Privilege::M);
        self.csr.write(CSR_MTVAL, tval, Privilege::M);

        let mut ms = self.csr.mstatus();
        let mie = (ms & MSTATUS_MIE) >> 3;
        ms &= !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP);
        ms |= mie << 7;
        ms |= self.priv_level.bits() << 11;
        self.csr.write(CSR_MSTATUS, ms, Privilege::M);

        let mtvec = self.csr.read(CSR_MTVEC, Privilege::M);
        self.pc = if cause.is_interrupt() && (mtvec & 1 == 1) {
            (mtvec & !3).wrapping_add(4 * (code & 0x7FFF_FFFF_FFFF_FFFF))
        } else {
            mtvec & !3
        };

        self.priv_level = Privilege::M;
    }

    fn take_s_trap(&mut self, cause: TrapCause, pc: u64, tval: u64) {
        let code = cause.code();
        self.csr.write(CSR_SEPC, pc, Privilege::S);
        self.csr.write(CSR_SCAUSE, code, Privilege::S);
        self.csr.write(CSR_STVAL, tval, Privilege::S);

        let mut ms = self.csr.mstatus();
        let sie = (ms & MSTATUS_SIE) >> 1;
        ms &= !(MSTATUS_SIE | MSTATUS_SPIE | MSTATUS_SPP);
        ms |= sie << 5;
        ms |= (self.priv_level.bits() & 1) << 8;
        self.csr.write(CSR_MSTATUS, ms, Privilege::M);

        let stvec = self.csr.read(CSR_STVEC, Privilege::S);
        self.pc = if cause.is_interrupt() && (stvec & 1 == 1) {
            (stvec & !3).wrapping_add(4 * (code & 0x7FFF_FFFF_FFFF_FFFF))
        } else {
            stvec & !3
        };

        self.priv_level = Privilege::S;
    }

    pub fn reg_name(r: usize) -> &'static str {
        const NAMES: [&str; 32] = [
            "zero","ra","sp","gp","tp","t0","t1","t2",
            "s0","s1","a0","a1","a2","a3","a4","a5",
            "a6","a7","s2","s3","s4","s5","s6","s7",
            "s8","s9","s10","s11","t3","t4","t5","t6",
        ];
        NAMES[r]
    }
}

fn translate_fetch(pc: u64, priv_level: Privilege, bus: &Bus, satp: u64) -> Result<u64, TrapCause> {
    if priv_level == Privilege::M {
        return Ok(pc);
    }
    let mode = (satp >> 60) & 0xF;
    if mode == 0 {
        return Ok(pc);
    }
    // Sv39 page walk for instruction fetch
    let vpn2 = (pc >> 30) & 0x1FF;
    let vpn1 = (pc >> 21) & 0x1FF;
    let vpn0 = (pc >> 12) & 0x1FF;
    let offset = pc & 0xFFF;

    let root_ppn = satp & ((1u64 << 44) - 1);
    let mut pte_addr = (root_ppn << 12) + (vpn2 * 8);

    let mut pte = match bus.read_phys64(pte_addr) {
        Ok(v) => v,
        Err(_) => return Err(TrapCause::InstructionPageFault),
    };

    if pte & 1 == 0 {
        return Err(TrapCause::InstructionPageFault);
    }

    for &vpn in &[vpn1, vpn0] {
        if pte & (PTE_R | PTE_W | PTE_X) != 0 {
            break;
        }
        let ppn = (pte >> 10) & 0x00FF_FFFF_FFFF;
        pte_addr = (ppn << 12) + (vpn * 8);
        pte = match bus.read_phys64(pte_addr) {
            Ok(v) => v,
            Err(_) => return Err(TrapCause::InstructionPageFault),
        };
        if pte & 1 == 0 {
            return Err(TrapCause::InstructionPageFault);
        }
    }

    if pte & PTE_X == 0 {
        return Err(TrapCause::InstructionPageFault);
    }

    let ppn = (pte >> 10) & 0x00FF_FFFF_FFFF;
    Ok((ppn << 12) | offset)
}

const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
