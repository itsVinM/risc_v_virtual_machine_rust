use crate::cpu::decoder::Inst;
use crate::cpu::csr::{
    CsrFile, CsrOp, Privilege, csr_access_ok,
    CSR_MEPC, CSR_MSTATUS, CSR_SATP, CSR_SEPC,
    MSTATUS_MIE, MSTATUS_MPP, MSTATUS_MPIE,
    MSTATUS_SIE, MSTATUS_SPP, MSTATUS_SPIE,
    SATP_PPN, SATP_MODE_BARE, SATP_MODE_SV39,
};
use crate::mmu::Mmu as Bus;
use crate::traps::TrapCause;

fn sext32(x: u32) -> u64 { (x as i32) as i64 as u64 }

pub struct ExecResult {
    pub next_pc: u64,
    pub trap: Option<TrapCause>,
    pub halt: bool,
    pub new_priv: Option<Privilege>,
}

fn satp_ppn(satp: u64) -> u64 { satp & SATP_PPN }
fn satp_mode(satp: u64) -> u64 { (satp >> 60) & 0xF }
fn pte_ppn(pte: u64) -> u64 { (pte >> 10) & 0x00FF_FFFF_FFFF }

fn csr_satp(csr: &CsrFile) -> u64 { csr.read(CSR_SATP, Privilege::M) }
fn csr_mstatus(csr: &CsrFile) -> u64 { csr.mstatus() }

const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const _PTE_A: u64 = 1 << 6;
const _PTE_D: u64 = 1 << 7;

