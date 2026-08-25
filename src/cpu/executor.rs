use crate::cpu::csr::{
    csr_access_ok, CsrFile, CsrOp, Privilege, CSR_MEPC, CSR_MSTATUS, CSR_SATP, CSR_SEPC,
    MSTATUS_MIE, MSTATUS_MPIE, MSTATUS_MPP, MSTATUS_SIE, MSTATUS_SPIE, MSTATUS_SPP, SATP_MODE_BARE,
    SATP_MODE_SV39, SATP_PPN,
};
use crate::cpu::decoder::Inst;
use crate::mmu::Mmu as Bus;
use crate::traps::TrapCause;

fn sext32(x: u32) -> u64 {
    (x as i32) as i64 as u64
}

pub struct ExecResult {
    pub next_pc: u64,
    pub trap: Option<TrapCause>,
    pub halt: bool,
    pub new_priv: Option<Privilege>,
}

fn satp_ppn(satp: u64) -> u64 {
    satp & SATP_PPN
}
fn satp_mode(satp: u64) -> u64 {
    (satp >> 60) & 0xF
}
fn pte_ppn(pte: u64) -> u64 {
    (pte >> 10) & 0x00FF_FFFF_FFFF
}

fn csr_satp(csr: &CsrFile) -> u64 {
    csr.read(CSR_SATP, Privilege::M)
}
fn csr_mstatus(csr: &CsrFile) -> u64 {
    csr.mstatus()
}

const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const _PTE_A: u64 = 1 << 6;
const _PTE_D: u64 = 1 << 7;

