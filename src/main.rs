mod cpu;
mod debug;
mod mmu;
mod traps;

use std::{env, fs, io::{self, BufRead, Write}};
use mmu::{Mmu, DRAM_BASE, DRAM_END};
use cpu::{Cpu, StepResult};

type Bus = Mmu;
use cpu::csr::{CSR_CYCLE, CSR_INSTRET};
use debug::disasm::disassemble;
use traps::TrapCause;

//  ELF64 loader — supports only the simplest executables (no dynamic linking, no TLS, no weird sections).
fn load_file(bytes: &[u8]) -> (Bus, u64) {
    let is_elf = bytes.len() >= 4 && bytes[0..4] == [0x7f, b'E', b'L', b'F'];
    if is_elf {
        if let Some((flat, entry)) = load_elf(bytes) {
            return (Mmu::from_dram(flat), entry);
        }
        eprintln!("warning: malformed ELF, falling back to flat binary");
    }
    (Mmu::new(bytes), DRAM_BASE)
}

fn load_elf(b: &[u8]) -> Option<(Vec<u8>, u64)> {
    if *b.get(4)? != 2 { return None; } // ELF64 only
    let entry   = u64::from_le_bytes(b.get(24..32)?.try_into().ok()?);
    let phoff   = u64::from_le_bytes(b.get(32..40)?.try_into().ok()?) as usize;
    let phentsz = u16::from_le_bytes(b.get(54..56)?.try_into().ok()?) as usize;
    let phnum   = u16::from_le_bytes(b.get(56..58)?.try_into().ok()?) as usize;

    let dram_size = (DRAM_END - DRAM_BASE) as usize;
    let mut flat = vec![0u8; dram_size];

    for i in 0..phnum {
        let ph = phoff + i * phentsz;
        let ptype  = u32::from_le_bytes(b.get(ph..ph+4)?.try_into().ok()?);
        if ptype != 1 { continue; } // PT_LOAD only
        let foff   = u64::from_le_bytes(b.get(ph+8..ph+16)?.try_into().ok()?) as usize;
        let paddr  = u64::from_le_bytes(b.get(ph+24..ph+32)?.try_into().ok()?);
        let filesz = u64::from_le_bytes(b.get(ph+32..ph+40)?.try_into().ok()?) as usize;
        if paddr < DRAM_BASE || paddr >= DRAM_END { continue; }
        let off = (paddr - DRAM_BASE) as usize;
        if off + filesz > dram_size { continue; }
        flat[off..off+filesz].copy_from_slice(b.get(foff..foff+filesz)?);
    }
    Some((flat, entry))
}

// ANSI HELPERS: control codes for clearing the screen and coloring text. --- IGNORE ---
const CLR: &str = "\x1b[2J\x1b[H";
const DIM: &str = "\x1b[2m";
const RST: &str = "\x1b[0m";
const YLW: &str = "\x1b[33m";
const GRN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYN: &str = "\x1b[36m";

fn print_state(cpu: &Cpu, bus: &Bus, bps: &[u64]) {
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
    let cycle   = cpu.csr.read(CSR_CYCLE);
    let instret = cpu.csr.read(CSR_INSTRET);
    println!("\n{CYN}── status ───────────────────────────────────────────────────────────{RST}");
    println!("  halted={:<5}  breakpoints={}  cycle={}  instret={}",
        cpu.halted, bps.len(), cycle, instret);
    if !bps.is_empty() {
        let bp_list: Vec<String> = bps.iter().map(|a| format!("{a:#010x}")).collect();
        println!("  breakpoints: {}", bp_list.join("  "));
    }
}

fn prompt(bps: &[u64]) -> String {
    let marker = if bps.is_empty() { "" } else { " [bp]" };
    print!("\n{YLW}dbg{marker}>{RST} ");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line).ok();
    line.trim().to_string()
}

//  VM helpers 

fn vm_tick(cpu: &mut Cpu, bus: &mut Bus) -> StepResult {
    if bus.clint.timer_pending() { cpu.csr.set_mip_timer(); }
    else                         { cpu.csr.clear_mip_timer(); }

    let r = cpu.step(bus);

    if matches!(r, StepResult::Trap(TrapCause::EcallFromM)) {
        if cpu.regs[17] == 93 { return StepResult::Halted; } // SYS_exit
        cpu.pc = cpu.pc.wrapping_add(4);
    }
    r
}

fn run_until(cpu: &mut Cpu, bus: &mut Bus, bps: &[u64], n: Option<u64>) -> StepResult {
    let max = n.unwrap_or(u64::MAX);
    for _ in 0..max {
        let r = vm_tick(cpu, bus);
        if !matches!(r, StepResult::Ok) { return r; }
        if bps.contains(&cpu.pc) { return StepResult::Ok; }
    }
    StepResult::Ok
}

//Debugger: a simple command-line interface for stepping through instructions, inspecting state, and setting breakpoints.
fn run_debugger(cpu: &mut Cpu, bus: &mut Bus) {
    let mut bps: Vec<u64> = Vec::new();
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
                    if let Some(pos) = bps.iter().position(|&a| a == addr) {
                        bps.remove(pos);
                        println!("  breakpoint removed at {addr:#010x}");
                    } else {
                        bps.push(addr);
                        println!("  breakpoint set at {addr:#010x}");
                    }
                } else {
                    let pc = cpu.pc;
                    if let Some(pos) = bps.iter().position(|&a| a == pc) { bps.remove(pos); }
                    else { bps.push(pc); }
                }
            }
            "q" | "quit" => break,
            other if !other.is_empty() =>
                println!("{DIM}  unknown command '{other}'. try: s r c b q{RST}"),
            _ => {}
        }
    }
}

//  Headless run 

fn run_headless(cpu: &mut Cpu, bus: &mut Bus) {
    loop {
        match vm_tick(cpu, bus) {
            StepResult::Halted => {
                println!("halted  pc={:#010x}  a0={}  cycles={}",
                    cpu.pc, cpu.regs[10] as i64, cpu.csr.read(CSR_CYCLE));
                break;
            }
            StepResult::Trap(t) if !matches!(t, TrapCause::EcallFromM) => {
                eprintln!("trap {:?}  pc={:#010x}", t, cpu.pc);
                break;
            }
            _ => {}
        }
    }
}

// Entry point 

fn main() {
    let args: Vec<String> = env::args().collect();
    let debug = args.contains(&"--debug".to_string()) || args.contains(&"-d".to_string());
    let path  = args.iter().find(|a| !a.starts_with('-') && *a != &args[0]);

    let demo_words: &[u32] = &[
        0x0000_0113, 0x0010_0193, 0x0000_0293, 0x00A0_0313,
        0x0062_8C63, 0x0031_0233, 0x0001_8113, 0x0002_0193,
        0x0012_8293, 0xFEDF_F06F, 0x0001_0513, 0x0010_0073,
    ];
    let demo: Vec<u8> = demo_words.iter().flat_map(|w| w.to_le_bytes()).collect();

    let (mut bus, entry) = match path {
        Some(p) => {
            let bytes = fs::read(p).unwrap_or_else(|e| { eprintln!("error: {e}"); std::process::exit(1); });
            load_file(&bytes)
        }
        None => {
            println!("no binary supplied — running built-in demo: fib(10)");
            (Mmu::new(&demo), DRAM_BASE)
        }
    };

    let mut cpu = Cpu::new(entry);
    if debug { run_debugger(&mut cpu, &mut bus); }
    else     { run_headless(&mut cpu, &mut bus); }
}
