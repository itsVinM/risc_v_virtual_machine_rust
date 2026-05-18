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

## 13. The bare-metal kernel — what we built and why

`kernel/` is a tiny Rust crate that runs directly on the VM in M-mode.
It is the first real "software" the VM executes — not a hand-encoded demo,
but compiled Rust + assembly producing an ELF binary.

### What the kernel does

```
_start (M-mode, PC = 0x80000000)
  │
  ├── li sp, 0x87FFFFF0      set stack near top of DRAM
  ├── la t0, _trap           compute trap handler address (auipc)
  ├── csrw mtvec, t0         install handler (direct mode)
  ├── arm CLINT timer        mtimecmp = mtime + 500
  ├── csrs mie, 0x80         enable MTIE (machine timer interrupt enable)
  ├── csrs mstatus, 0x8      set MIE (global interrupt enable)
  │
  └── spin: la t0, TICKS; ld t1, 0(t0); blt t1, 5, spin
           │                            │
           │ (interrupt fires every 500 ticks)
           └── _trap:
                  re-arm timer (mtimecmp += 500)
                  TICKS++
                  mret
  │
  └── mv a0, t1; ebreak   → VM prints "halted a0=5 cycles=2574"
```

Timer interrupt delivery path step-by-step:
1. `bus.tick()` increments `clint.mtime` by 1 each cpu step.
2. `clint.timer_pending()` → true when `mtime >= mtimecmp`.
3. `vm_tick()` sets `MIP.MTIE` in the CSR file.
4. `cpu.step()` calls `pending_interrupt(mstatus, mie, mip)`:
   checks `mstatus.MIE && mie.MTIE && mip.MTIE` → fires `TimerInterrupt`.
5. `handle_trap()` saves PC → mepc, sets mcause, jumps to mtvec.
6. Trap handler re-arms timer and returns with `mret`.
7. `mret` restores PC ← mepc, MIE ← MPIE.

### The C-extension debugging story

The kernel was compiled for `riscv64gc-unknown-none-elf`.
The `c` in `gc` means the **C compressed instruction extension** — 16-bit
encodings for common instructions. Our VM only decodes 32-bit instructions,
so the kernel crashed immediately with `IllegalInstruction` at PC=0.

Symptoms: `trap IllegalInstruction(0x016E4145)  pc=0x00000000`
- `0x016E4145` has opcode bits[1:0] = 01, not 11. That means it's a 16-bit
  compressed instruction. The VM decoded it as garbage.
- PC=0 because handle_trap() jumps to mtvec, which was never set (trap happened
  before csrw mtvec), so PC went to 0 (default mtvec = 0, outside DRAM).

Fix: add `.option norvc` at the start of `global_asm!` blocks.
`-C target-feature=-c` in rustflags does NOT propagate into `global_asm!` —
the assembler defaults to the full target ISA unless explicitly told otherwise.

Lesson: always check whether your assembler generates compressed instructions
when your runtime doesn't support them. `objdump -d` reveals this instantly.

### ELF loading

The VM now detects ELF files by magic (`\x7fELF`) and parses `PT_LOAD`
segments instead of treating the file as a flat binary.

Key ELF64 offsets used:
```
0x18 (24): e_entry      — entry point (u64)
0x20 (32): e_phoff      — program header table offset (u64)
0x36 (54): e_phentsize  — size of each program header entry (u16)
0x38 (56): e_phnum      — number of program header entries (u16)

Per program header (56 bytes each):
  0x00: p_type   (u32) — 1 = PT_LOAD
  0x08: p_offset (u64) — file offset of segment data
  0x18: p_paddr  (u64) — physical load address
  0x20: p_filesz (u64) — size in file (actual bytes to copy)
  0x28: p_memsz  (u64) — size in memory (memsz - filesz = BSS zero-fill)
```

The loader copies each PT_LOAD segment into the flat DRAM buffer at
`paddr - DRAM_BASE`. Segments outside DRAM range are skipped silently.
BSS (memsz > filesz) is already zero because the buffer is zeroed at init.