fn translate(va: u64, priv_level: Privilege, bus: &Bus, satp: u64, mstatus: u64, access_type: AccessType) -> Result<u64, TrapCause> {
    if priv_level == Privilege::M {
        return Ok(va);
    }
    if satp_mode(satp) == SATP_MODE_BARE {
        return Ok(va);
    }
    if satp_mode(satp) != SATP_MODE_SV39 {
        return Err(TrapCause::IllegalInstruction(0));
    }

    let sum = (mstatus >> 18) & 1;
    let mxr = (mstatus >> 19) & 1;

    let vpn = |level: u64| -> u64 { (va >> (12 + level * 9)) & 0x1FF };
    let root_ppn = satp_ppn(satp);

    let mut pte_addr = (root_ppn << 12) + (vpn(2) << 3);
    let mut pte = bus.read_phys64(pte_addr).map_err(|_| TrapCause::LoadPageFault)?;

    if pte & 1 == 0 {
        return Err(TrapCause::LoadPageFault);
    }

    for level in (0..=1).rev() {
        if pte & (PTE_R | PTE_W | PTE_X) != 0 {
            break;
        }
        let ppn = pte_ppn(pte);
        pte_addr = (ppn << 12) + (vpn(level) << 3);
        pte = bus.read_phys64(pte_addr).map_err(|_| TrapCause::LoadPageFault)?;
        if pte & 1 == 0 {
            return Err(TrapCause::LoadPageFault);
        }
    }

    if pte & (PTE_R | PTE_W | PTE_X) == 0 {
        return Err(TrapCause::LoadPageFault);
    }

    // Permission check with SUM and MXR
    let user_page = (pte & PTE_U) != 0;
    let readable = pte & (PTE_R | PTE_X) != 0 || (mxr != 0 && pte & PTE_X != 0);
    let writable = pte & (PTE_R | PTE_W) == (PTE_R | PTE_W);

    if priv_level == Privilege::U && !user_page {
        return Err(TrapCause::LoadPageFault);
    }
    if priv_level == Privilege::S && user_page && sum == 0 {
        return Err(TrapCause::LoadPageFault);
    }

    match access_type {
        AccessType::Read if !readable => return Err(TrapCause::LoadPageFault),
        AccessType::Write if !writable => return Err(TrapCause::StorePageFault),
        _ => {}
    }

    let ppn = pte_ppn(pte);
    let offset = va & 0xFFF;
    let pa = (ppn << 12) | offset;

    Ok(pa)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccessType { Read, Write }

pub fn execute(
    inst: Inst,
    pc: u64,
    regs: &mut [u64; 32],
    csr: &mut CsrFile,
    bus: &mut Bus,
    priv_level: Privilege,
) -> ExecResult {
    let mut next_pc = pc.wrapping_add(4);
    let mut trap: Option<TrapCause> = None;
    let mut halt = false;
    let mut new_priv: Option<Privilege> = None;

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
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read8(pa) {
                Ok(v) => set!(rd, v as i8 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lh  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read16(pa) {
                Ok(v) => set!(rd, v as i16 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lw  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read32(pa) {
                Ok(v) => set!(rd, sext32(v)),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Ld  { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read64(pa) {
                Ok(v) => set!(rd, v),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lbu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read8(pa) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }
        Inst::Lhu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read16(pa) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }
        Inst::Lwu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read32(pa) { Ok(v) => set!(rd, v as u64), Err(e) => trap = Some(e) }
        }

        Inst::Sb { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Err(e) = bus.write8(pa, reg!(rs2) as u8) { trap = Some(e); }
        }
        Inst::Sh { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Err(e) = bus.write16(pa, reg!(rs2) as u16) { trap = Some(e); }
        }
        Inst::Sw { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Err(e) = bus.write32(pa, reg!(rs2) as u32) { trap = Some(e); }
        }
        Inst::Sd { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Err(e) = bus.write64(pa, reg!(rs2)) { trap = Some(e); }
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

        Inst::Addiw { rd, rs1, imm } => set!(rd, sext32(reg!(rs1).wrapping_add(imm as u64) as u32)),
        Inst::Slliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) << shamt)),
        Inst::Srliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) >> shamt)),
        Inst::Sraiw { rd, rs1, shamt } => set!(rd, ((reg!(rs1) as i32) >> shamt) as i64 as u64),
        Inst::Addw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32).wrapping_add(reg!(rs2) as u32))),
        Inst::Subw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32).wrapping_sub(reg!(rs2) as u32))),
        Inst::Sllw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) << (reg!(rs2) & 31))),
        Inst::Srlw  { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) >> (reg!(rs2) & 31))),
        Inst::Sraw  { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i32) >> (reg!(rs2) & 31)) as i64 as u64),

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

        Inst::AmoaddW { rd, rs1, rs2, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Ok(old) = bus.read32(pa) {
                let sum = (old as u32).wrapping_add(reg!(rs2) as u32);
                if bus.write32(pa, sum).is_ok() {
                    set!(rd, old as i32 as i64 as u64);
                } else {
                    trap = Some(TrapCause::StoreAccessFault);
                }
            } else {
                trap = Some(TrapCause::LoadAccessFault);
            }
        }
        Inst::AmoswapW { rd, rs1, rs2, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if let Ok(old) = bus.read32(pa) {
                if bus.write32(pa, reg!(rs2) as u32).is_ok() {
                    set!(rd, old as i32 as i64 as u64);
                } else {
                    trap = Some(TrapCause::StoreAccessFault);
                }
            } else {
                trap = Some(TrapCause::LoadAccessFault);
            }
        }
        Inst::LrW { rd, rs1, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read32(pa) {
                Ok(v) => set!(rd, v as i32 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::ScW { rd, rs1, rs2, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if bus.write32(pa, reg!(rs2) as u32).is_ok() {
                set!(rd, 0);
            } else {
                trap = Some(TrapCause::StoreAccessFault);
            }
        }
        Inst::LrD { rd, rs1, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Read).unwrap_or(addr);
            match bus.read64(pa) {
                Ok(v) => set!(rd, v),
                Err(e) => trap = Some(e),
            }
        }
        Inst::ScD { rd, rs1, rs2, aq: _, rl: _ } => {
            let addr = reg!(rs1);
            let pa = translate(addr, priv_level, bus, csr_satp(csr), csr_mstatus(csr), AccessType::Write).unwrap_or(addr);
            if bus.write64(pa, reg!(rs2)).is_ok() {
                set!(rd, 0);
            } else {
                trap = Some(TrapCause::StoreAccessFault);
            }
        }

        Inst::Ecall  => {
            let cause = match priv_level {
                Privilege::U => TrapCause::EcallFromU,
                Privilege::S => TrapCause::EcallFromS,
                Privilege::M => TrapCause::EcallFromM,
            };
            trap = Some(cause);
        }
        Inst::Ebreak => { halt = true; }
        Inst::Fence  => {}

        Inst::Mret => {
            if priv_level != Privilege::M {
                trap = Some(TrapCause::IllegalInstruction(0x30200073));
            } else {
                next_pc = csr.read(CSR_MEPC, Privilege::M);
                let ms = csr.mstatus();
                let mpp = (ms >> 11) & 0b11;
                let mpie = (ms >> 7) & 1;
                let mut new_ms = ms & !(MSTATUS_MIE | MSTATUS_MPP | MSTATUS_MPIE);
                new_ms |= mpie << 3;      // MIE = MPIE
                new_ms |= 1 << 7;          // MPIE = 1
                new_ms |= mpp << 11;       // preserve MPP for priv update in step()
                csr.write(CSR_MSTATUS, new_ms, Privilege::M);
                new_priv = Some(match mpp { 1 => Privilege::S, 0 => Privilege::U, _ => Privilege::M });
            }
        }

        Inst::Sret => {
            let spp = (csr.mstatus() >> 8) & 1;
            let spie = (csr.mstatus() >> 5) & 1;
            let mut new_ms = csr.mstatus();
            new_ms &= !(MSTATUS_SIE | MSTATUS_SPP | MSTATUS_SPIE);
            new_ms |= spie << 1;       // SIE = SPIE
            new_ms |= 1 << 5;          // SPIE = 1
            new_ms |= spp << 8;        // preserve SPP for priv update
            if priv_level < Privilege::S {
                trap = Some(TrapCause::IllegalInstruction(0x10200073));
            } else {
                next_pc = csr.read(CSR_SEPC, priv_level);
                csr.write(CSR_MSTATUS, new_ms, Privilege::M);
                new_priv = Some(if spp == 1 { Privilege::S } else { Privilege::U });
            }
        }

        Inst::Csrrw  { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Write, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrs  { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Set, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrc  { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Clear, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrwi { rd, uimm, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Write, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrsi { rd, uimm, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Set, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrci { rd, uimm, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Clear, priv_level);
                set!(rd, v);
            }
        }

        Inst::Illegal(raw) => trap = Some(TrapCause::IllegalInstruction(raw)),
    }

    ExecResult { next_pc, trap, halt, new_priv }
}
