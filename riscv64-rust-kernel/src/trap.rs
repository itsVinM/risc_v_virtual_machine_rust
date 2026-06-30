use crate::timer;
use crate::scheduler;
use crate::syscall;
use crate::println;

#[repr(C)]
pub struct TrapFrame {
    pub ra: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
    pub sepc: u64,
    pub sstatus: u64,
}

core::arch::global_asm!(
    ".align 4",
    ".global trap_vector",
    "trap_vector:",
    "   addi sp, sp, -256",
    "   sd ra, 0*8(sp)",
    "   sd gp, 1*8(sp)",
    "   sd tp, 2*8(sp)",
    "   sd t0, 3*8(sp)",
    "   sd t1, 4*8(sp)",
    "   sd t2, 5*8(sp)",
    "   sd s0, 6*8(sp)",
    "   sd s1, 7*8(sp)",
    "   sd a0, 8*8(sp)",
    "   sd a1, 9*8(sp)",
    "   sd a2, 10*8(sp)",
    "   sd a3, 11*8(sp)",
    "   sd a4, 12*8(sp)",
    "   sd a5, 13*8(sp)",
    "   sd a6, 14*8(sp)",
    "   sd a7, 15*8(sp)",
    "   sd s2, 16*8(sp)",
    "   sd s3, 17*8(sp)",
    "   sd s4, 18*8(sp)",
    "   sd s5, 19*8(sp)",
    "   sd s6, 20*8(sp)",
    "   sd s7, 21*8(sp)",
    "   sd s8, 22*8(sp)",
    "   sd s9, 23*8(sp)",
    "   sd s10, 24*8(sp)",
    "   sd s11, 25*8(sp)",
    "   sd t3, 26*8(sp)",
    "   sd t4, 27*8(sp)",
    "   sd t5, 28*8(sp)",
    "   sd t6, 29*8(sp)",
    "   csrr t0, sepc",
    "   sd t0, 30*8(sp)",
    "   csrr t0, sstatus",
    "   sd t0, 31*8(sp)",
    "   mv a0, sp",
    "   call trap_handler",
    "   ld t0, 30*8(sp)",
    "   csrw sepc, t0",
    "   ld t0, 31*8(sp)",
    "   csrw sstatus, t0",
    "   ld ra, 0*8(sp)",
    "   ld gp, 1*8(sp)",
    "   ld tp, 2*8(sp)",
    "   ld t0, 3*8(sp)",
    "   ld t1, 4*8(sp)",
    "   ld t2, 5*8(sp)",
    "   ld s0, 6*8(sp)",
    "   ld s1, 7*8(sp)",
    "   ld a0, 8*8(sp)",
    "   ld a1, 9*8(sp)",
    "   ld a2, 10*8(sp)",
    "   ld a3, 11*8(sp)",
    "   ld a4, 12*8(sp)",
    "   ld a5, 13*8(sp)",
    "   ld a6, 14*8(sp)",
    "   ld a7, 15*8(sp)",
    "   ld s2, 16*8(sp)",
    "   ld s3, 17*8(sp)",
    "   ld s4, 18*8(sp)",
    "   ld s5, 19*8(sp)",
    "   ld s6, 20*8(sp)",
    "   ld s7, 21*8(sp)",
    "   ld s8, 22*8(sp)",
    "   ld s9, 23*8(sp)",
    "   ld s10, 24*8(sp)",
    "   ld s11, 25*8(sp)",
    "   ld t3, 26*8(sp)",
    "   ld t4, 27*8(sp)",
    "   ld t5, 28*8(sp)",
    "   ld t6, 29*8(sp)",
    "   addi sp, sp, 256",
    "   sret",
);

#[repr(u64)]
enum TrapCause {
    SupervisorSoftwareInterrupt = 1,
    SupervisorTimerInterrupt = 5,
    SupervisorExternalInterrupt = 9,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
    EnvironmentCallFromUMode = 8,
    IllegalInstruction = 2,
    Breakpoint = 3,
}

#[no_mangle]
extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let scause: u64;
    let stval: u64;
    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) scause);
        core::arch::asm!("csrr {}, stval", out(reg) stval);
    }

    match scause {
        1 => {}
        5 => {
            timer::tick();
            scheduler::tick_timer();
            timer::set_next_timer();
        }
        9 => {
            crate::plic::handle_irq();
        }
        8 => {
            let syscall_num = frame.a7;
            let ret = syscall::handle(
                syscall_num,
                [frame.a0, frame.a1, frame.a2, frame.a3, frame.a4, frame.a5],
            );
            frame.a0 = ret;
            frame.sepc = frame.sepc.wrapping_add(4);
        }
        12 => {
            println!("PANIC: Instruction page fault at 0x{:x}, stval=0x{:x}", frame.sepc, stval);
            dump_frame(frame);
            loop {}
        }
        13 => {
            println!("PANIC: Load page fault at 0x{:x}, stval=0x{:x}", frame.sepc, stval);
            dump_frame(frame);
            loop {}
        }
        15 => {
            println!("PANIC: Store page fault at 0x{:x}, stval=0x{:x}", frame.sepc, stval);
            dump_frame(frame);
            loop {}
        }
        2 => {
            println!("PANIC: Illegal instruction at 0x{:x}, stval=0x{:x}", frame.sepc, stval);
            dump_frame(frame);
            loop {}
        }
        3 => {
            println!("Breakpoint at 0x{:x}", frame.sepc);
            frame.sepc = frame.sepc.wrapping_add(2);
        }
        _ => {
            println!("PANIC: Unknown trap cause {} at 0x{:x}, stval=0x{:x}", scause, frame.sepc, stval);
            dump_frame(frame);
            loop {}
        }
    }
}

fn dump_frame(frame: &TrapFrame) {
    println!("  ra=0x{:016x} sp=0x{:016x}", frame.ra, frame as *const _ as u64);
    println!("  a0=0x{:016x} a1=0x{:016x}", frame.a0, frame.a1);
    println!("  a2=0x{:016x} a3=0x{:016x}", frame.a2, frame.a3);
    println!("  a4=0x{:016x} a5=0x{:016x}", frame.a4, frame.a5);
    println!("  a6=0x{:016x} a7=0x{:016x}", frame.a6, frame.a7);
    println!("  sepc=0x{:016x} sstatus=0x{:016x}", frame.sepc, frame.sstatus);
}

pub fn init() {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) trap_vector as *const () as u64);
        core::arch::asm!("csrw sie, {}", in(reg) (1 << 5) | (1 << 9));
    }
}

extern "C" {
    fn trap_vector();
}
