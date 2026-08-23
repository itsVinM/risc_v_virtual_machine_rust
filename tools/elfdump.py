#!/usr/bin/env python3
"""elfdump — inspect a little-endian RISC-V ELF64 and optionally validate it
against the rv64vm DRAM window.

    elfdump.py <file>              print header, program headers, sections
    elfdump.py --check <file>      also verify loadability into DRAM (nonzero
                                   exit code on violation)
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from rv64elf import PF_R, PF_W, PF_X, Elf, ElfError

DRAM_BASE = 0x8000_0000
DRAM_END = 0x8800_0000

PHDR_TYPES = {
    0: "NULL",
    1: "LOAD",
    2: "DYNAMIC",
    3: "INTERP",
    4: "NOTE",
    5: "SHLIB",
    6: "PHDR",
    7: "TLS",
}

SHDR_TYPES = {
    0: "NULL",
    1: "PROGBITS",
    2: "SYMTAB",
    3: "STRTAB",
    4: "RELA",
    5: "HASH",
    6: "DYNAMIC",
    7: "NOTE",
    8: "NOBITS",
    9: "REL",
    0x7000_0003: "RISCV_ATTRIBUTE",
}


def phdr_type(t: int) -> str:
    return PHDR_TYPES.get(t, "OTHER")


def shdr_type(t: int) -> str:
    return SHDR_TYPES.get(t, "OTHER")


def flags_str(f: int) -> str:
    return ("R" if f & PF_R else "-") + ("W" if f & PF_W else "-") + ("X" if f & PF_X else "-")


def dump(elf: Elf, check: bool) -> int:
    h = elf.header

    print("ELF header")
    print(f"  type      {h.e_type} (ET_EXEC)")
    print(f"  machine   {h.e_machine} (RISC-V)")
    print(f"  entry     {h.e_entry:#018x}")
    print(f"  phoff     {h.e_phoff:#018x}  shoff {h.e_shoff:#018x}")
    print(f"  phnum     {h.e_phnum}  shnum {h.e_shnum}  shstrndx {h.e_shstrndx}")

    print("\nProgram headers")
    print(f"  {'TYPE':<8} {'OFFSET':<8} {'VADDR':<16} {'PADDR':<16} "
          f"{'FILESZ':<10} {'MEMSZ':<10} {'FLAGS':<6} ALIGN")

    def phdr_row(p) -> str:
        return (f"  {phdr_type(p.p_type):<8} {p.p_offset:<8x} {p.p_vaddr:#016x} "
                f"{p.p_paddr:#016x} {p.p_filesz:<10x} {p.p_memsz:<10x} "
                f"{flags_str(p.p_flags)} {p.p_align:#x}")

    for p in elf.load_segments():
        print(phdr_row(p))
    for p in elf.phdrs:
        if p.p_type != 1:  # PT_LOAD already printed above
            print(phdr_row(p))

    print("\nSections")
    print(f"  [{'Nr':<3}] {'NAME':<16} {'TYPE':<14} {'ADDR':<16} "
          f"{'OFFSET':<10} {'SIZE':<10} FLAGS")

    for i, s in enumerate(elf.shdrs):
        fl = (("X" if s.sh_flags & 1 else "-") +
              ("W" if s.sh_flags & 2 else "-") +
              ("A" if s.sh_flags & 4 else "-"))
        name = elf.section_name(s)
        print(f"  [{i:>3}] {name:<16} {shdr_type(s.sh_type):<14} {s.sh_addr:#016x} "
              f"{s.sh_offset:#010x} {s.sh_size:#010x} {fl}")

    if check:
        ok = elf.loadable_in(DRAM_BASE, DRAM_END)
        print(f"\nVM check (DRAM {DRAM_BASE:#018x}..{DRAM_END:#018x}): "
              f"{'OK' if ok else 'FAIL'}")
        return 0 if ok else 1
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true",
                    help="verify loadability into the VM DRAM window")
    ap.add_argument("file", type=Path, help="ELF file to inspect")
    args = ap.parse_args()

    try:
        data = args.file.read_bytes()
    except OSError as e:
        print(f"elfdump: cannot open '{args.file}': {e.strerror}", file=sys.stderr)
        return 2

    try:
        elf = Elf.open(data)
    except ElfError as e:
        print(f"elfdump: {e}", file=sys.stderr)
        return 1

    return dump(elf, args.check)


if __name__ == "__main__":
    raise SystemExit(main())
