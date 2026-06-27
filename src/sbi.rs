use crate::mmu::Mmu;
use crate::uart;

// SBI extension IDs (legacy)
pub const SBI_SET_TIMER: u64 = 0;
pub const SBI_CONSOLE_PUTCHAR: u64 = 1;
pub const SBI_CONSOLE_GETCHAR: u64 = 2;
pub const SBI_CLEAR_IPI: u64 = 3;
pub const SBI_SEND_IPI: u64 = 4;
pub const SBI_REMOTE_FENCE_I: u64 = 5;
pub const SBI_REMOTE_SFENCE_VMA: u64 = 6;
pub const SBI_REMOTE_SFENCE_VMA_ASID: u64 = 7;
pub const SBI_SHUTDOWN: u64 = 8;

// SBI v0.2+ extension IDs
pub const SBI_EXT_BASE: u64 = 0x10;
pub const SBI_EXT_TIME: u64 = 0x54494D45;
pub const SBI_EXT_IPI: u64 = 0x735049;
pub const SBI_EXT_RFENCE: u64 = 0x52464E43;
pub const SBI_EXT_HSM: u64 = 0x48534D;

// SBI v0.2 function IDs
pub const SBI_BASE_GET_SPEC_VERSION: u64 = 0;
pub const SBI_BASE_GET_IMP_ID: u64 = 1;
pub const SBI_BASE_GET_IMP_VERSION: u64 = 2;
pub const SBI_BASE_PROBE_EXT: u64 = 3;
pub const SBI_BASE_GET_MVENDORID: u64 = 4;
pub const SBI_BASE_GET_MARCHID: u64 = 5;
pub const SBI_BASE_GET_MIMPID: u64 = 6;
pub const SBI_TIME_SET_TIMER: u64 = 0;
pub const SBI_HSM_HART_STOP: u64 = 1;

// SBI return values
pub const SBI_SUCCESS: u64 = 0;
pub const SBI_ERR_NOT_SUPPORTED: u64 = -2i64 as u64;

pub fn handle_sbi(a7: u64, a0: u64, _a1: u64, _a2: u64, a6: u64, bus: &mut Mmu) -> (u64, u64) {
    match a7 {
        // Legacy extensions
        SBI_SET_TIMER => {
            // stimecmp is at CLINT + 0x4000. Linux uses SBI to set timer.
            bus.clint.mtimecmp = a0;
            (0, 0)
        }
        SBI_CONSOLE_PUTCHAR => {
            let _ = bus.write8(uart::UART_BASE, a0 as u8);
            (0, 0)
        }
        SBI_CONSOLE_GETCHAR => {
            (-1i64 as u64, 0)
        }
        SBI_CLEAR_IPI | SBI_SEND_IPI | SBI_REMOTE_FENCE_I
        | SBI_REMOTE_SFENCE_VMA | SBI_REMOTE_SFENCE_VMA_ASID => (0, 0),
        SBI_SHUTDOWN => {
            // Signal halt by returning a special value
            (0, 1)
        }
        // v0.2 extensions
        SBI_EXT_BASE => handle_sbi_base(a6, a0),
        SBI_EXT_TIME => {
            if a6 == SBI_TIME_SET_TIMER {
                bus.clint.mtimecmp = a0;
                (0, SBI_SUCCESS)
            } else {
                (0, SBI_ERR_NOT_SUPPORTED)
            }
        }
        SBI_EXT_IPI => (0, SBI_SUCCESS),
        SBI_EXT_RFENCE => (0, SBI_SUCCESS),
        SBI_EXT_HSM => {
            if a6 == SBI_HSM_HART_STOP {
                (0, 1) // signal halt
            } else {
                (0, SBI_ERR_NOT_SUPPORTED)
            }
        }
        _ => (0, SBI_ERR_NOT_SUPPORTED),
    }
}

fn handle_sbi_base(func_id: u64, ext: u64) -> (u64, u64) {
    match func_id {
        SBI_BASE_GET_SPEC_VERSION => (0, 2),
        SBI_BASE_GET_IMP_ID => (1, SBI_SUCCESS),
        SBI_BASE_GET_IMP_VERSION => (1, SBI_SUCCESS),
        SBI_BASE_PROBE_EXT => {
            match ext {
                SBI_EXT_BASE | SBI_EXT_TIME | SBI_EXT_IPI | SBI_EXT_RFENCE | SBI_EXT_HSM => (1, SBI_SUCCESS),
                _ => (0, SBI_SUCCESS),
            }
        }
        SBI_BASE_GET_MVENDORID => (0, SBI_SUCCESS),
        SBI_BASE_GET_MARCHID => (0, SBI_SUCCESS),
        SBI_BASE_GET_MIMPID => (0, SBI_SUCCESS),
        _ => (0, SBI_ERR_NOT_SUPPORTED),
    }
}
