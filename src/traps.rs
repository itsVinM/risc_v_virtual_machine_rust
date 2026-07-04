use crate::cpu::csr::{
    MIP_MEIP, MIP_MSIP, MIP_MTIP,
    MIP_SEIP, MIP_SSIP, MIP_STIP,
    MSTATUS_MIE, MSTATUS_SIE,
    Privilege,
};



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TrapCause {
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

    pub fn exception_code(self) -> u64 {
        self.code() & !(1 << 63)
    }
}

pub fn pending_interrupt(
    mstatus: u64,
    mie: u64,
    mip: u64,
    priv_level: Privilege,
    mideleg: u64,
) -> Option<(TrapCause, bool)> {
    let pending = mie & mip;
    let m_irqs = [
        (MIP_MEIP, TrapCause::ExternalInterrupt, true),
        (MIP_MTIP, TrapCause::TimerInterrupt, true),
        (MIP_MSIP, TrapCause::SoftwareInterrupt, true),
    ];
    let s_irqs = [
        (MIP_SEIP, TrapCause::ExternalInterrupt, false),
        (MIP_STIP, TrapCause::TimerInterrupt, false),
        (MIP_SSIP, TrapCause::SoftwareInterrupt, false),
    ];

    // Check M-mode interrupts
    let m_enabled = match priv_level {
        Privilege::M => (mstatus & MSTATUS_MIE) != 0,
        Privilege::S | Privilege::U => true, // M-mode interrupts always enabled for S/U
    };
    if m_enabled {
        if let Some(&(_, cause, delegated)) = m_irqs
            .iter()
            .find(|&&(mask, _, _)| (pending & mask) != 0)
        {
            return Some((cause, delegated));
        }
    }

    // For M-mode, no further checks
    if priv_level == Privilege::M {return None;}

    // Check S-mode interrupts
    let s_enabled = match priv_level {
        Privilege::S => (mstatus & MSTATUS_SIE) != 0,
        Privilege::U => true, // S-mode interrupts always enabled for U
        Privilege::M => unreachable!(),
    };
    if s_enabled {
        if let Some(&(_bit, cause, _)) = s_irqs
            .iter()
            .find(|&&(bit, _, _) | pending & bit != 0 && (mideleg & bit) != 0) {
                return Some((cause, true));
            }
    }

    None
}