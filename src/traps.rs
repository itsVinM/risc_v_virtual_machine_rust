// ── Trap causes ───────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapCause {
    // Exceptions
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction(u32),
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    EcallFromU,
    EcallFromS,
    EcallFromM,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    // Interrupts (bit 63 set in mcause)
    SoftwareInterrupt,
    TimerInterrupt,
    ExternalInterrupt,
}

impl TrapCause {
    pub fn code(self) -> u64 {
        match self {
            Self::InstructionAddressMisaligned => 0,
            Self::InstructionAccessFault       => 1,
            Self::IllegalInstruction(_)        => 2,
            Self::Breakpoint                   => 3,
            Self::LoadAddressMisaligned        => 4,
            Self::LoadAccessFault              => 5,
            Self::StoreAddressMisaligned       => 6,
            Self::StoreAccessFault             => 7,
            Self::EcallFromU                   => 8,
            Self::EcallFromS                   => 9,
            Self::EcallFromM                   => 11,
            Self::InstructionPageFault         => 12,
            Self::LoadPageFault                => 13,
            Self::StorePageFault               => 15,
            Self::SoftwareInterrupt            => (1 << 63) | 3,
            Self::TimerInterrupt               => (1 << 63) | 7,
            Self::ExternalInterrupt            => (1 << 63) | 11,
        }
    }

    pub fn is_interrupt(self) -> bool {
        matches!(self, Self::SoftwareInterrupt | Self::TimerInterrupt | Self::ExternalInterrupt)
    }
}

// ── mstatus / mie / mip bit masks ─────────────────────────────────────────────
pub const MSTATUS_MIE:  u64 = 1 << 3;   // global interrupt enable
pub const MSTATUS_MPIE: u64 = 1 << 7;   // previous interrupt enable (saved on trap)
pub const MSTATUS_MPP:  u64 = 0b11 << 11; // previous privilege mode

pub const MIE_MSIE: u64 = 1 << 3;  // machine software interrupt enable
pub const MIE_MTIE: u64 = 1 << 7;  // machine timer interrupt enable
pub const MIE_MEIE: u64 = 1 << 11; // machine external interrupt enable

// ── Interrupt check ───────────────────────────────────────────────────────────
// Called every step: returns the highest-priority pending+enabled interrupt,
// or None if interrupts are globally disabled or nothing is pending.
pub fn pending_interrupt(mstatus: u64, mie: u64, mip: u64) -> Option<TrapCause> {
    if mstatus & MSTATUS_MIE == 0 { return None; }
    let pending = mie & mip;
    if pending & MIE_MEIE != 0 { return Some(TrapCause::ExternalInterrupt); }
    if pending & MIE_MTIE != 0 { return Some(TrapCause::TimerInterrupt); }
    if pending & MIE_MSIE != 0 { return Some(TrapCause::SoftwareInterrupt); }
    None
}
