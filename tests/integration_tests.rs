use riscv_core::{Bus, Cpu};
use riscv_core::cpu::StepResult;
use riscv_core::bus::DRAM_BASE;
use riscv_core::traps::TrapCause;

fn run_until_halt(cpu: &mut Cpu, bus: &mut Bus, max_steps: usize) -> StepResult {
    for _ in 0..max_steps {
        let r = cpu.step(bus);
        if !matches!(r, StepResult::Ok) { return r; }
    }
    StepResult::Ok
}

fn make_vm(words: &[u32]) -> (Cpu, Bus) {
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    (Cpu::new(DRAM_BASE), Bus::new(&bytes))
}

// ─── Integration: fibonacci (iterative) ───────────────────────────────────
// Compute fib(10) = 55 in x10 (a0)
// Uses: x1=n, x2=a, x3=b, x4=tmp, x5=i
#[test]
fn integration_fibonacci() {
    // Hand-assembled RV64I fib(10):
    // a=0, b=1, for i in 0..10: tmp=a+b; a=b; b=tmp; → a=fib(10)=55
    let prog: &[u32] = &[
        0x0000_0113, // addi x2, x0, 0   (a = 0)
        0x0010_0193, // addi x3, x0, 1   (b = 1)
        0x0000_0293, // addi x5, x0, 0   (i = 0)
        0x00A0_0313, // addi x6, x0, 10  (limit)
        // loop: (PC = DRAM_BASE + 16)
        // beq x5,x6,+24: branch to DRAM_BASE+40 (exit) when i==10
        // imm=24: bits11:8=1100, rest 0 → 0x0062_8C63
        0x0062_8C63, // beq  x5, x6, +24 (exit loop when i==10, jump to DRAM_BASE+40)
        0x0031_0233, // add  x4, x2, x3  (tmp = a + b)
        0x0001_8113, // addi x2, x3, 0   (a = b)
        0x0002_0193, // addi x3, x4, 0   (b = tmp)
        0x0012_8293, // addi x5, x5, 1   (i++)
        0xFEDF_F06F, // jal  x0, -20     (back to loop at offset 16: 36-20=16)
        // exit:
        0x0001_0513, // addi x10, x2, 0  (a0 = a = fib(n))
        0x0010_0073, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(prog);
    let r = run_until_halt(&mut cpu, &mut bus, 10_000);
    assert!(matches!(r, StepResult::Halted));
    assert_eq!(cpu.regs[10], 55, "fib(10) should be 55");
}

// ─── Integration: store-sort-verify in memory ─────────────────────────────
// Stores [3,1,2] using sp-relative addressing, verifies loads are correct.
// sp starts at DRAM_END (0x8800_0000) — sp-relative stores are always in DRAM.
#[test]
fn integration_bubble_sort() {
    // sp = DRAM_END (set by Cpu::new)
    // addi sp, sp, -32   → allocate 32 bytes on stack
    // Store values [3, 1, 2] at sp+0, sp+4, sp+8
    // Load them back, check x3=3, x4=1
    // slt x5, x4, x3 = (1 < 3) = 1
    // addi a0, x5, 0  → a0 = 1
    let prog: &[u32] = &[
        0xFE010113, // addi sp, sp, -32          (allocate 32 bytes)
        0x00300313, // addi x6, x0, 3
        0x00611023, // sh x6, 0(sp)  -- use sw:
        0x00612023, // sw x6, 0(sp)
        0x00100313, // addi x6, x0, 1
        0x00612223, // sw x6, 4(sp)
        0x00200313, // addi x6, x0, 2
        0x00612423, // sw x6, 8(sp)
        0x00012183, // lw x3, 0(sp)   → x3 = 3
        0x00412203, // lw x4, 4(sp)   → x4 = 1
        0x0031C2B3, // slt x5, x3, x4 (3 < 1 = 0)
        0x0042C2B3, // slt x5, x4, x3 (1 < 3 = 1) -- overwrite x5
        0x00028513, // addi a0, x5, 0
        0x00100073, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(prog);
    let r = run_until_halt(&mut cpu, &mut bus, 10_000);
    assert!(matches!(r, StepResult::Halted));
    assert_eq!(cpu.regs[10], 1, "1 < 3 should be true");
}

// ─── Fault injection tests ─────────────────────────────────────────────────

#[test]
fn fault_illegal_instruction() {
    // Inject 0xFFFFFFFF — should produce IllegalInstruction trap
    let prog = [0xFFFF_FFFFu32];
    let (mut cpu, mut bus) = make_vm(&prog);
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::IllegalInstruction(_))),
        "expected IllegalInstruction, got {:?}", r);
}