fn translate(
    va: u64,
    priv_level: Privilege,
    bus: &Bus,
    satp: u64,
    mstatus: u64,
    access_type: AccessType,
) -> Result<u64, TrapCause> {
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
    let mut pte = bus
        .read_phys64(pte_addr)
        .map_err(|_| TrapCause::LoadPageFault)?;

    if pte & 1 == 0 {
        return Err(TrapCause::LoadPageFault);
    }

    // Level at which the leaf PTE was found (2 = gigapage, 1 = megapage,
    // 0 = 4 KiB page). Needed to merge VA index bits into the final PA.
    let mut leaf_level = 2u64;
    for level in (0..=1).rev() {
        if pte & (PTE_R | PTE_W | PTE_X) != 0 {
            break;
        }
        let ppn = pte_ppn(pte);
        pte_addr = (ppn << 12) + (vpn(level) << 3);
        pte = bus
            .read_phys64(pte_addr)
            .map_err(|_| TrapCause::LoadPageFault)?;
        if pte & 1 == 0 {
            return Err(TrapCause::LoadPageFault);
        }
        leaf_level = level;
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
    let va_low_bits = 12 + leaf_level * 9;
    let pa = ((ppn << 12) & !((1u64 << va_low_bits) - 1)) | (va & ((1u64 << va_low_bits) - 1));

    Ok(pa)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccessType {
    Read,
    Write,
}

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

    macro_rules! reg {
        ($r:expr) => {
            regs[$r as usize]
        };
    }
    macro_rules! set {
        ($r:expr, $v:expr) => {
            if $r != 0 {
                regs[$r as usize] = $v;
            }
        };
    }

    match inst {
        Inst::Lui { rd, imm } => set!(rd, imm as u64),
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

        Inst::Beq { rs1, rs2, imm } => {
            if reg!(rs1) == reg!(rs2) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }
        Inst::Bne { rs1, rs2, imm } => {
            if reg!(rs1) != reg!(rs2) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }
        Inst::Blt { rs1, rs2, imm } => {
            if (reg!(rs1) as i64) < (reg!(rs2) as i64) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }
        Inst::Bge { rs1, rs2, imm } => {
            if (reg!(rs1) as i64) >= (reg!(rs2) as i64) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }
        Inst::Bltu { rs1, rs2, imm } => {
            if reg!(rs1) < reg!(rs2) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }
        Inst::Bgeu { rs1, rs2, imm } => {
            if reg!(rs1) >= reg!(rs2) {
                next_pc = pc.wrapping_add(imm as u64);
            }
        }

        Inst::Lb { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read8(pa) {
                Ok(v) => set!(rd, v as i8 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lh { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read16(pa) {
                Ok(v) => set!(rd, v as i16 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lw { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read32(pa) {
                Ok(v) => set!(rd, sext32(v)),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Ld { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read64(pa) {
                Ok(v) => set!(rd, v),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lbu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read8(pa) {
                Ok(v) => set!(rd, v as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lhu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read16(pa) {
                Ok(v) => set!(rd, v as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::Lwu { rd, rs1, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read32(pa) {
                Ok(v) => set!(rd, v as u64),
                Err(e) => trap = Some(e),
            }
        }

        Inst::Sb { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if let Err(e) = bus.write8(pa, reg!(rs2) as u8) {
                trap = Some(e);
            }
        }
        Inst::Sh { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if let Err(e) = bus.write16(pa, reg!(rs2) as u16) {
                trap = Some(e);
            }
        }
        Inst::Sw { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if let Err(e) = bus.write32(pa, reg!(rs2) as u32) {
                trap = Some(e);
            }
        }
        Inst::Sd { rs1, rs2, imm } => {
            let addr = reg!(rs1).wrapping_add(imm as u64);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if let Err(e) = bus.write64(pa, reg!(rs2)) {
                trap = Some(e);
            }
        }

        Inst::Addi { rd, rs1, imm } => set!(rd, reg!(rs1).wrapping_add(imm as u64)),
        Inst::Slti { rd, rs1, imm } => set!(rd, ((reg!(rs1) as i64) < imm) as u64),
        Inst::Sltiu { rd, rs1, imm } => set!(rd, (reg!(rs1) < imm as u64) as u64),
        Inst::Xori { rd, rs1, imm } => set!(rd, reg!(rs1) ^ imm as u64),
        Inst::Ori { rd, rs1, imm } => set!(rd, reg!(rs1) | imm as u64),
        Inst::Andi { rd, rs1, imm } => set!(rd, reg!(rs1) & imm as u64),
        Inst::Slli { rd, rs1, shamt } => set!(rd, reg!(rs1) << shamt),
        Inst::Srli { rd, rs1, shamt } => set!(rd, reg!(rs1) >> shamt),
        Inst::Srai { rd, rs1, shamt } => set!(rd, ((reg!(rs1) as i64) >> shamt) as u64),

        Inst::Add { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_add(reg!(rs2))),
        Inst::Sub { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_sub(reg!(rs2))),
        Inst::Sll { rd, rs1, rs2 } => set!(rd, reg!(rs1) << (reg!(rs2) & 63)),
        Inst::Slt { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i64) < (reg!(rs2) as i64)) as u64),
        Inst::Sltu { rd, rs1, rs2 } => set!(rd, (reg!(rs1) < reg!(rs2)) as u64),
        Inst::Xor { rd, rs1, rs2 } => set!(rd, reg!(rs1) ^ reg!(rs2)),
        Inst::Srl { rd, rs1, rs2 } => set!(rd, reg!(rs1) >> (reg!(rs2) & 63)),
        Inst::Sra { rd, rs1, rs2 } => set!(rd, ((reg!(rs1) as i64) >> (reg!(rs2) & 63)) as u64),
        Inst::Or { rd, rs1, rs2 } => set!(rd, reg!(rs1) | reg!(rs2)),
        Inst::And { rd, rs1, rs2 } => set!(rd, reg!(rs1) & reg!(rs2)),

        Inst::Addiw { rd, rs1, imm } => set!(rd, sext32(reg!(rs1).wrapping_add(imm as u64) as u32)),
        Inst::Slliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) << shamt)),
        Inst::Srliw { rd, rs1, shamt } => set!(rd, sext32((reg!(rs1) as u32) >> shamt)),
        Inst::Sraiw { rd, rs1, shamt } => set!(rd, ((reg!(rs1) as i32) >> shamt) as i64 as u64),
        Inst::Addw { rd, rs1, rs2 } => set!(
            rd,
            sext32((reg!(rs1) as u32).wrapping_add(reg!(rs2) as u32))
        ),
        Inst::Subw { rd, rs1, rs2 } => set!(
            rd,
            sext32((reg!(rs1) as u32).wrapping_sub(reg!(rs2) as u32))
        ),
        Inst::Sllw { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) << (reg!(rs2) & 31))),
        Inst::Srlw { rd, rs1, rs2 } => set!(rd, sext32((reg!(rs1) as u32) >> (reg!(rs2) & 31))),
        Inst::Sraw { rd, rs1, rs2 } => {
            set!(rd, ((reg!(rs1) as i32) >> (reg!(rs2) & 31)) as i64 as u64)
        }

        Inst::Mul { rd, rs1, rs2 } => set!(rd, reg!(rs1).wrapping_mul(reg!(rs2))),
        Inst::Mulh { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64 as i128;
            let b = reg!(rs2) as i64 as i128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Mulhsu { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64 as i128;
            let b = reg!(rs2) as u128 as i128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Mulhu { rd, rs1, rs2 } => {
            let a = reg!(rs1) as u128;
            let b = reg!(rs2) as u128;
            set!(rd, ((a * b) >> 64) as u64);
        }
        Inst::Div { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64;
            let b = reg!(rs2) as i64;
            set!(
                rd,
                if b == 0 {
                    u64::MAX
                } else if a == i64::MIN && b == -1 {
                    a as u64
                } else {
                    (a / b) as u64
                }
            );
        }
        Inst::Divu { rd, rs1, rs2 } => {
            let b = reg!(rs2);
            set!(rd, reg!(rs1).checked_div(b).unwrap_or(u64::MAX));
        }
        Inst::Rem { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i64;
            let b = reg!(rs2) as i64;
            set!(
                rd,
                if b == 0 {
                    a as u64
                } else if a == i64::MIN && b == -1 {
                    0
                } else {
                    (a % b) as u64
                }
            );
        }
        Inst::Remu { rd, rs1, rs2 } => {
            let b = reg!(rs2);
            set!(rd, if b == 0 { reg!(rs1) } else { reg!(rs1) % b });
        }
        Inst::Mulw { rd, rs1, rs2 } => set!(
            rd,
            sext32((reg!(rs1) as u32).wrapping_mul(reg!(rs2) as u32))
        ),
        Inst::Divw { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i32;
            let b = reg!(rs2) as i32;
            set!(
                rd,
                if b == 0 {
                    u64::MAX
                } else if a == i32::MIN && b == -1 {
                    a as i64 as u64
                } else {
                    (a / b) as i64 as u64
                }
            );
        }
        Inst::Divuw { rd, rs1, rs2 } => {
            let b = reg!(rs2) as u32;
            let a = reg!(rs1) as u32;
            set!(rd, sext32(a.checked_div(b).unwrap_or(u32::MAX)));
        }
        Inst::Remw { rd, rs1, rs2 } => {
            let a = reg!(rs1) as i32;
            let b = reg!(rs2) as i32;
            set!(
                rd,
                if b == 0 {
                    a as i64 as u64
                } else if a == i32::MIN && b == -1 {
                    0
                } else {
                    (a % b) as i64 as u64
                }
            );
        }
        Inst::Remuw { rd, rs1, rs2 } => {
            let b = reg!(rs2) as u32;
            set!(
                rd,
                if b == 0 {
                    sext32(reg!(rs1) as u32)
                } else {
                    sext32(reg!(rs1) as u32 % b)
                }
            );
        }

        Inst::AmoaddW {
            rd,
            rs1,
            rs2,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if let Ok(old) = bus.read32(pa) {
                let sum = old.wrapping_add(reg!(rs2) as u32);
                if bus.write32(pa, sum).is_ok() {
                    set!(rd, old as i32 as i64 as u64);
                } else {
                    trap = Some(TrapCause::StoreAccessFault);
                }
            } else {
                trap = Some(TrapCause::LoadAccessFault);
            }
        }
        Inst::AmoswapW {
            rd,
            rs1,
            rs2,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
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
        Inst::LrW {
            rd,
            rs1,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read32(pa) {
                Ok(v) => set!(rd, v as i32 as i64 as u64),
                Err(e) => trap = Some(e),
            }
        }
        Inst::ScW {
            rd,
            rs1,
            rs2,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if bus.write32(pa, reg!(rs2) as u32).is_ok() {
                set!(rd, 0);
            } else {
                trap = Some(TrapCause::StoreAccessFault);
            }
        }
        Inst::LrD {
            rd,
            rs1,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Read,
            )
            .unwrap_or(addr);
            match bus.read64(pa) {
                Ok(v) => set!(rd, v),
                Err(e) => trap = Some(e),
            }
        }
        Inst::ScD {
            rd,
            rs1,
            rs2,
            aq: _,
            rl: _,
        } => {
            let addr = reg!(rs1);
            let pa = translate(
                addr,
                priv_level,
                bus,
                csr_satp(csr),
                csr_mstatus(csr),
                AccessType::Write,
            )
            .unwrap_or(addr);
            if bus.write64(pa, reg!(rs2)).is_ok() {
                set!(rd, 0);
            } else {
                trap = Some(TrapCause::StoreAccessFault);
            }
        }

        Inst::Ecall => {
            let cause = match priv_level {
                Privilege::U => TrapCause::EcallFromU,
                Privilege::S => TrapCause::EcallFromS,
                Privilege::M => TrapCause::EcallFromM,
            };
            trap = Some(cause);
        }
        Inst::Ebreak => {
            halt = true;
        }
        Inst::Fence => {}

        Inst::Mret => {
            if priv_level != Privilege::M {
                trap = Some(TrapCause::IllegalInstruction(0x30200073));
            } else {
                next_pc = csr.read(CSR_MEPC, Privilege::M);
                let ms = csr.mstatus();
                let mpp = (ms >> 11) & 0b11;
                let mpie = (ms >> 7) & 1;
                let mut new_ms = ms & !(MSTATUS_MIE | MSTATUS_MPP | MSTATUS_MPIE);
                new_ms |= mpie << 3; // MIE = MPIE
                new_ms |= 1 << 7; // MPIE = 1
                new_ms |= mpp << 11; // preserve MPP for priv update in step()
                csr.write(CSR_MSTATUS, new_ms, Privilege::M);
                new_priv = Some(match mpp {
                    1 => Privilege::S,
                    0 => Privilege::U,
                    _ => Privilege::M,
                });
            }
        }

        Inst::Sret => {
            let spp = (csr.mstatus() >> 8) & 1;
            let spie = (csr.mstatus() >> 5) & 1;
            let mut new_ms = csr.mstatus();
            new_ms &= !(MSTATUS_SIE | MSTATUS_SPP | MSTATUS_SPIE);
            new_ms |= spie << 1; // SIE = SPIE
            new_ms |= 1 << 5; // SPIE = 1
            new_ms |= spp << 8; // preserve SPP for priv update
            if priv_level < Privilege::S {
                trap = Some(TrapCause::IllegalInstruction(0x10200073));
            } else {
                next_pc = csr.read(CSR_SEPC, priv_level);
                csr.write(CSR_MSTATUS, new_ms, Privilege::M);
                new_priv = Some(if spp == 1 { Privilege::S } else { Privilege::U });
            }
        }

        Inst::Csrrw { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Write, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrs { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Set, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrc { rd, rs1, csr: addr } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, reg!(rs1), CsrOp::Clear, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrwi {
            rd,
            uimm,
            csr: addr,
        } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Write, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrsi {
            rd,
            uimm,
            csr: addr,
        } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Set, priv_level);
                set!(rd, v);
            }
        }
        Inst::Csrrci {
            rd,
            uimm,
            csr: addr,
        } => {
            if !csr_access_ok(addr as usize, priv_level) {
                trap = Some(TrapCause::IllegalInstruction(addr as u32));
            } else {
                let v = csr.read_write(addr as usize, uimm as u64, CsrOp::Clear, priv_level);
                set!(rd, v);
            }
        }

        Inst::Illegal(raw) => trap = Some(TrapCause::IllegalInstruction(raw)),
    }

    ExecResult {
        next_pc,
        trap,
        halt,
        new_priv,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::csr::Privilege;
    use crate::cpu::csr::CSR_MSTATUS;
    use crate::cpu::decoder::Inst;
    use crate::mmu::Mmu;

    fn run(inst: Inst, regs: &mut [u64; 32], csr: &mut CsrFile) -> ExecResult {
        let mut bus = Mmu::new(&[]);
        execute(inst, 0x1000, regs, csr, &mut bus, Privilege::M)
    }
    fn r(val: [u64; 32]) -> [u64; 32] {
        val
    }

    // ---- ALU immediate ----
    #[test]
    fn test_addi_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Addi {
                rd: 2,
                rs1: 1,
                imm: 3,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 8);
        assert_eq!(res.next_pc, 0x1004);
    }
    #[test]
    fn test_addi_neg() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        let mut csr = CsrFile::new();
        run(
            Inst::Addi {
                rd: 2,
                rs1: 1,
                imm: -3,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 2);
    }
    #[test]
    fn test_addi_zero_rd() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        let mut csr = CsrFile::new();
        run(
            Inst::Addi {
                rd: 0,
                rs1: 1,
                imm: 42,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[0], 0); // x0 stays zero
    }
    #[test]
    fn test_slti_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        let mut csr = CsrFile::new();
        run(
            Inst::Slti {
                rd: 2,
                rs1: 1,
                imm: 10,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 1);
        run(
            Inst::Slti {
                rd: 2,
                rs1: 1,
                imm: 3,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 0);
    }

    // ---- ALU register ----
    #[test]
    fn test_add_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 10;
        regs[2] = 20;
        let mut csr = CsrFile::new();
        run(
            Inst::Add {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 30);
    }
    #[test]
    fn test_sub_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 20;
        regs[2] = 10;
        let mut csr = CsrFile::new();
        run(
            Inst::Sub {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 10);
    }
    #[test]
    fn test_xor_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xFF00;
        regs[2] = 0x0FF0;
        let mut csr = CsrFile::new();
        run(
            Inst::Xor {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 0xF0F0);
    }
    #[test]
    fn test_or_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xFF00;
        regs[2] = 0x0FF0;
        let mut csr = CsrFile::new();
        run(
            Inst::Or {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 0xFFF0);
    }
    #[test]
    fn test_and_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xFF00;
        regs[2] = 0x0FF0;
        let mut csr = CsrFile::new();
        run(
            Inst::And {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 0x0F00);
    }

    // ---- Shifts ----
    #[test]
    fn test_slli_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        let mut csr = CsrFile::new();
        run(
            Inst::Slli {
                rd: 2,
                rs1: 1,
                shamt: 3,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 40);
    }
    #[test]
    fn test_srli_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0x100;
        let mut csr = CsrFile::new();
        run(
            Inst::Srli {
                rd: 2,
                rs1: 1,
                shamt: 4,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 0x10);
    }
    #[test]
    fn test_srai_exec() {
        let mut regs = r([0; 32]);
        regs[1] = -256i64 as u64;
        let mut csr = CsrFile::new();
        run(
            Inst::Srai {
                rd: 2,
                rs1: 1,
                shamt: 4,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2] as i64, -16);
    }

    // ---- M-extension ----
    #[test]
    fn test_mul_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 1000;
        regs[2] = 2000;
        let mut csr = CsrFile::new();
        run(
            Inst::Mul {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 2_000_000);
    }
    #[test]
    fn test_mulh_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xABCD_EF01_2345_6789u64 as i64 as u64;
        regs[2] = 0x1234_5678_9ABC_DEF0u64 as i64 as u64;
        let mut csr = CsrFile::new();
        run(
            Inst::Mulh {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        let a = regs[1] as i64 as i128;
        let b = regs[2] as i64 as i128;
        assert_eq!(regs[3], ((a * b) >> 64) as u64);
    }
    #[test]
    fn test_divu_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 100;
        regs[2] = 7;
        let mut csr = CsrFile::new();
        run(
            Inst::Divu {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 14);
    }
    #[test]
    fn test_div_by_zero() {
        let mut regs = r([0; 32]);
        regs[1] = 100;
        regs[2] = 0;
        let mut csr = CsrFile::new();
        run(
            Inst::Div {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], u64::MAX);
    }
    #[test]
    fn test_rem_by_zero() {
        let mut regs = r([0; 32]);
        regs[1] = 100;
        regs[2] = 0;
        let mut csr = CsrFile::new();
        run(
            Inst::Rem {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 100);
    }
    #[test]
    fn test_div_overflow() {
        let mut regs = r([0; 32]);
        regs[1] = i64::MIN as u64;
        regs[2] = -1i64 as u64;
        let mut csr = CsrFile::new();
        run(
            Inst::Div {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], i64::MIN as u64);
    }

    // ---- JAL and branches ----
    #[test]
    fn test_jal_exec() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let res = run(Inst::Jal { rd: 1, imm: 0x100 }, &mut regs, &mut csr);
        assert_eq!(regs[1], 0x1004); // return address = pc+4
        assert_eq!(res.next_pc, 0x1100);
    }
    #[test]
    fn test_jalr_exec() {
        let mut regs = r([0; 32]);
        regs[2] = 0x2000;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Jalr {
                rd: 1,
                rs1: 2,
                imm: 0x100,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[1], 0x1004);
        assert_eq!(res.next_pc, 0x2100);
    }
    #[test]
    fn test_beq_taken() {
        let mut regs = r([0; 32]);
        regs[1] = 42;
        regs[2] = 42;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Beq {
                rs1: 1,
                rs2: 2,
                imm: 0x20,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(res.next_pc, 0x1020);
    }
    #[test]
    fn test_beq_not_taken() {
        let mut regs = r([0; 32]);
        regs[1] = 42;
        regs[2] = 43;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Beq {
                rs1: 1,
                rs2: 2,
                imm: 0x20,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(res.next_pc, 0x1004);
    }
    #[test]
    fn test_blt_taken() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        regs[2] = 10;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Blt {
                rs1: 1,
                rs2: 2,
                imm: 0x30,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(res.next_pc, 0x1030);
    }
    #[test]
    fn test_bge_taken() {
        let mut regs = r([0; 32]);
        regs[1] = 10;
        regs[2] = 5;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Bge {
                rs1: 1,
                rs2: 2,
                imm: 0x30,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(res.next_pc, 0x1030);
    }
    #[test]
    fn test_bltu() {
        let mut regs = r([0; 32]);
        regs[1] = 5;
        regs[2] = 10;
        let mut csr = CsrFile::new();
        let res = run(
            Inst::Bltu {
                rs1: 1,
                rs2: 2,
                imm: 0x40,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(res.next_pc, 0x1040);
    }

    // ---- LUI and AUIPC ----
    #[test]
    fn test_lui_exec() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        run(
            Inst::Lui {
                rd: 1,
                imm: 0x12345000,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[1], 0x12345000);
    }
    #[test]
    fn test_auipc_exec() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        run(Inst::Auipc { rd: 1, imm: 0x1000 }, &mut regs, &mut csr);
        assert_eq!(regs[1], 0x2000); // pc=0x1000 + imm=0x1000
    }

    // ---- 32-bit ops ----
    #[test]
    fn test_addiw_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xFFFF_FFFFu64;
        let mut csr = CsrFile::new();
        run(
            Inst::Addiw {
                rd: 2,
                rs1: 1,
                imm: 1,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[2], 0); // 0xFFFFFFFF + 1 = 0, sign-extended from 32-bit
    }
    #[test]
    fn test_addw_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 0xFFFF_FFFFu64;
        regs[2] = 2;
        let mut csr = CsrFile::new();
        run(
            Inst::Addw {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[3], 1); // 0xFFFFFFFF + 2 = 1 (32-bit) then sign-extended
    }
    #[test]
    fn test_mulw_exec() {
        let mut regs = r([0; 32]);
        regs[1] = 100_000;
        regs[2] = 200_000;
        let mut csr = CsrFile::new();
        run(
            Inst::Mulw {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(
            regs[3],
            ((100_000u64 * 200_000) as u32) as i32 as i64 as u64
        );
    }

    // ---- CSR ----
    #[test]
    fn test_csrrw_exec() {
        let mut regs = r([0; 32]);
        regs[10] = 0xABCD;
        let mut csr = CsrFile::new();
        run(
            Inst::Csrrw {
                rd: 1,
                rs1: 10,
                csr: 0x300,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[1], 0); // old mstatus was 0
        assert_eq!(csr.read(CSR_MSTATUS, Privilege::M), 0xABCD);
    }
    #[test]
    fn test_csrrs_exec() {
        let mut regs = r([0; 32]);
        regs[10] = 0x80; // MSTATUS.MPIE=1
        let mut csr = CsrFile::new();
        run(
            Inst::Csrrs {
                rd: 1,
                rs1: 10,
                csr: 0x300,
            },
            &mut regs,
            &mut csr,
        );
        assert_eq!(regs[1], 0); // old mstatus had no MIE
        assert!(csr.read(CSR_MSTATUS, Privilege::M) & (1 << 7) != 0);
    }

    // ---- System ----
    #[test]
    fn test_ecall_m() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let res = run(Inst::Ecall, &mut regs, &mut csr);
        assert_eq!(res.trap, Some(TrapCause::EcallFromM));
    }
    #[test]
    fn test_ebreak_halt() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let res = run(Inst::Ebreak, &mut regs, &mut csr);
        assert!(res.halt);
    }
    #[test]
    fn test_mret_exec() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        csr.write(CSR_MEPC, 0x2000, Privilege::M);
        csr.write(CSR_MSTATUS, (3 << 11) | (1 << 7), Privilege::M); // MPP=3 (M), MPIE=1
        let res = run(Inst::Mret, &mut regs, &mut csr);
        assert_eq!(res.next_pc, 0x2000);
    }
    #[test]
    fn test_sret_requires_s_mode() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let mut bus = Mmu::new(&[]);
        let res = execute(
            Inst::Sret,
            0x1000,
            &mut regs,
            &mut csr,
            &mut bus,
            Privilege::U,
        );
        assert_eq!(res.trap, Some(TrapCause::IllegalInstruction(0x10200073)));
    }
    #[test]
    fn test_fence_noop() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let res = run(Inst::Fence, &mut regs, &mut csr);
        assert_eq!(res.next_pc, 0x1004);
        assert!(res.trap.is_none());
    }

    // ---- Illegal instruction ----
    #[test]
    fn test_illegal_trap() {
        let mut regs = r([0; 32]);
        let mut csr = CsrFile::new();
        let res = run(Inst::Illegal(0xFFFFFFFF), &mut regs, &mut csr);
        assert_eq!(res.trap, Some(TrapCause::IllegalInstruction(0xFFFFFFFF)));
    }
}
