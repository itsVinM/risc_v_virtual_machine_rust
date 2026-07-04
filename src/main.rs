mod cpu;
mod debug;
mod dtb;
mod mmu;
mod sbi;
mod traps;
mod uart;
mod virtio;

use std::{
    collections::HashSet,
    env, fs,
    io::{self, BufRead, Write},
};
use crate::mmu::{Mmu, DRAM_BASE, DRAM_END};
use crate::cpu::{Cpu, StepResult};
use crate::cpu::csr::{CSR_CYCLE, CSR_INSTRET, CSR_MEPC, CSR_MSTATUS, Privilege};

type Bus = Mmu;
use crate::cpu::csr::{MSTATUS_SPP, MSTATUS_SPIE, MSTATUS_SIE};
use crate::debug::disasm::disassemble;
use crate::traps::TrapCause;

const KERNEL_BASE: u64 = 0x8020_0000; // Standard Linux kernel entry
const DTB_ADDR:    u64 = 0x8600_0000; // inside DRAM

// RISC-V ecall/SBI convetions
const SYS_EXIT: u64 = 93; //Linux syscall number for exit
const SBI_SHUTDOWN: u64 = 8; // SBI call for shutdown

// ===== Binary loading =====
fn load_file(bytes: &[u8]) -> (Bus, u64){
    if let Some((flat, entry)) = load_elf(bytes){
        (Mmu::from_dram(flat), entry)
    } else {(Mmu::new(bytes), DRAM_BASE)}   
}

/// Parse a 64-bit little-endian RISC-V ELF and flatten its loadable
/// segments into a DRAM-sized buffer
fn load_elf(bytes: &[u8])->Option<(Vec<u8>, u64)>{
    if bytes.len() < 64 || bytes.get(0..4)? != &[0x7f, b'E', b'L', b'F'] { return None; }
    if *bytes.get(4)? != 2 { return None; } // 64-bit
    if *bytes.get(5)? != 1 { return None; } // little-endian
    if *bytes.get(6)? != 1 { return None; } // RISCV
    let machine = u16::from_le_bytes(bytes.get(18..20)?.try_into().ok()?);
    if machine != 0xF3 { return None; } // RISCV
    
    let entry = u64::from_le_bytes(bytes.get(24..32)?.try_into().ok()?);
    let phoff = u64::from_le_bytes(bytes.get(32..40)?.try_into().ok()?) as usize;
    let phentsize = u16::from_le_bytes(bytes.get(54..56)?.try_into().ok()?) as usize;
    let phnum = u16::from_le_bytes(bytes.get(56..58)?.try_into().ok()?) as usize;
    let dram_size = (DRAM_END - DRAM_BASE) as usize;
    let mut flat = vec![0u8; dram_size];


    for idx in 0..phnum{
        let off = phoff + idx * phentsize;
        let p_type = u32::from_le_bytes(bytes.get(off..off+4)?.try_into().ok()?);
        if p_type != 1 { continue; } // PT_LOAD
        let p_offset = u64::from_le_bytes(bytes.get(off+8..off+16)?.try_into().ok()?) as usize;
        let p_vaddr = u64::from_le_bytes(bytes.get(off+16..off+24)?.try_into().ok()?) as usize;
        let p_filesz = u64::from_le_bytes(bytes.get(off+32..off+40)?.try_into().ok()?) as usize;
        let p_memsz = u64::from_le_bytes(bytes.get(off+40..off+48)?.try_into().ok()?) as usize;
        if p_vaddr < DRAM_BASE as usize || p_vaddr + p_memsz > DRAM_END as usize { return None; }
        flat[p_vaddr - DRAM_BASE as usize .. p_vaddr - DRAM_BASE as usize + p_filesz]
            .copy_from_slice(&bytes[p_offset .. p_offset + p_filesz]);
    }
    Some((flat, entry))
}

