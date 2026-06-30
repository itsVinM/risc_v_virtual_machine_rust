#![no_std]
#![no_main]

mod entry;
mod sbi;
mod console;
mod trap;
mod spinlock;
mod timer;
mod allocator;
mod paging;
mod plic;
mod process;
mod scheduler;
mod syscall;
mod virtio_blk;
mod fs;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    console::puts("PANIC: ");
    if let Some(msg) = info.message().as_str() {
        console::puts(msg);
    }
    console::puts("\n");
    loop { sbi::legacy_shutdown(); }
}

extern "C" {
    static _kernel_end: u8;
    static _heap_start: u8;
    static _heap_end: u8;
}

#[no_mangle]
pub extern "C" fn kernel_main(hartid: u64, dtb: u64) -> ! {
    console::puts("\n");
    console::puts("========================================\n");
    console::puts("     riscv64-rust-kernel v0.1.0\n");
    console::puts("========================================\n");
    console::puts("\n");

    println!("Boot hart: {}", hartid);
    println!("DTB: 0x{:x}", dtb);

    let kernel_end = unsafe { &_kernel_end as *const u8 as usize };
    let heap_start = unsafe { &_heap_start as *const u8 as usize };
    let heap_end = unsafe { &_heap_end as *const u8 as usize };

    println!("Kernel end: 0x{:x}", kernel_end);
    println!("Heap: 0x{:x} - 0x{:x} ({} KB)", heap_start, heap_end, (heap_end - heap_start) / 1024);

    println!("Allocator...");
    allocator::init();
    println!("  ready");

    println!("Page table...");
    let mut kernel_pt = paging::PageTable::new().expect("no page table");
    for vaddr in (0x80200000..kernel_end).step_by(paging::PAGE_SIZE) {
        kernel_pt.map(vaddr, vaddr, paging::READ_WRITE_EXECUTE);
    }
    kernel_pt.map(0x10000000, 0x10000000, paging::READ_WRITE);
    kernel_pt.map(0x0C000000, 0x0C000000, paging::READ_WRITE);
    kernel_pt.map(0x02000000, 0x02000000, paging::READ_WRITE);
    kernel_pt.map(0x10001000, 0x10001000, paging::READ_WRITE);
    paging::enable(kernel_pt.root_paddr());
    println!("  enabled");

    println!("Traps...");
    trap::init();
    println!("  ready");

    println!("PLIC...");
    plic::init();
    plic::set_priority(plic::IRQ_VIRTIO_BLK, 1);
    plic::enable(plic::IRQ_VIRTIO_BLK);
    println!("  ready");

    println!("Timer...");
    timer::init();
    println!("  armed");

    println!("Virtio block...");
    virtio_blk::init();

    println!("Filesystem...");
    fs::init();

    println!("\nTesting block read...");
    let mut sector = [0u8; 512];
    if virtio_blk::read_sector(0, &mut sector) {
        let non_zero = sector.iter().filter(|&&b| b != 0).count();
        println!("  sector 0: {} non-zero bytes", non_zero);
    } else {
        println!("  read failed");
    }

    println!("\nTesting filesystem...");
    let data = b"Hello from riscv64-rust-kernel!\n";
    if fs::write_file("hello.txt", data) {
        println!("  wrote hello.txt");
    }
    fs::ls();

    let mut buf = [0u8; 64];
    if let Some(n) = fs::read_file("hello.txt", &mut buf) {
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("?");
        println!("  read: {}", s);
    }

    println!("\nKernel ready. Type a command.\n");

    let mut line = [0u8; 64];
    let mut pos;

    loop {
        console::putchar(b'>');
        console::putchar(b' ');
        pos = 0;
        loop {
            match console::getchar() {
                Some(b'\r') | Some(b'\n') => {
                    console::putchar(b'\n');
                    if pos > 0 {
                        let cmd = core::str::from_utf8(&line[..pos]).unwrap_or("");
                        match cmd.trim() {
                            "help" => {
                                println!("Commands: help, info, uptime, ls, alloc, reboot, panic, read");
                            }
                            "info" => {
                                println!("Kernel at 0x80200000");
                                println!("Heap {} KB", (heap_end - heap_start) / 1024);
                            }
                            "uptime" => {
                                println!("Uptime: {} ms", timer::uptime_ms());
                            }
                            "ls" => fs::ls(),
                            "alloc" => {
                                let a_info = allocator::info();
                                println!("Pages: {} total, {} free, {} used ({} KB)",
                                    a_info.0, a_info.1, a_info.0 - a_info.1, (a_info.0 - a_info.1) * 4);
                            }
                            "reboot" => {
                                println!("Reboot.");
                                sbi::system_reset();
                            }
                            "panic" => panic!("manual panic"),
                            _ => {
                                println!("Unknown: '{}'", cmd);
                            }
                        }
                    }
                    break;
                }
                Some(c) => {
                    console::putchar(c);
                    if pos < line.len() {
                        line[pos] = c;
                        pos += 1;
                    }
                }
                None => {}
            }
        }
    }
}