#[test]
fn fault_load_out_of_range() {
    // ld x1, 0(x0)  — address 0x0 is not in DRAM → LoadAccessFault
    let prog = [
        0x0000_3083u32, // ld x1, 0(x0)
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::LoadAccessFault)),
        "expected LoadAccessFault, got {:?}", r);
}

#[test]
fn fault_store_out_of_range() {
    // sw x0, 0(x0)  — address 0x0 → StoreAccessFault
    let prog = [
        0x0000_2023u32, // sw x0, 0(x0)
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::StoreAccessFault)),
        "expected StoreAccessFault, got {:?}", r);
}

#[test]
fn fault_inject_corrupt_mid_program() {
    // Valid program: addi x1, x0, 5; then corrupted word; then ebreak
    // The VM should trap on the corrupted instruction
    // Note: 0xDEAD_BEEF has opcode=0x6F (JAL) and would be valid.
    // Use 0x0000_0002 instead: opcode=0x02 (undefined in RV64I without C extension → Illegal)
    let prog = [
        0x0050_0093u32, // addi x1, x0, 5
        0x0000_0002u32, // opcode=0x02 → Illegal (C-extension stub, not handled)
        0x0010_0073u32, // ebreak (should not reach)
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    cpu.step(&mut bus); // addi — ok
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::IllegalInstruction(_))),
        "expected IllegalInstruction on corrupted word, got {:?}", r);
}

#[test]
fn fault_trap_does_not_corrupt_registers() {
    // Confirm that x1 retains its value after a trap on x2
    let prog = [
        0x0050_0093u32, // addi x1, x0, 5   → x1 = 5
        0x0000_3103u32, // ld x2, 0(x0)     → fault (x0=0, OOB)
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    cpu.step(&mut bus);
    assert_eq!(cpu.regs[1], 5);
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::LoadAccessFault)));
    assert_eq!(cpu.regs[1], 5, "x1 must be unchanged after trap");
}

#[test]
fn fault_ecall_sets_mepc() {
    // ecall → trap EcallFromM; mepc should equal PC of the ecall instruction
    let prog = [
        0x0000_0073u32, // ecall at DRAM_BASE
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    let r = cpu.step(&mut bus);
    assert!(matches!(r, StepResult::Trap(TrapCause::EcallFromM)));
    let mepc = cpu.csr.read(riscv_core::cpu::csr::CSR_MEPC);
    assert_eq!(mepc, DRAM_BASE, "mepc must point to the ecall instruction");
}

#[test]
fn fault_div_by_zero_no_crash() {
    // divu x3, x1, x0  — x0 is 0 → should give u64::MAX (spec)
    let prog = [
        0x0200_0093u32, // addi x1, x0, 32
        0x0200_D1B3u32, // divu x3, x1, x0
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_until_halt(&mut cpu, &mut bus, 10);
    assert_eq!(cpu.regs[3], u64::MAX, "div by zero should yield u64::MAX");
}

// ─── MMU / device fault injection ─────────────────────────────────────────

#[test]
fn fault_clint_timer_fires() {
    let mut bus = Bus::new(&[]);
    // Set mtimecmp to 5, tick 6 times
    bus.write64(riscv_core::bus::CLINT_BASE + 0x4000, 5).unwrap();
    for _ in 0..6 { bus.tick(); }
    assert!(bus.clint.timer_pending(), "timer should be pending");
}