The 64-bit value loading trick: `u64::from_le_bytes(slice.try_into().ok()?)` 
with `?` propagates None up to load_elf() — any malformed ELF causes a safe
fallback to flat binary mode, never a panic.

---

## 14. Real-time systems, timing, and why this matters (Rapita context)

This section bridges what we built to concepts in **timing analysis of
safety-critical real-time embedded systems** — relevant to work at companies
like Rapita Systems (DO-178C, WCET analysis, MPSoC).

### What is WCET and why does the VM help understand it?

**Worst-Case Execution Time (WCET)** is the maximum time a task can ever take.
In safety-critical systems (avionics, automotive), you must *prove* a task
always finishes before its deadline. The VM is a simplified model of exactly
the hardware concepts involved:

- Each `cpu.step()` is one "tick" — deterministic, no cache miss, no pipeline.
  This is the idealized model WCET tools start from.
- Real hardware adds: branch prediction, instruction cache, data cache,
  out-of-order execution, write buffers, prefetchers — all non-deterministic.
- MPSoCs add: shared bus contention, shared LLC, coherency traffic.
  These are the *hard problems* Rapita analyzes.

### The CLINT timer as a hardware timing reference

Our CLINT is a simplified version of the real RISC-V CLINT:
- `mtime` is a free-running counter, incremented by the platform.
- `mtimecmp` is the comparator — write a deadline here.
- When `mtime >= mtimecmp`, `MIP.MTIE` is set → timer interrupt.

In real systems, `mtime` runs at a fixed reference clock (e.g. 1 MHz on
SiFive boards). It is the *timebase* used to measure real-time deadlines.
Rapita's MACH178 tooling instruments code and uses hardware timers like
this to measure actual execution times.

### Interrupt latency — what our kernel shows

The kernel arms the timer and then spins. The time between "timer fires" and
"trap handler starts" is **interrupt latency**. In our VM it is always 1 cycle
(deterministic). On real hardware it depends on:
- Pipeline depth (instructions in-flight when interrupt arrives)
- Memory access in progress (cache miss adds latency)
- Other interrupts already being handled (non-preemptible critical sections)

WCET tools must account for worst-case interrupt latency. Our VM teaches the
mechanics; real MPSoC analysis teaches the variability.

### M-mode vs S-mode vs U-mode — privilege levels in safety systems

```
M-mode (Machine)     — firmware, OpenSBI, trap handling
  │ can delegate traps via medeleg/mideleg
S-mode (Supervisor)  — OS kernel (Linux, VxWorks, PikeOS, Integrity)
  │ manages U-mode processes
U-mode (User)        — application code
```

Our VM is M-mode only. Safety RTOSes (PikeOS, Integrity, DEOS) typically
run in S-mode on top of an M-mode firmware (OpenSBI or a custom BIOS).
Hypervisors add an H (hypervisor) extension between M and S.

### MPSoC multicore challenges

The job description mentions QorIQ, UltraScale, Jacinto, TriCore, RISC-V.
Common multicore problems in safety-critical systems:

| Problem             | Description                                                  |
|---------------------|--------------------------------------------------------------|
| Shared bus contention | Two cores access DRAM simultaneously → one stalls           |
| Cache coherency      | Core 0 writes, Core 1 reads stale data → MESI protocol      |
| Shared LLC pollution | Core 1 evicts Core 0's working set from L3 cache            |
| Interrupt routing    | Which core handles an interrupt? (PLIC routes to a hart)    |
| Lock-step vs lockless | Safety requires determinism; performance wants concurrency  |

Our VM has a **PLIC stub** — the Platform-Level Interrupt Controller that
in a real MPSoC routes external interrupts to specific harts (CPU cores).
Rapita's work is about measuring and bounding the *timing impact* of these
interference effects on each core's WCET.

### DO-178C and software levels

DO-178C is the avionics software standard:

