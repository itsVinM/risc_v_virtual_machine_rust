# risc-v virtual machine

Minimal **RV64IMA** RISC-V VM in pure Rust. Boots S-mode kernels (xv6, Linux) via SBI or M-mode binaries.

```
cargo run                                     # fib(10) demo
cargo run -- program.bin                      # M-mode binary
cargo run -- --sbi kernel                     # S-mode kernel
cargo run -- --sbi --debug kernel             # with debugger
```

| Flag | Description |
|------|-------------|
| `--sbi` / `-s` | Boot S-mode with DTB + SBI |
| `--debug` / `-d` | Interactive debugger |

## Debugger

Each step shows one line: `pc  raw_instruction_hex`

| Command | Description |
|---------|-------------|
| `s` | Step one instruction |
| `r [n]` | Run n steps (default 100) |
| `c` | Continue until breakpoint/halt |
| `b [addr]` | Toggle breakpoint (default PC) |
| `reg [name] [val]` | Read/write register |
| `mem addr [n]` | Dump n×32-bit words |
| `mem8/16/32/64 addr` | Read 1/2/4/8 bytes |
| `csr addr` | Read a CSR |
| `reset` | Reset CPU |
| `h` | Help |

## Memory map

| Region | Base | Size |
|--------|------|------|
| CLINT | `0x02000000` | 1 MB |
| PLIC | `0x0C000000` | 64 MB |
| UART | `0x10000000` | 256 B |
| VIRTIO | `0x10001000` | 4 KB |
| DRAM | `0x80000000` | 128 MB |

## SBI support

Legacy v0.1 (putchar, getchar, shutdown), Base, Timer, IPI, RFENCE, HSM, SRST, DBCN.
