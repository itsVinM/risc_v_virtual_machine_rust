use crate::println;

const PLIC_BASE: usize = 0x0C00_0000;

const PLIC_PRIORITY: usize = PLIC_BASE;
const PLIC_PENDING: usize = PLIC_BASE + 0x1000;
const PLIC_ENABLE: usize = PLIC_BASE + 0x2000;
const PLIC_THRESHOLD: usize = PLIC_BASE + 0x200000;
const PLIC_CLAIM: usize = PLIC_BASE + 0x200004;

pub const IRQ_UART: u32 = 10;
pub const IRQ_VIRTIO_BLK: u32 = 2;

pub fn set_priority(irq: u32, priority: u32) {
    let addr = (PLIC_PRIORITY + irq as usize * 4) as *mut u32;
    unsafe { core::ptr::write_volatile(addr, priority); }
}

pub fn enable(irq: u32) {
    let enable_addr = PLIC_ENABLE as *mut u32;
    unsafe {
        let val = core::ptr::read_volatile(enable_addr);
        core::ptr::write_volatile(enable_addr, val | (1 << irq));
    }
}

pub fn disable(irq: u32) {
    let enable_addr = PLIC_ENABLE as *mut u32;
    unsafe {
        let val = core::ptr::read_volatile(enable_addr);
        core::ptr::write_volatile(enable_addr, val & !(1 << irq));
    }
}

pub fn set_threshold(threshold: u32) {
    let addr = PLIC_THRESHOLD as *mut u32;
    unsafe { core::ptr::write_volatile(addr, threshold); }
}

pub fn claim() -> u32 {
    let addr = PLIC_CLAIM as *mut u32;
    unsafe { core::ptr::read_volatile(addr) }
}

pub fn complete(irq: u32) {
    let addr = PLIC_CLAIM as *mut u32;
    unsafe { core::ptr::write_volatile(addr, irq); }
}

pub fn init() {
    set_threshold(0);
}

pub fn handle_irq() {
    let irq = claim();
    match irq {
        IRQ_UART => {
            // UART interrupt - handled via getchar polling
            complete(irq);
        }
        IRQ_VIRTIO_BLK => {
            crate::virtio_blk::handle_irq();
            complete(irq);
        }
        _ => {
            println!("Unhandled IRQ {}", irq);
            complete(irq);
        }
    }
}
