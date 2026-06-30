use crate::console;
use crate::println;

const SYS_WRITE: u64 = 64;
const SYS_READ: u64 = 63;
const SYS_EXIT: u64 = 93;

const STDOUT: u64 = 1;
const STDERR: u64 = 2;

pub fn handle(num: u64, args: [u64; 6]) -> u64 {
    match num {
        SYS_WRITE => {
            let fd = args[0];
            let buf = args[1] as *const u8;
            let len = args[2] as usize;
            if fd == STDOUT || fd == STDERR {
                for i in 0..len {
                    let c = unsafe { core::ptr::read_volatile(buf.add(i)) };
                    console::putchar(c);
                }
                len as u64
            } else {
                -1i64 as u64
            }
        }
        SYS_READ => {
            let buf = args[1] as *mut u8;
            let mut count = 0u64;
            loop {
                if let Some(c) = console::getchar() {
                    unsafe { core::ptr::write_volatile(buf, c); }
                    count += 1;
                    if c == b'\n' || count >= args[2] {
                        return count;
                    }
                }
            }
        }
        SYS_EXIT => {
            println!("Process exited with code {}", args[0]);
            0
        }
        _ => {
            println!("Unknown syscall {}", num);
            -1i64 as u64
        }
    }
}
