# risc-v virtual machine

A minimal **RV64IM** RISC-V virtual machine in pure Rust (zero dependencies).  
Boots supervisor-mode kernels (xv6, Linux) via SBI or bare-metal M-mode binaries.

## Features

- **ISA**: RV64I base + M (multiply/divide) extension
- **Privilege levels**: M-mode (bare-metal) and S-mode (supervisor with SBI)
- **CSRs**: Full M-mode and S-mode CSR set; MIDELEG/MEDELEG for interrupt delegation
- **SBI**: Legacy (v0.1) and v0.2 extensions: console putchar, timer, IPI, RFENCE, HSM (hart stop)
- **CLINT**: Memory-mapped `mtime`/`mtimecmp` with timer interrupts; MSIP for software interrupts
- **SSTC**: Supervisor-mode timer compare (`stimecmp`) extension
- **PLIC**: Platform-Level Interrupt Controller with S-mode context, supports external interrupt delegation
- **UART 16550**: Memory-mapped at `0x1000_0000`, polled output
- **Virtio block**: MMIO transport v1.0, single virtqueue, 64 MB zeroed disk image
- **DTB**: Built-in Flattened Device Tree generator for S-mode kernel boot (clock, cpus, CLINT, PLIC, UART)
- **ELF loader**: Loads ELF64 binaries at program-header-specified addresses
- **PMP**: Physical Memory Protection — supervisor access to all physical memory
- **Interactive debugger**: Terminal-based CLI with stepping, breakpoints, run-to-breakpoint, register/disassembly view
- **No standard library dependency** in core logic: `#![no_std]` with `alloc`

## Build & run

```sh
cargo build --release

# Built-in M-mode demo: fib(10) = 55
./target/release/riscv-vm

# Raw M-mode binary loaded at 0x80000000
./target/release/riscv-vm <binary.bin>

# S-mode kernel (elf or raw) with DTB + SBI
./target/release/riscv-vm --sbi <kernel>       # headless
./target/release/riscv-vm --sbi --debug <kernel>  # with debugger

# xv6 (ELF, boots in M-mode from entry point)
./target/release/riscv-vm xv6-kernel
```

## Usage

| Flag | Description |
|------|-------------|
| `--sbi` / `-s` | Boot kernel in S-mode via SBI; generates DTB at `0x8600_0000` |
| `--debug` / `-d` | Interactive CLI debugger (ANSI terminal) |

Without `--sbi`, ELF binaries boot in M-mode at their entry address; raw binaries load at `0x80000000`.

## Memory map

| Region | Base | Size | Description |
|--------|------|------|-------------|
| CLINT  | `0x02000000` | 1 MB | Timer (`mtime`/`mtimecmp`) and software interrupts |
| PLIC   | `0x0C000000` | 64 MB | External interrupt controller |
| UART   | `0x10000000` | 256 B | 16550-compatible serial port |
| VIRTIO | `0x10001000` | 4 KB | Virtio MMIO block device |
| DRAM   | `0x80000000` | 128 MB | Main memory (kernel, DTB `0x86000000`) |

### CLINT register layout (relative to `0x02000000`)

| Offset | Register | Access |
|--------|----------|--------|
| `0x0000` | MSIP (hart 0) | R/W — bit 0 maps to `mip.SSIP` |
| `0x4000` | MTIMECMP (hart 0) | R/W — 64-bit |
| `0xBFF8` | MTIME | R/W — 64-bit, increments every tick |

### Virtio register layout (relative to `0x10001000`)

| Offset | Register | Description |
|--------|----------|-------------|
| `0x000` | MagicValue | `0x74726976` |
| `0x004` | Version | `2` (MMIO v1) |
| `0x008` | DeviceID | `2` (block device) |
| `0x010` | DeviceFeatures | `VIRTIO_F_VERSION_1` |
| `0x050` | QueueNotify | Write to kick queue |
| `0x060` | InterruptStatus | Read; bit 0 = used buffer |
| `0x064` | InterruptACK | Write to acknowledge interrupt |
| `0x070` | Status | Device status register |
| `0x080`–`0x0A4` | Queue addresses | Desc, Avail, Used ring physical addresses |

## Debugger

| Command | Action |
|---------|--------|
| `s` / Enter | Step one instruction |
| `r <n>` | Run `n` instructions |
| `c` | Continue until breakpoint or halt |
| `b [\<hex\>]` | Toggle breakpoint at address (default: PC) |
| `q` | Quit |

`►` marks the current PC; `●` marks a breakpoint. Non-zero registers highlighted in green.

## SBI support

| Extension | Functions |
|-----------|-----------|
| Legacy v0.1 | `SBI_SET_TIMER`, `SBI_CONSOLE_PUTCHAR`, `SBI_SHUTDOWN` |
| Base (0x10) | `GET_SPEC_VERSION`, `GET_IMP_ID`, `PROBE_EXT`, `GET_MVENDORID`/`MARCHID`/`MIMPID` |
| Timer (0x54494D45) | `TIME_SET_TIMER` — sets `mtimecmp` |
| IPI (0x735049) | Stub (success) |
| RFENCE (0x52464E43) | Stub (success) |
| HSM (0x48534D) | `HART_STOP` — halts execution |

## Boot conventions

- **M-mode raw binaries**: loaded at `0x80000000`, start with `mstatus.MPP=S`, `mepc=entry`, `a0=hartid`, `a1=DTB_ADDR` (when `--sbi`).
- **ELF binaries** (without `--sbi`): boot in M-mode at ELF entry point. xv6 uses this to transition M→S via `mret`.
- **DTB** placed at `0x86000000` for S-mode kaboots.

## Test

```sh
cargo test
```

## Docker (headless)

```sh
docker build -t riscv-vm .
docker run --rm -v $(pwd):/work riscv-vm /work/program.bin
```
