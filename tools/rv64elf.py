"""rv64elf — minimal little-endian ELF64 reader for RISC-V (rv64vm host tooling).

"""

from __future__ import annotations

import struct
from dataclasses import dataclass

EM_RISCV = 0xF3
PT_LOAD = 1
PF_X = 1
PF_W = 2
PF_R = 4
SHN_UNDEF = 0


class ElfError(Exception):
    """Raised when a buffer is not a usable RISC-V ELF64 image."""

    Truncated = "file is truncated"
    BadMagic = "not an ELF (bad magic)"
    Not64Bit = "not a 64-bit ELF"
    NotLittleEndian = "not little-endian"
    NotRiscV = "not RISC-V (e_machine != 0xF3)"
    BadPhdrs = "program header table out of bounds"
    BadShdrs = "section header table out of bounds"
    BadShstrtab = "section string table out of bounds"


@dataclass(frozen=True)
class Ehdr:
    e_type: int
    e_machine: int
    e_version: int
    e_entry: int
    e_phoff: int
    e_shoff: int
    e_flags: int
    e_ehsize: int
    e_phentsize: int
    e_phnum: int
    e_shentsize: int
    e_shnum: int
    e_shstrndx: int


@dataclass(frozen=True)
class Phdr:
    p_type: int
    p_flags: int
    p_offset: int
    p_vaddr: int
    p_paddr: int
    p_filesz: int
    p_memsz: int
    p_align: int


@dataclass(frozen=True)
class Shdr:
    sh_name: int
    sh_type: int
    sh_flags: int
    sh_addr: int
    sh_offset: int
    sh_size: int
    sh_link: int
    sh_info: int
    sh_addralign: int
    sh_entsize: int


_EHDR = struct.Struct("<HHIQQQIHHHHHH")   # fields from offset 16
_PHDR = struct.Struct("<IIQQQQQQ")
_SHDR = struct.Struct("<IIQQQQIIQQ")


def _read_table(data: bytes, off: int, ent_size: int, count: int) -> bytes:
    if count == 0:
        return b""
    if ent_size == 0 or off > len(data):
        raise ElfError(ElfError.BadPhdrs)
    need = count * ent_size
    if need > len(data) - off:
        raise ElfError(ElfError.BadPhdrs)
    return data[off : off + need]


class Elf:
    __slots__ = ("_data", "_ehdr", "_phdrs", "_shdrs", "_shstrtab")

    def __init__(self, data: memoryview | bytes):
        self._data = bytes(data)
        self._ehdr: Ehdr | None = None
        self._phdrs: list[Phdr] = []
        self._shdrs: list[Shdr] = []
        self._shstrtab = b""

    #Parsing
    @staticmethod
    def open(data: memoryview | bytes) -> "Elf":
        """Parse a RISC-V ELF64 image; raises ElfError on any problem."""
        buf = bytes(data)
        if len(buf) < 64:
            raise ElfError(ElfError.Truncated)
        if buf[0:4] != b"\x7fELF":
            raise ElfError(ElfError.BadMagic)
        if buf[4] != 2:
            raise ElfError(ElfError.Not64Bit)
        if buf[5] != 1:
            raise ElfError(ElfError.NotLittleEndian)

        (e_type, e_machine, e_version, e_entry, e_phoff, e_shoff, e_flags,
         e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum,
         e_shstrndx) = _EHDR.unpack_from(buf, 16)
        ehdr = Ehdr(e_type, e_machine, e_version, e_entry, e_phoff, e_shoff,
                    e_flags, e_ehsize, e_phentsize, e_phnum, e_shentsize,
                    e_shnum, e_shstrndx)

        if ehdr.e_machine != EM_RISCV:
            raise ElfError(ElfError.NotRiscV)

        elf = Elf(buf)
        elf._ehdr = ehdr

        try:
            phdr_bytes = _read_table(buf, ehdr.e_phoff, ehdr.e_phentsize, ehdr.e_phnum)
        except ElfError:
            raise ElfError(ElfError.BadPhdrs) from None
        if ehdr.e_phnum and ehdr.e_phentsize < _PHDR.size:
            raise ElfError(ElfError.BadPhdrs)
        for i in range(ehdr.e_phnum):
            elf._phdrs.append(Phdr(*_PHDR.unpack_from(phdr_bytes, i * ehdr.e_phentsize)))

        try:
            shdr_bytes = _read_table(buf, ehdr.e_shoff, ehdr.e_shentsize, ehdr.e_shnum)
        except ElfError:
            raise ElfError(ElfError.BadShdrs) from None
        if ehdr.e_shnum and ehdr.e_shentsize < _SHDR.size:
            raise ElfError(ElfError.BadShdrs)
        for i in range(ehdr.e_shnum):
            elf._shdrs.append(Shdr(*_SHDR.unpack_from(shdr_bytes, i * ehdr.e_shentsize)))

        if ehdr.e_shstrndx != SHN_UNDEF and ehdr.e_shstrndx < ehdr.e_shnum:
            strtab = elf._shdrs[ehdr.e_shstrndx]
            try:
                elf._shstrtab = _read_table(buf, strtab.sh_offset, 1, strtab.sh_size)
            except ElfError:
                raise ElfError(ElfError.BadShstrtab) from None

        return elf

    #Accessors
    @property
    def header(self) -> Ehdr:
        return self._ehdr

    @property
    def phdrs(self) -> list[Phdr]:
        return self._phdrs

    @property
    def shdrs(self) -> list[Shdr]:
        return self._shdrs

    def load_segments(self) -> list[Phdr]:
        return [p for p in self._phdrs if p.p_type == PT_LOAD]

    def named_sections(self) -> list[tuple[str, Shdr]]:
        pairs = []
        for s in self._shdrs:
            name = self.section_name(s)
            if name:
                pairs.append((name, s))
        return pairs

    def section_name(self, sh: Shdr) -> str:
        tab = self._shstrtab
        if not tab or sh.sh_name >= len(tab):
            return ""
        end = tab.find(b"\x00", sh.sh_name)
        if end < 0:
            end = len(tab)
        return tab[sh.sh_name:end].decode("utf-8", errors="replace")

    def loadable_in(self, base: int, end: int) -> bool:
        """True if every PT_LOAD segment fits inside [base, end)."""
        for p in self._phdrs:
            if p.p_type != PT_LOAD:
                continue
            if p.p_vaddr < base or p.p_vaddr + p.p_memsz > end:
                return False
        return True
