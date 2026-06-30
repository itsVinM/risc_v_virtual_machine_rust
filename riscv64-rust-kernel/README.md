# riscv64-rust-kernel

A bare-metal RISC‑V 64‑bit kernel written in Rust — zero external dependencies.

Boots on the [risc_v_virtual_machine_rust](https://github.com/anomalyco/risc_v_virtual_machine_rust) VM
or any QEMU‑compatible RISC‑V virt platform (OpenSBI S‑mode).

## Build

```sh
cargo build --release
```

Produces `target/riscv64gc-unknown-none-elf/release/riscv64-rust-kernel` (~240 KB with LTO).

## Run

```sh
# With the RISC-V VM
riscv-vm --sbi target/release/riscv64-rust-kernel

# With QEMU
qemu-system-riscv64 -M virt -m 128M \
  -bios default \
  -kernel target/riscv64gc-unknown-none-elf/release/riscv64-rust-kernel \
  -nographic
```

## Architecture

| Module | File | Role |
|--------|------|------|
| Boot | `entry.rs` | `global_asm!` entry at `0x80200000`, clears BSS, sets stack, calls `kernel_main` |
| SBI | `sbi.rs` | Ecall wrappers — Base, Timer, IPI, DBCN, SRST, HSM extensions |
| Console | `console.rs` | `putchar`/`getchar` via SBI; `print!`/`println!` macros |
| Trap | `trap.rs` | `stvec` handler — timer, external IRQ, syscall, page fault dispatch |
| Page allocator | `allocator.rs` | Bitmap-based, O(1) via `trailing_zeros()`, heap at end of DRAM |
| Paging | `paging.rs` | Sv39 page table — map/unmap/translate, `sfence.vma` |
| PLIC | `plic.rs` | Platform-Level Interrupt Controller — priority, enable, claim/complete |
| Timer | `timer.rs` | CLINT `mtime` via SBI, 10 ms periodic ticks |
| Virtio block | `virtio_blk.rs` | Virtio-MMIO v1, single virtqueue, read/write sectors |
| Filesystem | `fs.rs` | Sector bitmap allocator, inode table, read/write/ls |
| Syscall | `syscall.rs` | `SYS_read`/`SYS_write`/`SYS_exit` |
| Scheduler | `scheduler.rs` | Stub (single-threaded, ready for round-robin) |
| Process | `process.rs` | Stub (PID allocation for future processes) |
| Spinlock | `spinlock.rs` | `AtomicBool`-based spinlock with guard |

## Memory map

| Region | Address | Size |
|--------|---------|------|
| DRAM | `0x80000000` | 128 MB |
| Kernel | `0x80200000` | ~64 KB (text + data + BSS) |
| Heap | `_kernel_end` – `_heap_end` | remainder minus 16 KB |
| Stack | `_stack_bottom` – `_stack_top` | 8 KB at top of DRAM |
| UART | `0x10000000` | 256 B |
| PLIC | `0x0C000000` | 64 MB |
| CLINT | `0x02000000` | 1 MB |
| Virtio | `0x10001000` | 4 KB |

## Interactive shell

After boot, the kernel drops into a shell:

```
> help
Commands: help, info, uptime, ls, alloc, reboot, panic
> uptime
Uptime: 340 ms
> alloc
Pages: 28630 total, 28626 free, 4 used (16 KB)
> ls
  / (0b)
  hello.txt (29b)
> reboot
System reset...
```

## Boot protocol

Follows OpenSBI S‑mode convention:

- Load address: `0x80200000`
- `a0` = hart ID
- `a1` = DTB address
- SBI available for console, timer, and shutdown

## Dependencies

**Zero.** All functionality is hand-rolled — no `alloc`, no `cc`, no proc macros, no HAL crates.
The only external artifacts are `rust-std` for `riscv64gc-unknown-none-elf` (compiler builtins)
and `rust-lld` for linking.
