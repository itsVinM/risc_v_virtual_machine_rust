use crate::cpu::decoder::Inst;
use crate::cpu::csr::{CsrFile, CsrOp};
use crate::mmu::Mmu as Bus;
use crate::traps::TrapCause;

// Sign-extend a 32-bit value to 64 bits
#[inline(always)] fn sext32(x: u32) -> u64 { (x as i32) as i64 as u64 }

pub struct ExecResult {
    pub next_pc: u64,
    pub trap: Option<TrapCause>,
    pub halt: bool,
}

pub fn execute(
    inst: Inst,
    pc: u64,
    regs: &mut [u64; 32],
    csr: &mut CsrFile,
    bus: &mut Bus,
) -> ExecResult {
    let mut next_pc = pc.wrapping_add(4);
    let mut trap: Option<TrapCause> = None;
    let mut halt = false;

    macro_rules! reg { ($r:expr) => { regs[$r as usize] }; }
    macro_rules! set { ($r:expr, $v:expr) => { if $r != 0 { regs[$r as usize] = $v; } }; }

    match inst {
        Inst::Lui { rd, imm }   => set!(rd, imm as u64),
        Inst::Auipc { rd, imm } => set!(rd, pc.wrapping_add(imm as u64)),

        Inst::Jal { rd, imm } => {
            set!(rd, next_pc);
            next_pc = pc.wrapping_add(imm as u64);
        }
        Inst::Jalr { rd, rs1, imm } => {
            let t = next_pc;
            next_pc = (reg!(rs1).wrapping_add(imm as u64)) & !1;
            set!(rd, t);
        }

        Inst::Beq  { rs1, rs2, imm } => if reg!(rs1) == reg!(rs2) { next_pc = pc.wrapping_add(imm as u64); }
        Inst::Bne  { rs1, rs2, imm } => if reg!(rs1) != reg!(rs2) { next_pc = pc.wrapping_add(imm as u64); }
        Inst::Blt  { rs1, rs2, imm } => if (reg!(rs1) as i64) <  (reg!(rs2) as i64) { next_pc = pc.wrapping_add(imm as u64); }
        Inst::Bge  { rs1, rs2, imm } => if (reg!(rs1) as i64) >= (reg!(rs2) as i64) { next_pc = pc.wrapping_add(imm as u64); }
        Inst::Bltu { rs1, rs2, imm } => if reg!(rs1) <  reg!(rs2) { next_pc = pc.wrapping_add(imm as u64); }
        Inst::Bgeu { rs1, rs2, imm } => if reg!(rs1) >= reg!(rs2) { next_pc = pc.wrapping_add(imm as u64); }

        Inst::Lb  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read8(addr) {
                Ok(v) => set!(rd, v as i8 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lh  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read16(addr) {
                Ok(v) => set!(rd, v as i16 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lw  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read32(addr) {
                Ok(v) => set!(rd, sext32(v)),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Ld  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read64(addr) {
                Ok(v) => set!(rd, v),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lbu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read8(addr) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }
        Inst::Lhu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read16(addr) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }
        Inst::Lwu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            match bus.read32(addr) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }

        Inst::Sb { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            if let Err(e) = bus.write8(addr, reg!(rs2) as u8) { trap = Some(e); }
        }
        Inst::Sh { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            if let Err(e) = bus.write16(addr, reg!(rs2) as u16) { trap = Some(e); }
        }
        Inst::Sw { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            if let Err(e) = bus.write32(addr, reg!(rs2) as u32) { trap = Some(e); }
        }
        Inst::Sd { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            if let Err(e) = bus.write64(addr, reg!(rs2)) { trap = Some(e); }
        }

        Inst::Addi  { rd, rs1, imm } => set!(rd, reg!(rs1).wrapping_add(imm as u64)),
        Inst::Slti  { rd, rs1, imm } => set!(rd, ((reg!(rs1) as i64) < imm) as u64),
        Inst::Sltiu { rd, rs1, imm } => set!(rd, (reg!(rs1) < imm as u64) as u64),
        Inst::Xori  { rd, rs1, imm } => set!(rd, reg!(rs1) ^ imm as u64),
        Inst::Ori   { rd, rs1, imm } => set!(rd, reg!(rs1) | imm as u64),
        Inst::Andi  { rd, rs1, imm } => set!(rd, reg!(rs1) & imm as u64),
        Inst::Slli  { rd, rs1, shamt } => set!(rd, reg!(rs1) << shamt),
        Inst::Srli  { rd, rs1, shamt } => set!(rd, reg!(rs1) >> shamt),
        Inst::Srai  { rd, rs1, shamt } => set!(rd, ((reg!(rs1) as i64) >> shamt) as u64),

        Inst::Add  { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_add(reg!(rs2))),
        Inst::Sub  { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_sub(reg!(rs2))),
        Inst::Sll  { rd, rs1, rs2 } => set!(rd, reg!(rs1) << (reg!(rs2) & 63)),
        Inst::Slt  { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i64) < (reg!(rs2) as i64)) as u64),
        Inst::Sltu { rd, rs1, rs2 } => set!(rd, (reg!(rs1) < reg!(rs2)) as u64),
        Inst::Xor  { rd, rs1, rs2 } => set!(rd, reg!(rs1) ^ reg!(rs2)),
        Inst::Srl  { rd, rs1, rs2 } => set!(rd, reg!(rs1) >> (reg!(rs2) & 63)),
        Inst::Sra  { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i64) >> (reg!(rs2) & 63)) as u64),
        Inst::Or   { rd, rs1, rs2 } => set!(rd, reg!(rs1) | reg!(rs2)),
        Inst::And  { rd, rs1, rs2 } => set!(rd, reg!(rs1) & reg!(rs2)),

        // W-suffix: operate on low 32 bits, sign-extend result to 64
        Inst::Addiw { rd, rs1, imm } => set!(rd, sext32(reg!(rs1).wrapping_add(imm as u64) as u32)),
        Inst::Slliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) << shamt)),
        Inst::Srliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) >> shamt)),
        Inst::Sraiw { rd, rs1, shamt } => set!(rd, ((reg!(rs1) as i32) >> shamt) as i64 as u64),
        Inst::Addw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32).wrapping_add(reg!(rs2) as u32))),
        Inst::Subw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32).wrapping_sub(reg!(rs2) as u32))),
        Inst::Sllw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) << (reg!(rs2) & 31))),
        Inst::Srlw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) >> (reg!(rs2) & 31))),
        Inst::Sraw  { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i32) >> (reg!(rs2) & 31)) as i64 as u64),

        // M extension
        Inst::Mul    { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_mul(reg!(rs2))),
        Inst::Mulh   { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64 as i128;
            let b = reg!(rs2) as i64 as i128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Mulhsu { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64 as i128;
            let b = reg!(rs2) as u128 as i128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Mulhu  { rd, rs1, rs2 } => {
            let a = reg!(rs1) as u128;
            let b = reg!(rs2) as u128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Div  { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64; let b = reg!(rs2) as i64;
            set!(rd, if b == 0 { u64::MAX } else if a == i64::MIN && b == -1 { a as u64 } else { (a / b) as u64 });
        }
        Inst::Divu { rd, rs1, rs2 } => {
            let b = reg!(rs2);
            set!(rd, if b == 0 { u64::MAX } else { reg!(rs1) / b });
        }
        Inst::Rem  { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64; let b = reg!(rs2) as i64;
            set!(rd, if b == 0 { a as u64 } else if a == i64::MIN && b == -1 { 0 } else { (a % b) as u64 });
        }
        Inst::Remu { rd, rs1, rs2 } => {
            let b = reg!(rs2);
            set!(rd, if b == 0 { reg!(rs1) } else { reg!(rs1) % b });
        }
        Inst::Mulw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32).wrapping_mul(reg!(rs2) as u32))),
        Inst::Divw  { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i32; let b = reg!(rs2) as i32;
            set!(rd, if b == 0 { u64::MAX } else if a == i32::MIN && b == -1 { a as i64 as u64 } else { (a / b) as i64 as u64 });
        }
        Inst::Divuw { rd, rs1, rs2 } => {
            let b = reg!(rs2) as u32;
            set!(rd, if b == 0 { u64::MAX } else { sext32(reg!(rs1) as u32 / b) });
        }
        Inst::Remw  { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i32; let b = reg!(rs2) as i32;
            set!(rd, if b == 0 { a as i64 as u64 } else if a == i32::MIN && b == -1 { 0 } else { (a % b) as i64 as u64 });
        }
        Inst::Remuw { rd, rs1, rs2 } => {
            let b = reg!(rs2) as u32;
            set!(rd, if b == 0 { sext32(reg!(rs1) as u32) } else { sext32(reg!(rs1) as u32 % b) });
        }

        Inst::Ecall  => trap = Some(TrapCause::EcallFromM),
        Inst::Ebreak => { halt = true; }
        Inst::Fence  => { /* no-op for single-core */ }
        Inst::Mret   => {
            // Restore PC from mepc, restore MIE from MPIE
            next_pc = csr.read(crate::cpu::csr::CSR_MEPC);
            let mut ms = csr.mstatus();
            let mpie = (ms >> 7) & 1;
            ms = (ms & !(1 << 3)) | (mpie << 3); // MIE = MPIE
            ms |= 1 << 7;                          // MPIE = 1
            ms = (ms & !0x1800) | 0x0000;          // MPP = U (0)
            csr.write(crate::cpu::csr::CSR_MSTATUS, ms);
        }

        Inst::Csrrw  { rd, rs1, csr: addr } => {
            let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Write);
            set!(rd, v);
        }
        Inst::Csrrs  { rd, rs1, csr: addr } => {
            let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Set);
            set!(rd, v);
        }
        Inst::Csrrc  { rd, rs1, csr: addr } => {
            let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Clear);
            set!(rd, v);
        }
        Inst::Csrrwi { rd, uimm, csr: addr } => {
            let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Write);
            set!(rd, v);
        }
        Inst::Csrrsi { rd, uimm, csr: addr } => {
            let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Set);
            set!(rd, v);
        }
        Inst::Csrrci { rd, uimm, csr: addr } => {
            let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Clear);
            set!(rd, v);
        }

        Inst::Illegal(raw) => trap = Some(TrapCause::IllegalInstruction(raw)),
    }

    ExecResult { next_pc, trap, halt }
}
