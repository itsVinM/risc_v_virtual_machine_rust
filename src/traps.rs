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

pub fn pending_interrupt(mstatus: u64, mie: u64, mip: u64, priv_level: Privilege, mideleg: u64) -> Option<(TrapCause, bool)> {
    let m_irqs: [(u64, TrapCause, bool); 3] = [
        (MIP_MEIP, TrapCause::ExternalInterrupt, false),
        (MIP_MTIP, TrapCause::TimerInterrupt,    false),
        (MIP_MSIP, TrapCause::SoftwareInterrupt, false),
    ];
    let s_irqs: [(u64, TrapCause, bool); 3] = [
        (MIP_SEIP, TrapCause::ExternalInterrupt, true),
        (MIP_STIP, TrapCause::TimerInterrupt,    true),
        (MIP_SSIP, TrapCause::SoftwareInterrupt, true),
    ];
    let pending = mie & mip;

    match priv_level {
        Privilege::M => {
            if (mstatus & MSTATUS_MIE) != 0 {
                for &(bit, ref cause, del) in &m_irqs {
                    if pending & bit != 0 { return Some((*cause, del)); }
                }
            }
        }
        Privilege::S => {
            // M-mode interrupts always enabled when in lower privilege
            for &(bit, ref cause, _) in &m_irqs {
                if pending & bit != 0 { return Some((*cause, false)); }
            }
            if (mstatus & MSTATUS_SIE) != 0 {
                for &(bit, ref cause, _) in &s_irqs {
                    if pending & bit != 0 {
                        let delegated = (mideleg & bit) != 0;
                        return Some((*cause, delegated));
                    }
                }
            }
        }
        Privilege::U => {
            // M-mode interrupts always enabled
            for &(bit, ref cause, _) in &m_irqs {
                if pending & bit != 0 { return Some((*cause, false)); }
            }
            // S-mode interrupts always enabled from U-mode
            for &(bit, ref cause, _) in &s_irqs {
                if pending & bit != 0 {
                    let delegated = (mideleg & bit) != 0;
                    return Some((*cause, delegated));
                }
            }
        }
    }

    None
}