// ===== S-MODE DEMO KERNEL =====
fn build_smode_demo() -> Vec<u8> {
    let msg = b"Hello from S-mode!\n";
    let mut buf = Vec::new();
    for &ch in msg.iter(){
        let imm = ch as u32;
        let addi_a0 = (imm << 20) | (10 << 7) | 0x13;   // li a0, ch
        code.extend_from_slice(&addi_a0.to_le_bytes());
        let li_a7: u32 = (1 << 20) | (17 << 7) | 0x13;  // li a7, 1 (SBI putchar)
        code.extend_from_slice(&li_a7.to_le_bytes());
        code.extend_from_slice(&0x00000073u32.to_le_bytes()); // ecall
    }
    let li_a7_8: u32 = (SBI_SHUTDOWN as u32) << 20 | (17 << 7) | 0x13; // li a7, 8
    code.extend_from_slice(&li_a7_8.to_le_bytes());
    code.extend_from_slice(&0x00000073u32.to_le_bytes()); // ecall
    code
}

// ===== DTB placement =====
fn write_dtb(bus: &mut Bus, addr: u64) -> Result<(), Box<dyn std::error::Error>>{
    // Implementation for writing DTB
    let dtb = dtb::generate_dtb();
    let off = (addr - DRAM_BASE) as usize;
    let dram = & mut bus.dram_mus().data;
    let end = (off + dtb.len()).min(dram.len());
    dram[off..end].copy_from_slice(&dtb[..end-off]);
    Ok(())
}

// ====== Boot kernel in S-mode ======
fn boot_smode(cpu: &mut Cpu, bus: &mut Bus, entry: u64) -> Result<(), Box<dyn std::error::Error>>{
    write_dtb(bus, DTB_ADDR);

    cpu.regs[10] = 0;        // a0 = hartid
    cpu.regs[11] = DTB_ADDR; // a1 = DTB address

    let ms = cpu.csr.mstatus() & !(MSTATUS_SPP | MSTATUS_SPIE | MSTATUS_SIE);
    cpu.csr.write(CSR_MSTATUS, ms, Privilege::M);
    cpu.csr.write(CSR_MEPC, entry, Privilege::M); 

    cpu.priv_level = Privilege::S;
    cpu.pc = entry;
    Ok(())
}


// ====== ANSI terminal interface ======
const CLR: &str = "\x1b[2J\x1b[H";
const DIM: &str = "\x1b[2m";
const RST: &str = "\x1b[0m";
const YLW: &str = "\x1b[33m";
const GRN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYN: &str = "\x1b[36m";

fn print_state(cpu: &Cpu, bus: &Bus, bps: &HashSet<u64>) {
    print!("{CLR}");
    println!("{CYN}── registers ────────────────────────────────────────────────────────{RST}");
    for i in 0..32usize {
        let v = cpu.regs[i];
        let color = if v != 0 { GRN } else { DIM };
        print!("{color}{:>4}:{RST} {:#018x}  ", Cpu::reg_name(i), v);
        if i % 4 == 3 { println!(); }
    }
    println!("\n{CYN}── disassembly ──────────────────────────────────────────────────────{RST}");
    for offset in -3i64..=3 {
        let addr = cpu.pc.wrapping_add((offset * 4) as u64);
        let raw  = bus.read32(addr).unwrap_or(0);
        let asm  = disassemble(raw);
        let is_pc = offset == 0;
        let is_bp = bps.contains(&addr);
        let (arrow, color) = match (is_pc, is_bp) {
            (true,  true)  => ("►●", YLW),
            (true,  false) => ("► ", YLW),
            (false, true)  => (" ●", RED),
            (false, false) => ("  ", DIM),
        };
        println!("{color}{arrow} {addr:#010x}: {raw:08x}  {asm}{RST}");
    }
    let cycle   = cpu.csr.read(CSR_CYCLE, Privilege::M);
    let instret = cpu.csr.read(CSR_INSTRET, Privilege::M);
    println!("\n{CYN}── status ───────────────────────────────────────────────────────────{RST}");
    println!("  halted={:<5}  mode={:?}  breakpoints={}  cycle={}  instret={}",
        cpu.halted, cpu.priv_level, bps.len(), cycle, instret);
    if !bps.is_empty() {
        let mut sorted: Vec<u64> = bps.iter().copied().collect();
        sorted.sort_unstable();
        let bp_list: Vec<String> = sorted.iter().map(|a| format!("{a:#010x}")).collect();
        println!("  breakpoints: {}", bp_list.join("  "));
    }
}

