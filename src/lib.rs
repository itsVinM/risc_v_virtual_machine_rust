use wasm_bindgen::prelude::*;

pub mod bus;
pub mod cpu;
pub mod debug;
pub mod devices;
pub mod traps;

pub use bus::Bus;
pub use cpu::{Cpu, StepResult};
use traps::TrapCause;

#[wasm_bindgen]
pub struct EmulatorState {
    cpu: Cpu,
    bus: Bus,
}

#[wasm_bindgen]
impl EmulatorState {
    #[wasm_bindgen(constructor)]
    pub fn new(binary: Vec<u8>) -> Self {
        console_error_panic_hook::set_once();
        let bus = Bus::new(&binary);
        let cpu = Cpu::new(bus::DRAM_BASE);
        Self { cpu, bus }
    }

    pub fn step(&mut self) {
        if self.bus.clint.timer_pending() { self.cpu.csr.set_mip_timer(); }
        else                              { self.cpu.csr.clear_mip_timer(); }

        let r = self.cpu.step(&mut self.bus);
        if matches!(r, StepResult::Trap(TrapCause::EcallFromM)) && self.cpu.regs[17] == 93 {
            self.cpu.halted = true;
        }
    }

    pub fn run(&mut self, n: u32) {
        for _ in 0..n {
            if self.cpu.halted { break; }
            self.step();
        }
    }

    pub fn pc(&self)      -> u64  { self.cpu.pc }
    pub fn halted(&self)  -> bool { self.cpu.halted }
    pub fn cycles(&self)  -> u64  { self.cpu.csr.read(cpu::csr::CSR_CYCLE) }
    pub fn instret(&self) -> u64  { self.cpu.csr.read(cpu::csr::CSR_INSTRET) }

    pub fn reg(&self, i: u32) -> u64 {
        if i < 32 { self.cpu.regs[i as usize] } else { 0 }
    }

    pub fn read_mem32(&self, addr: u64) -> u32 {
        self.bus.read32(addr).unwrap_or(0xDEAD_BEEF)
    }
}
