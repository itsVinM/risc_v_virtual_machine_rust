use riscv_core::{Bus, Cpu};
use riscv_core::cpu::StepResult;
use riscv_core::cpu::decoder::{decode, Inst};
use riscv_core::bus::DRAM_BASE;

// ─── Helper: assemble a small program into DRAM and run for N steps ────────

fn make_vm(words: &[u32]) -> (Cpu, Bus) {
    let bytes: Vec<u8> = words.iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let bus = Bus::new(&bytes);
    let cpu = Cpu::new(DRAM_BASE);
    (cpu, bus)
}

fn run_n(cpu: &mut Cpu, bus: &mut Bus, n: usize) {
    for _ in 0..n {
        match cpu.step(bus) {
            StepResult::Halted | StepResult::Trap(_) => break,
            StepResult::Ok => {}
        }
    }
}

// ─── Decoder unit tests ────────────────────────────────────────────────────

#[test]
fn decode_lui() {
    // lui x1, 0xDEAD
    let raw = 0xDEAD_00B7u32;
    assert!(matches!(decode(raw), Inst::Lui { rd: 1, .. }));
}

#[test]
fn decode_addi() {
    // addi x1, x0, 42
    let raw = 0x02A0_0093u32;
    match decode(raw) {
        Inst::Addi { rd: 1, rs1: 0, imm: 42 } => {}
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn decode_jal() {
    // jal x0, 0  (infinite loop)
    let raw = 0x0000_006Fu32;
    assert!(matches!(decode(raw), Inst::Jal { rd: 0, imm: 0 }));
}

#[test]
fn decode_branch() {
    // beq x0, x0, +8
    let raw = 0x0000_0463u32;
    match decode(raw) {
        Inst::Beq { rs1: 0, rs2: 0, imm: 8 } => {}
        other => panic!("unexpected: {:?}", other),
    }
}

#[test]
fn decode_illegal() {
    let raw = 0xFFFF_FFFFu32;
    assert!(matches!(decode(raw), Inst::Illegal(_)));
}

// ─── Execution unit tests ──────────────────────────────────────────────────

#[test]
fn exec_lui_auipc() {
    // lui x1, 1      → x1 = 0x1000
    // auipc x2, 1    → x2 = PC + 0x1000
    let prog = [
        0x0000_10B7u32, // lui x1, 1
        0x0000_1117u32, // auipc x2, 1
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 3);
    assert_eq!(cpu.regs[1], 0x1000);
    assert_eq!(cpu.regs[2], DRAM_BASE + 4 + 0x1000);
}

#[test]
fn exec_addi_chain() {
    // addi x1, x0, 10
    // addi x1, x1, 10
    // addi x1, x1, 10
    // ebreak
    let prog = [
        0x00A0_0093u32, // addi x1, x0, 10
        0x00A0_8093u32, // addi x1, x1, 10
        0x00A0_8093u32, // addi x1, x1, 10
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 4);
    assert_eq!(cpu.regs[1], 30);
}

#[test]
fn exec_x0_immutable() {
    // addi x0, x0, 99  → x0 must stay 0
    let prog = [0x0630_0013u32, 0x0010_0073u32];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 2);
    assert_eq!(cpu.regs[0], 0);
}

#[test]
fn exec_add_sub() {
    // addi x1, x0, 100
    // addi x2, x0, 37
    // add  x3, x1, x2   → 137
    // sub  x4, x1, x2   → 63
    // ebreak
    let prog = [
        0x0640_0093u32, // addi x1, x0, 100
        0x0250_0113u32, // addi x2, x0, 37
        0x0020_81B3u32, // add  x3, x1, x2
        0x4020_8233u32, // sub  x4, x1, x2
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 5);
    assert_eq!(cpu.regs[3], 137);
    assert_eq!(cpu.regs[4], 63);
}

#[test]
fn exec_logical_ops() {
    // addi x1, x0, 0xFF
    // addi x2, x0, 0x0F
    // and  x3, x1, x2   → 0x0F
    // or   x4, x1, x2   → 0xFF
    // xor  x5, x1, x2   → 0xF0
    let prog = [
        0x0FF0_0093u32, // addi x1, x0, 255
        0x00F0_0113u32, // addi x2, x0, 15
        0x0020_F1B3u32, // and  x3, x1, x2
        0x0020_E233u32, // or   x4, x1, x2
        0x0020_C2B3u32, // xor  x5, x1, x2
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 6);
    assert_eq!(cpu.regs[3], 0x0F);
    assert_eq!(cpu.regs[4], 0xFF);
    assert_eq!(cpu.regs[5], 0xF0);
}

#[test]
fn exec_shifts() {
    // addi x1, x0, 1
    // slli x2, x1, 10   → 1024
    // srli x3, x2, 2    → 256
    // srai x4, x2, 2    → 256 (positive, so same)
    let prog = [
        0x0010_0093u32, // addi x1, x0, 1
        0x00A0_9113u32, // slli x2, x1, 10
        0x0021_5193u32, // srli x3, x2, 2
        0x4021_5213u32, // srai x4, x2, 2
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 5);
    assert_eq!(cpu.regs[2], 1024);
    assert_eq!(cpu.regs[3], 256);
    assert_eq!(cpu.regs[4], 256);
}

#[test]
fn exec_load_store_word() {
    // Store 0xDEAD_BEEF to DRAM+100, then load it back
    // lui  x1, 0xDEADB        -- upper
    // addi x1, x1, -0x411     -- lower (sign-adjust for DEADBEEF)
    // addi x2, x0, 100
    // add  x2, x2, sp (sp = DRAM_END but we'll use absolute)
    // For simplicity: store at DRAM_BASE directly
    // addi x3, x0, 0x42       -- value
    // sw   x3, 0(sp)  (sp points to DRAM_END, let's adjust)
    // Actually simpler: use relative offset within the default sp (DRAM_END)
    // We just check sw+lw roundtrip at a known address.
    let addr = DRAM_BASE + 0x100;
    let mut words = vec![0u32; 256];
    // addi x1, x0, 0x42
    words[0] = 0x0420_0093u32;
    // sw x1, 0x10(x0) — but x0 is 0, addr=0x10, not in DRAM...
    // Use sp (x2) which is DRAM_END
    // addi x2, x2, -8  (sp -= 8, into valid DRAM)
    words[1] = 0xFF81_0113u32; // addi x2, x2, -8
    // sw x1, 0(x2)
    words[2] = 0x0011_2023u32; // sw x1, 0(x2)
    // lw x3, 0(x2)
    words[3] = 0x0001_2183u32; // lw x3, 0(x2)
    // ebreak
    words[4] = 0x0010_0073u32;

    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut bus = Bus::new(&bytes);
    let mut cpu = Cpu::new(DRAM_BASE);
    run_n(&mut cpu, &mut bus, 5);
    assert_eq!(cpu.regs[1], 0x42);
    assert_eq!(cpu.regs[3], 0x42);
}

#[test]
fn exec_jal_jalr() {
    // jal x1, +8   → jump over next instruction, x1 = PC+4
    // addi x7, x0, 0xFF  (should be skipped - use t2/x7 to avoid clobbering sp/x2)
    // addi x3, x0, 0x42  (should execute)
    // ebreak
    // jal x1, +8: opcode=0x6F, rd=1, imm=8
    // J-type: imm[20|10:1|11|19:12]=0x00800, rd=00001, op=1101111
    // = 0000_0000_1000_0000_0000_0000_1110_1111 = 0x008000EF
    let prog = [
        0x0080_00EFu32, // jal x1, +8
        0x0FF0_0393u32, // addi x7, x0, 255  (skipped)
        0x0420_0193u32, // addi x3, x0, 66
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 4);
    assert_eq!(cpu.regs[7], 0); // x7 was never written (instruction skipped)
    assert_eq!(cpu.regs[3], 0x42);
    assert_eq!(cpu.regs[1], DRAM_BASE + 4); // return address
}

#[test]
fn exec_beq_taken() {
    // addi x1, x0, 5
    // addi x2, x0, 5
    // beq  x1, x2, +8   (taken)
    // addi x3, x0, 0xFF (skipped)
    // addi x4, x0, 1    (executed)
    // ebreak
    let prog = [
        0x0050_0093u32, // addi x1, x0, 5
        0x0050_0113u32, // addi x2, x0, 5
        0x0020_8463u32, // beq  x1, x2, +8
        0x0FF0_0193u32, // addi x3, x0, 255 (skipped)
        0x0010_0213u32, // addi x4, x0, 1
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 6);
    assert_eq!(cpu.regs[3], 0); // skipped
    assert_eq!(cpu.regs[4], 1);
}

#[test]
fn exec_slt_sltu() {
    // slt:  -1 < 1 (signed) → 1
    // sltu: -1 < 1 (unsigned, -1 = MAX) → 0
    let prog = [
        0xFFF0_0093u32, // addi x1, x0, -1
        0x0010_0113u32, // addi x2, x0, 1
        0x0020_A1B3u32, // slt  x3, x1, x2  → 1
        0x0020_B233u32, // sltu x4, x1, x2  → 0
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 5);
    assert_eq!(cpu.regs[3], 1);
    assert_eq!(cpu.regs[4], 0);
}

#[test]
fn exec_mul_div() {
    // mul  x3, x1(6), x2(7) → 42
    // div  x4, x1(6), x2(7) → 0
    // rem  x5, x3(42), x2(7) → 0
    let prog = [
        0x0060_0093u32, // addi x1, x0, 6
        0x0070_0113u32, // addi x2, x0, 7
        0x0220_81B3u32, // mul  x3, x1, x2
        0x0220_C233u32, // div  x4, x1, x2
        0x0221_62B3u32, // rem  x5, x2, x2  (7%7=0)
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 6);
    assert_eq!(cpu.regs[3], 42);
    assert_eq!(cpu.regs[4], 0);
    assert_eq!(cpu.regs[5], 0);
}

#[test]
fn exec_word_ops_sign_extend() {
    // addiw x1, x0, -1  → x1 = 0xFFFFFFFFFFFFFFFF (sign extended)
    // Encoding: imm=0xFFF, rs1=x0, funct3=0, rd=x1, opcode=0x1B
    // = 1111_1111_1111_0000_0000_0000_1001_1011 = 0xFFF0_009B
    let prog = [
        0xFFF0_009Bu32, // addiw x1, x0, -1
        0x0010_0073u32, // ebreak
    ];
    let (mut cpu, mut bus) = make_vm(&prog);
    run_n(&mut cpu, &mut bus, 2);
    assert_eq!(cpu.regs[1], u64::MAX);
}
