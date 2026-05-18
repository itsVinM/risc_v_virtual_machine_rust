use crate::traps::TrapCause;

// mstatus fields
pub const MSTATUS_MIE:  u64 = 1 << 3;
pub const MSTATUS_MPIE: u64 = 1 << 7;
pub const MSTATUS_MPP:  u64 = 0b11 << 11;

// mie / mip bits
pub const MIE_MSIE: u64 = 1 << 3;  // machine software interrupt
pub const MIE_MTIE: u64 = 1 << 7;  // machine timer interrupt
pub const MIE_MEIE: u64 = 1 << 11; // machine external interrupt

pub fn pending_interrupt(mstatus: u64, mie: u64, mip: u64) -> Option<TrapCause> {
    if mstatus & MSTATUS_MIE == 0 {
        return None;
    }
    let pending = mie & mip;
    if pending & MIE_MEIE != 0 { return Some(TrapCause::ExternalInterrupt); }
    if pending & MIE_MTIE != 0 { return Some(TrapCause::TimerInterrupt); }
    if pending & MIE_MSIE != 0 { return Some(TrapCause::SoftwareInterrupt); }
    None
}
