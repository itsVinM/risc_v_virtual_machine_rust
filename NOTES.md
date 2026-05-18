# risc-v vm — deep notes

Personal learning reference. Not tracked by git (run `git check-ignore NOTES.md` — nothing).
Keep adding to this as understanding grows.

---

## 1. what is RISC-V

RISC-V ("risk five") is an open-source instruction set architecture (ISA).
It is deliberately *minimal*: the base spec fits in a short document.
Extensions are opt-in and labelled with letters:

| Letter | Meaning                         |
|--------|---------------------------------|
| I      | base integer (all 32 regs, ALU) |
| M      | multiply & divide               |
| A      | atomic operations               |
| F / D  | single / double float           |
| C      | compressed 16-bit instructions  |
| S      | supervisor mode                 |
| U      | user mode                       |

This VM implements **RV64IM** — 64-bit registers, integer base, multiply.

### Why 64-bit?
The "64" means general-purpose registers hold 64-bit values.
RV64I adds the W-suffix instructions (ADDW, ADDIW, …) that operate on
the low 32 bits and sign-extend the result into 64 bits.
This matters because C's `int` is 32 bits even on 64-bit systems.

### Why no floating point here?
FP adds ~100 instructions and a separate register file (f0–f31).
A baremetal "hello world" only needs integer + UART; FP can be added later.

---

## 2. RISC-V instruction encoding — bit manipulation

