"""Unit tests for the rv64elf ELF reader. Fixtures are assembled in memory so
the suite is self-contained.

Run:  python3 -m unittest tests.test_elf -v     (from tools/)
"""

from __future__ import annotations

import struct
import unittest

import rv64elf
from rv64elf import Elf, ElfError


def build(*, vaddr: int = 0x8020_0000, memsz: int = 0x1000,
          with_shstrtab: bool = False) -> bytes:
    """Assemble a minimal one-PT_LOAD ELF64 image (mirrors the old C++ fixture)."""
    v = bytearray(64)
    v[0:4] = b"\x7fELF"
    v[4] = 2                       # EI_CLASS: 64-bit
    v[5] = 1                       # EI_DATA: little-endian
    v[6] = 1                       # EI_VERSION
    struct.pack_into("<H", v, 16, 2)          # e_type: ET_EXEC
    struct.pack_into("<H", v, 18, 0xF3)       # e_machine: RISC-V
    struct.pack_into("<I", v, 20, 1)          # e_version
    struct.pack_into("<Q", v, 24, vaddr)      # e_entry
    struct.pack_into("<Q", v, 32, 64)         # e_phoff
    struct.pack_into("<H", v, 52, 64)         # e_ehsize
    struct.pack_into("<H", v, 54, 56)         # e_phentsize
    struct.pack_into("<H", v, 56, 1)          # e_phnum
    struct.pack_into("<H", v, 58, 64)         # e_shentsize

    # One PT_LOAD, filesz == 16 payload bytes at the end.
    v += bytes(56 + 16)
    p = 64
    struct.pack_into("<I", v, p,      1)              # p_type: PT_LOAD
    struct.pack_into("<I", v, p + 4,  5)              # p_flags: R | X
    struct.pack_into("<Q", v, p + 8,  64 + 56)        # p_offset
    struct.pack_into("<Q", v, p + 16, vaddr)          # p_vaddr
    struct.pack_into("<Q", v, p + 24, vaddr)          # p_paddr
    struct.pack_into("<Q", v, p + 32, 16)             # p_filesz
    struct.pack_into("<Q", v, p + 40, memsz)          # p_memsz
    struct.pack_into("<Q", v, p + 48, 0x1000)         # p_align

    if with_shstrtab:
        strtab_off = len(v)
        names = b"\x00.text\x00.bss\x00"
        v += names
        shdr_off = len(v)
        struct.pack_into("<Q", v, 40, shdr_off)
        struct.pack_into("<H", v, 58, 64)
        struct.pack_into("<H", v, 60, 2)              # e_shnum
        struct.pack_into("<H", v, 62, 1)              # e_shstrndx
        for idx in range(2):
            sh = len(v)
            v += bytes(64)
            struct.pack_into("<I", v, sh,     1 if idx == 0 else 7)   # sh_name
            struct.pack_into("<I", v, sh + 4, 1 if idx == 0 else 8)   # sh_type
            struct.pack_into("<Q", v, sh + 8, 6 if idx == 0 else 2)   # sh_flags
            struct.pack_into("<Q", v, sh + 16, vaddr + idx * 0x800)   # sh_addr
            struct.pack_into("<Q", v, sh + 24,
                             strtab_off if idx == 1 else 0)           # sh_offset
            struct.pack_into("<Q", v, sh + 32,
                             len(names) if idx == 1 else 0x800)       # sh_size
    return bytes(v)


class ElfReaderTests(unittest.TestCase):

    def test_parse_ok(self):
        elf = Elf.open(build(vaddr=0x8020_0000, memsz=0x1000))
        h = elf.header
        self.assertEqual(h.e_machine, rv64elf.EM_RISCV)
        self.assertEqual(h.e_entry, 0x8020_0000)
        self.assertEqual(h.e_phnum, 1)
        self.assertEqual(len(elf.phdrs), 1)
        self.assertEqual(elf.phdrs[0].p_type, rv64elf.PT_LOAD)
        self.assertEqual(elf.phdrs[0].p_memsz, 0x1000)

    def test_loadable_window_ok(self):
        elf = Elf.open(build())
        self.assertTrue(elf.loadable_in(0x8000_0000, 0x8800_0000))

    def test_loadable_window_fail(self):
        elf = Elf.open(build(memsz=0x0801_0000))
        self.assertFalse(elf.loadable_in(0x8000_0000, 0x8800_0000))

    def test_bad_magic(self):
        data = bytearray(build())
        data[0] = 0x00
        with self.assertRaisesRegex(ElfError, "bad magic"):
            Elf.open(data)

    def test_not_64bit(self):
        data = bytearray(build())
        data[4] = 1
        with self.assertRaisesRegex(ElfError, "64-bit"):
            Elf.open(data)

    def test_not_little_endian(self):
        data = bytearray(build())
        data[5] = 2
        with self.assertRaisesRegex(ElfError, "little-endian"):
            Elf.open(data)

    def test_not_riscv(self):
        data = bytearray(build())
        struct.pack_into("<H", data, 18, 0x3E)    # x86-64
        with self.assertRaisesRegex(ElfError, "RISC-V"):
            Elf.open(data)

    def test_truncated_short(self):
        with self.assertRaisesRegex(ElfError, "truncated"):
            Elf.open(b"\x7fELF")

    def test_truncated_phdrs(self):
        data = build()[:64 + 8]
        with self.assertRaisesRegex(ElfError, "program header"):
            Elf.open(data)

    def test_section_names(self):
        elf = Elf.open(build(with_shstrtab=True))
        self.assertEqual(len(elf.shdrs), 2)
        self.assertEqual(elf.section_name(elf.shdrs[0]), ".text")
        self.assertEqual(elf.section_name(elf.shdrs[1]), ".bss")

    def test_load_segments_filter_and_ordering(self):
        elf = Elf.open(build())
        loads = elf.load_segments()
        self.assertEqual(len(loads), 1)
        self.assertEqual(loads[0], elf.phdrs[0])

    def test_named_sections(self):
        elf = Elf.open(build(with_shstrtab=True))
        self.assertEqual([n for n, _ in elf.named_sections()], [".text", ".bss"])


if __name__ == "__main__":
    unittest.main()
