#!/usr/bin/env python3
"""elfdump — inspect a little-endian RISC-V ELF64 and optionally validate it
against the rv64vm DRAM window.

    elfdump.py <file>              print header, program headers, sections
    elfdump.py --check <file>      also verify loadability into DRAM (nonzero
                                   exit code on violation)
"""

from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from rv64elf import PF_R, PF_W, PF_X, Elf, ElfError

log = logging.getLogger(__name__)

DRAM_BASE = 0x8000_0000
DRAM_END = 0x8800_0000


class Types:
    PHDR = {
        0: "NULL", 1: "LOAD", 2: "DYNAMIC", 3: "INTERP",
        4: "NOTE", 5: "SHLIB", 6: "PHDR", 7: "TLS",
    }

    SHDR = {
        0: "NULL", 1: "PROGBITS", 2: "SYMTAB", 3: "STRTAB",
        4: "RELA", 5: "HASH", 6: "DYNAMIC", 7: "NOTE",
        8: "NOBITS", 9: "REL", 0x7000_0003: "RISCV_ATTRIBUTE",
    }

    @staticmethod
    def phdr(value: int) -> str:
        return Types.PHDR.get(value, "OTHER")

    @staticmethod
    def shdr(value: int) -> str:
        return Types.SHDR.get(value, "OTHER")

    @staticmethod
    def flags(value: int) -> str:
        return (
            ("R" if value & PF_R else "-")
            + ("W" if value & PF_W else "-")
            + ("X" if value & PF_X else "-")
        )


class ElfDumper:
    def __init__(self, elf: Elf, *, check: bool = False) -> None:
        self._elf = elf
        self._check = check

    def _print_header(self) -> None:
        h = self._elf.header
        log.info("ELF header")
        log.info("  type      %s (ET_EXEC)", h.e_type)
        log.info("  machine   %s (RISC-V)", h.e_machine)
        log.info("  entry     %#018x", h.e_entry)
        log.info("  phoff     %#018x  shoff %#018x", h.e_phoff, h.e_shoff)
        log.info("  phnum     %d  shnum %d  shstrndx %d",
                 h.e_phnum, h.e_shnum, h.e_shstrndx)

    def _print_phdrs(self) -> None:
        log.info("")
        log.info("Program headers")
        log.info("  %-8s %-8s %-16s %-16s %-10s %-10s %-6s %s",
                 "TYPE", "OFFSET", "VADDR", "PADDR",
                 "FILESZ", "MEMSZ", "FLAGS", "ALIGN")
        for seg in self._elf.load_segments():
            self._log_phdr_row(seg)
        for ph in self._elf.phdrs:
            if ph.p_type != 1:
                self._log_phdr_row(ph)

    @staticmethod
    def _log_phdr_row(ph) -> None:
        log.info(
            "  %-8s %-8x %#016x %#016x %-10x %-10x %s %#x",
            Types.phdr(ph.p_type), ph.p_offset, ph.p_vaddr,
            ph.p_paddr, ph.p_filesz, ph.p_memsz,
            Types.flags(ph.p_flags), ph.p_align,
        )

    def _print_shdrs(self) -> None:
        log.info("")
        log.info("Sections")
        log.info("  [%-3s] %-16s %-14s %-16s %-10s %-10s %s",
                 "Nr", "NAME", "TYPE", "ADDR", "OFFSET", "SIZE", "FLAGS")
        for idx, s in enumerate(self._elf.shdrs):
            fl = (
                ("X" if s.sh_flags & 1 else "-")
                + ("W" if s.sh_flags & 2 else "-")
                + ("A" if s.sh_flags & 4 else "-")
            )
            name = self._elf.section_name(s)
            log.info("  [%3d] %-16s %-14s %#016x %#010x %#010x %s",
                     idx, name, Types.shdr(s.sh_type),
                     s.sh_addr, s.sh_offset, s.sh_size, fl)

    def _check_dram(self) -> int:
        ok = self._elf.loadable_in(DRAM_BASE, DRAM_END)
        log.info("")
        log.info("VM check (DRAM %#018x..%#018x): %s",
                 DRAM_BASE, DRAM_END, "OK" if ok else "FAIL")
        return 0 if ok else 1

    def dump(self) -> int:
        self._print_header()
        self._print_phdrs()
        self._print_shdrs()
        if self._check:
            return self._check_dram()
        return 0


def main() -> int:
    arg = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    arg.add_argument(
        "--check", action="store_true",
        help="verify loadability into VM DRAM window",
    )
    arg.add_argument("file", type=Path, help="ELF file to inspect")
    args = arg.parse_args()

    logging.basicConfig(format="%(message)s", level=logging.INFO)

    try:
        data = args.file.read_bytes()
    except OSError as e:
        log.error("elfdump: cannot open '%s': %s", args.file, e.strerror)
        return 2

    try:
        elf = Elf.open(data)
    except ElfError as e:
        log.error("elfdump: %s", e)
        return 1

    return ElfDumper(elf, check=args.check).dump()


if __name__ == "__main__":
    raise SystemExit(main())