Every base instruction is exactly **32 bits** wide.
The opcode occupies bits [6:0], and bits [1:0] are always `11`
(if they aren't, it is a compressed 16-bit instruction from the C extension).

There are six instruction *formats*; the register fields (rd, rs1, rs2)
always live in the same bit positions regardless of format:

```
bit:  31      25 24   20 19   15 14 12 11    7 6     0
       ┌────────┬───────┬───────┬────┬────────┬───────┐
R      │ funct7 │  rs2  │  rs1  │ f3 │   rd   │ opcode│
       ├────────┴┬──────┴───────┴────┴────────┴───────┤
I      │  imm[11:0]      │  rs1  │ f3 │   rd   │ opcode│
       ├─────────┴────────┴───────┴────┴────────┴───────┤
S      │imm[11:5]│  rs2  │  rs1  │ f3 │imm[4:0]│ opcode│
       ├─────────┴───────┴───────┴────┴────────┴───────┤
B      │i12│i[10:5]│rs2│rs1│ f3 │i[4:1]│i11│ opcode│
       ├───────────────────────────────────────────────┤
U      │         imm[31:12]         │   rd   │ opcode│
       ├───────────────────────────────────────────────┤
J      │i20│ i[10:1] │i11│i[19:12]│   rd   │ opcode│
       └───────────────────────────────────────────────┘
```

### Why are branch and jump immediates scrambled?
In B and J formats the bits of the immediate are spread across the instruction
in a non-obvious order. This is deliberate: it keeps rs1, rs2, rd, funct3
in fixed bit positions so hardware can start decoding registers before
knowing the instruction type. The bit-manipulation to reassemble them is
in `src/cpu/decoder.rs`.

### Sign extension
Immediates are *sign-extended* to 64 bits.
The function `sign_ext(x, bit_pos)` in the decoder shifts the value left
until the sign bit is at position 63, then shifts back arithmetically.
This propagates the sign bit into all the high bits.

Example: I-type imm = 12 bits. If bit 11 = 1 (negative number),
after sign extension to 64 bits all upper 52 bits are also 1.

### x0 is hardwired to zero
Writing to register x0 is silently discarded.
The `set!` macro in executor.rs checks `if rd != 0` before writing.
This lets encodings like `addi x0, x0, 0` serve as the canonical NOP.

---

## 3. The instruction lifecycle inside the VM

```
cpu.step(bus)
  ├─ check pending interrupts (MIE + MIP)
  ├─ fetch: bus.read32(pc) → u32
  ├─ decode: decoder::decode(raw) → Inst enum
  ├─ execute: executor::execute(inst, …) → ExecResult
  │     (modifies regs, reads/writes bus)
  ├─ if trap → handle_trap (sets mepc, mcause, pc ← mtvec)
  └─ pc ← next_pc
```

### Why a dedicated `Inst` enum?
The decoder turns a raw `u32` into a typed Rust enum variant
(`Inst::Add { rd, rs1, rs2 }` etc.).
This cleanly separates *what* the instruction means from *how* to execute it,
and makes the executor a straightforward match on variants with no bit twiddling.
The compiler also catches unhandled variants.

---

## 4. CSRs — Control and Status Registers

CSRs are a separate 4096-entry register file (addressed by a 12-bit index).
Key machine-mode CSRs used here:

| Address | Name      | Purpose                          |
|---------|-----------|----------------------------------|
| 0x300   | mstatus   | interrupt enable, privilege mode |
| 0x304   | mie       | interrupt enable bits            |
| 0x305   | mtvec     | trap vector base address         |
| 0x340   | mscratch  | scratch for trap handler         |
| 0x341   | mepc      | PC saved on trap entry           |
| 0x342   | mcause    | reason for the trap              |
| 0x344   | mip       | pending interrupt bits           |
| 0xC00   | cycle     | read-only cycle counter          |
| 0xC02   | instret   | instructions retired counter     |

CSR instructions (CSRRW, CSRRS, CSRRC and their immediate variants)
do a *read-modify-write* atomically. The old value is returned to `rd`
and the new value is computed from the operation and written back.

### mstatus bit layout (relevant bits)
```
bit  3: MIE   — machine interrupt enable (global)
bit  7: MPIE  — saved MIE on trap entry
bits 12:11 MPP — previous privilege mode saved on trap
```
When a trap fires: MPIE = MIE, MIE = 0 (interrupts disabled during handler).
`mret` reverses this: MIE = MPIE, MPIE = 1.

---

## 5. Traps and interrupts

A *trap* is either an *exception* (synchronous, caused by an instruction)
or an *interrupt* (asynchronous, from a timer or external device).

### Exception flow
1. Save PC → mepc
2. Save cause code → mcause (bit 63 = 0 for exceptions)
3. Save bad address (if applicable) → mtval
4. Update mstatus (save MIE → MPIE, clear MIE, set MPP)
5. Jump to mtvec (vectored or direct mode)

### Timer interrupt
The CLINT (Core Local Interruptor) has two 64-bit memory-mapped registers:
- `mtime` — increments every tick
- `mtimecmp` — write a future time here to schedule an interrupt

When `mtime >= mtimecmp`, the hardware sets MIP.MTIE.
If MIE.MTIE is also set and mstatus.MIE is set, the CPU takes a timer trap.

In this VM, mtime is incremented on every `cpu.step()` call via `bus.tick()`.
The MIP bit is wired in `main.rs` before each step.

---

## 6. Memory map — why these addresses?

The addresses are conventional for RISC-V SoC designs
(specifically the SiFive HiFive1 and virt QEMU machine):

| Device | Base       | Why there                                  |
|--------|------------|--------------------------------------------|
| CLINT  | 0x02000000 | standard RISC-V platform spec              |
| PLIC   | 0x0C000000 | standard RISC-V platform spec              |
| UART   | 0x10000000 | SiFive / QEMU virt convention              |
| DRAM   | 0x80000000 | above 2 GB — keeps low addresses for MMIO |

Programs expect to be loaded at 0x80000000 and find the UART at 0x10000000.
Real firmware (like OpenSBI) follows the same layout.

---

## 7. The `Device` trait — Rust trait design

```rust
pub trait Device {
    fn read32(&self, offset: u64) -> u32;
    fn write32(&mut self, offset: u64, val: u32);

    // Defaults — derived from the required methods above
    fn read8(&self, offset: u64) -> u8 { self.read32(offset) as u8 }
    fn write8(&mut self, offset: u64, val: u8) { self.write32(offset, val as u32); }
    fn read64(&self, offset: u64) -> u64 { … }
    fn write64(&mut self, offset: u64, val: u64) { … }
}
```

### Why a trait and not just methods on each struct?
- **Uniform interface** — the bus calls `device.read8()` identically for UART,
  CLINT, PLIC. No per-device if/else inside the bus dispatch.
- **Overridable defaults** — CLINT overrides `read64`/`write64` for correct
  64-bit register semantics. UART and PLIC inherit the defaults.
- **Testability** — you can write a mock device by `impl Device for MockDev`.
- **No trait objects needed** — `Bus` holds concrete types (`uart: Uart`, …),
  so there's no heap allocation and no vtable overhead. The trait is zero-cost.

### Why not `Box<dyn Device>`?
Dynamic dispatch (`dyn Trait`) requires a heap allocation and a vtable pointer.
In a `no_std` environment it would require `alloc`. Since we know all device
types at compile time, generic dispatch (monomorphisation) is cheaper and
simpler.

---

## 8. `no_std` — what it means and why

`#![no_std]` removes the Rust standard library from the crate.
The standard library depends on an OS for: memory allocation, threads,
file I/O, timers, panic formatting.

Without `std` you still have:
- **`core`** — language primitives, iterators, Option, Result, traits
- **`alloc`** — heap containers (Vec, String, Box) if an allocator is linked

The VM library (`src/lib.rs`) is `no_std + alloc` because:
1. A RISC-V program (the *guest*) may run on bare metal — no OS.
2. The VM itself should be embeddable in contexts without `std`
   (e.g., a firmware that hosts a RISC-V interpreter).
3. It forces discipline: no accidental `std::println!` in library code.

The binary (`src/main.rs`) uses `std` freely for file I/O, the CLI, etc.
Rust lets one crate be `no_std` while the binary it links into uses `std`.
`alloc` types (Vec, String) work because the binary's std provides the
global allocator.

---

## 9. Bit manipulation in the decoder

The decoder uses only bitwise operations — no parsing, no lookup tables.

```rust
fn bits(x: u32, lo: u32, hi: u32) -> u32 {
    (x >> lo) & ((1 << (hi - lo + 1)) - 1)
}
fn sign_ext(x: u32, bit_pos: u32) -> i64 {
    let shift = 63 - bit_pos;
    ((x as i64) << shift) >> shift
}
```

`bits(raw, 15, 19)` extracts the rs1 field.
`sign_ext(imm, 11)` sign-extends a 12-bit I-type immediate.

These inline functions compile to just a handful of CPU instructions.
Using a bitfield crate would add a dependency and hide the RISC-V spec;
doing it manually keeps the decoder readable alongside the ISA manual.

### Instruction fetch must be 32-bit aligned
RISC-V requires PC to be 4-byte aligned for base instructions.
An unaligned fetch would give a random pair of bytes from two instructions.
The executor masks JALR's target with `& !1` (clear lowest bit)
to prevent 2-byte misalignment.

---

## 10. Test strategy

### Unit tests (`tests/cpu_tests.rs`)
Each test encodes a hand-crafted instruction (or short sequence) as raw `u32`
values, loads them into DRAM, runs the VM, and checks registers.
This catches: wrong sign extension, wrong immediates, wrong ALU operation.

Why hand-encode instead of using an assembler?
- No toolchain dependency for running tests
- Forces understanding of the encoding
- Makes encoding bugs visible (e.g., the EBREAK bug where the check was
  `raw >> 7 == 0x001` instead of `raw == 0x00100073`)

### Integration tests (`tests/integration_tests.rs`)
Multi-instruction programs: iterative fibonacci, store/load roundtrip.
These verify that branches, jumps, and memory work *together*.

### Fault injection tests
Deliberately corrupt or OOB instructions:
- `0x0000_0002` — opcode 0x02 = undefined → `Inst::Illegal`
- `ld x1, 0(x0)` — address 0 is not DRAM → `TrapCause::LoadAccessFault`
- `sw x0, 0(x0)` → `TrapCause::StoreAccessFault`

Verifying that traps fire correctly and don't corrupt other registers.

### Memory/bus tests (`tests/mmu_tests.rs`)
Boundary conditions: OOB reads/writes, byte/word/dword granularity,
UART write→read cycle, CLINT mtime advancing.

---

## 11. Rust idioms used and why

### `Result<T, TrapCause>` instead of panicking
Bus reads/writes return `Result`. This means:
- Bad addresses propagate cleanly to the CPU as traps
- No unwanted panics in library code
- The CPU decides what to do (trap the guest, not crash the host)

### `match` on enums
The decoder returns `Inst::Add { rd, rs1, rs2 }` — a named-field enum variant.
The executor `match`es on it. Rust ensures all variants are handled;
missing one is a compile error.

### Macros for register access
```rust
macro_rules! reg { ($r:expr) => { regs[$r as usize] }; }
macro_rules! set { ($r:expr, $v:expr) => { if $r != 0 { regs[$r as usize] = $v; } }; }
```
These avoid repetitive index casts and the x0-is-zero check everywhere.
Macros expand inline — zero overhead.

### `wrapping_add` for PC arithmetic
PC arithmetic wraps in 64-bit space: `0xFFFF_FFFF_8000_0010`.
Using `+` would panic in debug mode on overflow; `wrapping_add` is explicit
and correct by the RISC-V spec.

### `i64 as u64` for sign-extended immediates
When an immediate is negative (e.g., `addi x1, x0, -1`), the Rust type
`i64` correctly holds -1 = `0xFFFF_FFFF_FFFF_FFFF` as a u64 bit pattern.
`imm as u64` then passes this bit pattern to `wrapping_add`.

---

## 12. The EBREAK / ECALL decoder bug (a real debugging story)

Initial decoder:
```rust
0x0 => match raw >> 7 {
    0x000 => Inst::Ecall,
    0x001 => Inst::Ebreak,  // BUG
    …
}
```

ECALL   = `0x0000_0073`; `0x73 >> 7 = 0`  → matched 0x000 ✓  
EBREAK  = `0x0010_0073`; `0x0010_0073 >> 7 = 0x2000` → fell to `Illegal` ✗

The fibonacci integration test ran 10,000 steps without halting because
every `ebreak` was decoded as `Illegal`, which caused a trap → PC jumped
to mtvec (= 0, outside DRAM) → fetch fault → another trap → infinite loop.

Fix: match the full 32-bit encoding:
```rust
0x0 => match raw {
    0x0000_0073 => Inst::Ecall,
    0x0010_0073 => Inst::Ebreak,
    0x3020_0073 => Inst::Mret,
    _           => Inst::Illegal(raw),
},
```

Lesson: when decoding instructions, prefer matching the *full* known encoding
rather than a shifted sub-field, unless you are absolutely sure of the mask.

---

## 13. Where to go next

- **RV64A** (atomics): LR/SC, AMO instructions — needed for `spinlock` in OS
- **Sv39 virtual memory**: 39-bit virtual address space, 3-level page table
- **S-mode / U-mode**: supervisor and user privilege levels, `sret`
- **OpenSBI**: the standard RISC-V firmware, can run on this VM once
  virtual memory and S-mode are in place
- **Linux**: after OpenSBI, Linux can boot with virtio block + network
- **GDB RSP stub**: TCP socket at port 1234, `riscv64-unknown-elf-gdb` connects
- **JIT compilation**: translate RISC-V basic blocks to host machine code
  for 10–100× speedup (see `rvjit`)