| Level | Consequence of failure         | What it requires                           |
|-------|--------------------------------|--------------------------------------------|
| A     | Catastrophic (loss of aircraft)| Full structural coverage (MC/DC)           |
| B     | Hazardous                      | Decision + condition coverage              |
| C     | Major                          | Statement + branch coverage                |
| D     | Minor                          | Statement coverage                         |

WCET is required at Level A/B. Rapita's RVS tool automates coverage collection
and WCET measurement. DO-254 is the equivalent standard for hardware (FPGAs).

AMC 20-193 specifically addresses multicore processors in aviation.

---

## 15. Expanding the kernel — roadmap

The current kernel lives at `kernel/src/main.rs`. Each step below adds one
concept and can be tested in isolation on the VM.

### Step 1: Add UART output (re-add UART device to VM)

Right now the kernel has no output. Add back the UART device (16550):
- In the VM: restore `src/devices/uart.rs` and `UART_BASE` in bus.rs.
- In the kernel: write to UART_BASE + 0 to send bytes.
- Result: kernel can print `"Hello from M-mode!\n"` via UART.

This is the most satisfying quick win — seeing text output from your kernel.

### Step 2: Multiple interrupt sources (software + timer)

Add a software interrupt handler alongside the timer:
- Write `1` to `CLINT_BASE + 0` (msip register) to trigger software interrupt.
- Handler checks `mcause`: timer = `(1<<63)|7`, software = `(1<<63)|3`.
- Tests `mie.MSIE` enable bit separately.

### Step 3: Trap table (vectored mode)

Change mtvec to vectored mode: `csrw mtvec, (base | 1)`.
In vectored mode, each interrupt cause gets its own entry at `base + 4*cause`.
This eliminates the mcause dispatch inside one flat handler.

### Step 4: Context switching (two "tasks")

Set up two stack areas. On each timer interrupt, save all 32 registers to
the current task's stack frame, switch to the other task's saved frame,
restore registers, mret to the other task's PC. This is the core of any RTOS.

Key data structure — the task control block (TCB):
```rust
struct Tcb {
    regs: [u64; 32],   // saved register file
    pc:   u64,          // saved program counter
}
static mut TASKS: [Tcb; 2] = ...;
static mut CURRENT: usize = 0;
```

### Step 5: Cycle-accurate timing measurement

In the handler, read `minstret` before and after each task to measure
exactly how many instructions it executed. This is the toy version of
what Rapita's MACH178 does for WCET evidence.

```asm
csrr t0, minstret      # instructions retired counter
# ... task runs ...
csrr t1, minstret
sub  t0, t1, t0        # delta = instructions this quantum
```

### Step 6: S-mode + SBI (the Linux prerequisite)

To run a real OS kernel:
1. Add S-mode CSRs: sstatus, sepc, scause, stvec, sip, sie, satp (0x180).
2. Add `medeleg`/`mideleg` CSRs: delegate exceptions/interrupts to S-mode.
3. Add `sret` instruction: restores PC←sepc, priv←sstatus.SPP.
4. Add SBI ecall handler: M-mode handles `ecall` from S-mode (console, timer).
5. Add Sv39 page walker: when satp.MODE=8, translate VA→PA via 3-level table.
6. Load kernel image + device tree blob (DTB) at known DRAM addresses.

---

## 16. Where to go next

- **RV64A** (atomics): LR/SC, AMO instructions — needed for `spinlock` in OS
- **Sv39 virtual memory**: 39-bit virtual address space, 3-level page table
- **S-mode / U-mode**: supervisor and user privilege levels, `sret`
- **OpenSBI**: the standard RISC-V firmware, can run on this VM once
  virtual memory and S-mode are in place
- **Linux**: after OpenSBI, Linux can boot with virtio block + network
- **GDB RSP stub**: TCP socket at port 1234, `riscv64-unknown-elf-gdb` connects
- **JIT compilation**: translate RISC-V basic blocks to host machine code
  for 10–100× speedup (see `rvjit`)