fn prompt(bps: &HashSet<u64>) -> String {
    let marker = if bps.is_empty() { "" } else { " [bp]" };
    print!("\n{YLW}dbg{marker}>{RST} ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok();
    line.trim().to_string()
}

// ====== VM TICK ====
fn vm_tick(cpu: &mut Cpu, bus: &mut Bus)-> StepResult{
    if bus.clint.timer_pending() { cpu.csr.set_mip_timer(); }
    else { cpu.csr.clear_mip_timer(); }
    // SSTC: time >= stimecmp → raise supervisor timer interrupt
    if cpu.csr.sstc_enabled() && bus.clint.mtime >= cpu.csr.stimecmp() {
        cpu.csr.set_mip_stimer();
    } else {
        cpu.csr.clear_mip_stimer();
    }
    // PLIC: S-mode external interrupt
    if bus.has_pending_plic_s() { cpu.csr.set_mip_seip(); }
    else { cpu.csr.clear_mip_seip(); }
    // CLINT: software interrupt → mip.SSIP
    if bus.clint.softint_pending() { cpu.csr.set_mip_ssip(); }
    else { cpu.csr.clear_mip_ssip(); }

    let result_step = cpu.step(bus);

    // Handle ecalls ->> SBI for S-mode, SYS_exit for M-mode
    match result_step {
        StepResult::Trap(TrapCause::EcallFromM) => {
            if cpu.regs[17] == SYS_EXIT { return StepResult::Halted; }
            cpu.pc = cpu.pc.wrapping_add(4);
        }
        StepResult::Trap(TrapCause::EcallFromS) => {
            let a7 = cpu.regs[17];
            let a0 = cpu.regs[10];
            let a1 = cpu.regs[11];
            let a2 = cpu.regs[12];
            let a6 = cpu.regs[16];
            let res = sbi::handle_sbi(a7, a0, a1, a2, a6, bus);
            cpu.regs[10] = res.a0;
            if res.halt { return StepResult::Halted; }
            cpu.pc = cpu.pc.wrapping_add(4);
        }
        _ => {}
    }
    result_step
}

fn run_until(cpu: &mut Cpu, bus: &mut Bus, bps: &HashSet<u64>, number: Option<u64>) -> StepResult {
    let max = number.unwrap_or(u64::MAX);
    for _ in 0..max {
        let r = vm_tick(cpu, bus);
        if !matches!(r, StepResult::Ok) { return r; }
        if bps.contains(&cpu.pc) { return StepResult::Ok; }
    }
    StepResult::Ok
}

// ====== DEBUGGER ======
fn run_debugger(cpu: &mut Cpu, bus: &mut Bus) {
    let mut bps: HashSet<u64> = HashSet::new();
    println!("{CYN}RISC-V64 live debugger{RST}");
    println!("commands: s  step   r <n>  run n steps   c  continue");
    println!("          b <hex>  toggle breakpoint     q  quit");
    loop {
        print_state(cpu, bus, &bps);
        if cpu.halted { println!("{YLW}VM halted.{RST}"); break; }
        let cmd = prompt(&bps);
        let mut parts = cmd.splitn(2, ' ');
        match parts.next().unwrap_or("") {
            "s" | "" => { vm_tick(cpu, bus); }
            "r" => {
                let n: u64 = parts.next().and_then(|s| s.trim().parse().ok()).unwrap_or(100);
                let r = run_until(cpu, bus, &bps, Some(n));
                if matches!(r, StepResult::Halted) { cpu.halted = true; }
            }
            "c" => {
                let r = run_until(cpu, bus, &bps, None);
                if matches!(r, StepResult::Halted) { cpu.halted = true; }
            }
            "b" => {
                if let Some(hex) = parts.next() {
                    let addr = u64::from_str_radix(hex.trim().trim_start_matches("0x"), 16)
                        .unwrap_or(cpu.pc);
                    if bps.remove(&addr) {
                        println!("  breakpoint removed at {addr:#010x}");
                    } else {
                        bps.insert(addr);
                        println!("  breakpoint set at {addr:#010x}");
                    }
                } else {
                    let pc = cpu.pc;
                    if !bps.remove(&pc) { bps.insert(pc); }
                }
            }
            "q" | "quit" => break,
            other if !other.is_empty() =>
                println!("{DIM}  unknown command '{other}'. try: s r c b q{RST}"),
            _ => {}
        }
    }
}

// ==== Headless run  ====
fn drain_uart(bus: &mut Bus) {
    let out = bus.uart.flush_output();
    if !out.is_empty() {
        print!("{out}");
    }
}

fn run_headless(cpu: &mut Cpu, bus: &mut Bus) {
    loop {
        match vm_tick(cpu, bus) {
            StepResult::Halted => {
                drain_uart(bus);
                let _ = io::stdout().flush();
                println!("halted  pc={:#010x}  a0={}  cycles={}",
                    cpu.pc, cpu.regs[10] as i64, cpu.csr.read(CSR_CYCLE, Privilege::M));
                break;
            }
            StepResult::Trap(t) if !matches!(t, TrapCause::EcallFromM | TrapCause::EcallFromS) => {
                drain_uart(bus);
                let _ = io::stdout().flush();
                eprintln!("trap {:?}  pc={:#010x}", t, cpu.pc);
                break;
            }
            _ => {
                drain_uart(bus);
            }
        }
    }
    let _ = io::stdout().flush();
}

// ====== MAIN ======
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let debug = args.contains(&"--debug".to_string()) || args.contains(&"-d".to_string());
    let smode = args.contains(&"--sbi".to_string()) || args.contains(&"-s".to_string());
    let path = args.iter().skip(1).find(|a| !a.starts_with('-'));

    if smode {
        let mut is_elf = false;
        let (bin, entry) = match path {
            Some(p) => {
                let bytes = fs::read(p).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
                if bytes.starts_with(&[0x7f, b'E', b'L', b'F']) {
                    is_elf = true;
                    load_file(&bytes)
                } else {
                    (Mmu::new_at(bytes, KERNEL_BASE), KERNEL_BASE)
                }
            }
            None => {
                println!("no kernel supplied — running built-in S-mode demo");
                (Mmu::new_at(build_smode_demo(), KERNEL_BASE), KERNEL_BASE)
            }
        };
        let mut bus = bin;
        let mut cpu = Cpu::new(entry);
        if is_elf {
            write_dtb(&mut bus, DTB_ADDR);
            cpu.regs[10] = 0;
            cpu.regs[11] = DTB_ADDR;
        } else {
            boot_smode(&mut cpu, &mut bus, entry);
        }
        if debug { run_debugger(&mut cpu, &mut bus); }
        else     { run_headless(&mut cpu, &mut bus); }
        return;
    }

    const DEMO: &[u32] = &[
        0x0000_0113, 0x0010_0193, 0x0000_0293, 0x00A0_0313,
        0x0062_8C63, 0x0031_0233, 0x0001_8113, 0x0002_0193,
        0x0012_8293, 0xFEDF_F06F, 0x0001_0513, 0x0010_0073,
    ];
    let demo_mcode: Vec<u8> = DEMO.iter().flat_map(|w| w.to_le_bytes()).collect();

    let (mut bus, entry) = match path {
        Some(p) => {
            let bytes = fs::read(p).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
            load_file(&bytes)
        }
        None => {
            println!("no binary supplied — running built-in demo: fib(10)");
            (Mmu::new(&demo_mcode), DRAM_BASE)
        }
    };
    let mut cpu = Cpu::new(entry);
    if debug { run_debugger(&mut cpu, &mut bus); }
    else     { run_headless(&mut cpu, &mut bus); }
}