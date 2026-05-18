pub mod interrupts;

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
    // Interrupts
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
