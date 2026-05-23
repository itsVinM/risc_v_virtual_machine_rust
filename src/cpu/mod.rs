pub mod csr;
pub mod decoder;
pub mod executor;

use crate::mmu::Mmu as Bus;
use crate::traps::{TrapCause, pending_interrupt, MSTATUS_MIE, MSTATUS_MPIE, MSTATUS_MPP};
use csr::{CsrFile, CSR_MSTATUS, CSR_MEPC, CSR_MCAUSE, CSR_MTVAL, CSR_MTVEC};
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
}

impl Cpu {
    pub fn new(entry: u64) -> Self {
        let mut regs = [0u64; 32];
        // sp at top of DRAM
        regs[2] = crate::mmu::DRAM_END;
        Self { regs, pc: entry, csr: CsrFile::new(), halted: false }
    }

    pub fn step(&mut self, bus: &mut Bus) -> StepResult {
        if self.halted { return StepResult::Halted; }

        bus.tick();
        self.csr.inc_cycle();

        // Check pending interrupts
        if let Some(irq) = pending_interrupt(self.csr.mstatus(), self.csr.mie(), self.csr.mip()) {
            self.handle_trap(irq, self.pc, 0);
            return StepResult::Ok;
        }

        // Fetch
        let raw = match bus.read32(self.pc) {
            Ok(v)  => v,
            Err(e) => { self.handle_trap(e, self.pc, self.pc); return StepResult::Trap(e); }
        };

        // Decode → Execute
        let result = execute(decode(raw), self.pc, &mut self.regs, &mut self.csr, bus);
        self.csr.inc_instret();

        if result.halt {
            self.halted = true;
            return StepResult::Halted;
        }

        if let Some(trap) = result.trap {
            self.handle_trap(trap, self.pc, 0);
            return StepResult::Trap(trap);
        }

        self.regs[0] = 0; // x0 always zero
        self.pc = result.next_pc;
        StepResult::Ok
    }

    fn handle_trap(&mut self, cause: TrapCause, pc: u64, tval: u64) {
        let code = cause.code();
        self.csr.write(CSR_MEPC, pc);
        self.csr.write(CSR_MCAUSE, code);
        self.csr.write(CSR_MTVAL, tval);

        // Update mstatus: save MIE to MPIE, set MPP=M(3), clear MIE
        let mut ms = self.csr.mstatus();
        let mie = (ms & MSTATUS_MIE) >> 3;
        ms &= !(MSTATUS_MIE | MSTATUS_MPIE | MSTATUS_MPP);
        ms |= mie << 7;       // MPIE = old MIE
        ms |= 0b11 << 11;     // MPP = M
        self.csr.write(CSR_MSTATUS, ms);

        let mtvec = self.csr.read(CSR_MTVEC);
        self.pc = if cause.is_interrupt() && (mtvec & 1 == 1) {
            (mtvec & !3).wrapping_add(4 * (code & 0x7FFF_FFFF_FFFF_FFFF))
        } else {
            mtvec & !3
        };
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
