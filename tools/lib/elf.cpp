#include "elf.hpp"

#include <algorithm>
#include <format>
#include <functional>
#include <ranges>

namespace rv64 {

// ── 6. using enum ────────────────────────────────────────────────────────

using enum ElfError::Code;

// ── 4. <bit> — compile-time endianness gate ──────────────────────────────

static_assert(std::endian::native == std::endian::little,
              "rv64 ELF reader assumes a little-endian host");

// ── elf_error_str ────────────────────────────────────────────────────────

std::string_view elf_error_str(ElfError::Code e) {
    switch (e) {
    case Truncated:      return "file is truncated";
    case BadMagic:       return "not an ELF (bad magic)";
    case Not64Bit:       return "not a 64-bit ELF";
    case NotLittleEndian:return "not little-endian";
    case NotRiscV:       return "not RISC-V (e_machine != 0xF3)";
    case BadPhdrs:       return "program header table out of bounds";
    case BadShdrs:       return "section header table out of bounds";
    case BadShstrtab:    return "section string table out of bounds";
    }
    return "unknown error";
}

// ── 4. <bit> — little-endian reads via std::bit_cast ─────────────────────

std::uint16_t Elf::rd16(std::span<const std::uint8_t> d, std::size_t off) {
    return std::bit_cast<std::uint16_t>(
        std::array<std::uint8_t, 2>{d[off], d[off + 1]});
}

std::uint32_t Elf::rd32(std::span<const std::uint8_t> d, std::size_t off) {
    return std::bit_cast<std::uint32_t>(
        std::array<std::uint8_t, 4>{d[off], d[off + 1], d[off + 2], d[off + 3]});
}

std::uint64_t Elf::rd64(std::span<const std::uint8_t> d, std::size_t off) {
    return std::bit_cast<std::uint64_t>(std::array<std::uint8_t, 8>{
        d[off], d[off + 1], d[off + 2], d[off + 3],
        d[off + 4], d[off + 5], d[off + 6], d[off + 7]});
}

// ── internal helpers ─────────────────────────────────────────────────────

namespace {

bool read_table(std::span<const std::uint8_t> data, std::uint64_t off,
                std::size_t ent_size, std::size_t count,
                std::span<const std::uint8_t> &out) {
    if (count == 0) {
        out = {};
        return true;
    }
    if (ent_size == 0 || off > data.size()) {
        return false;
    }
    const std::uint64_t need = static_cast<std::uint64_t>(count) * ent_size;
    if (need > data.size() - off) {
        return false;
    }
    out = data.subspan(static_cast<std::size_t>(off),
                       static_cast<std::size_t>(need));
    return true;
}

}  // namespace

// ── 1. Result factory ────────────────────────────────────────────────────

Result<Elf, ElfError> Elf::open(std::span<const std::uint8_t> data) {
    Elf elf(data);
    // Inline the parse logic so errors carry source_location from the call site
    if (data.size() < 64) {
        return ElfError{Truncated};
    }
    if (data[0] != 0x7F || data[1] != 'E' || data[2] != 'L' || data[3] != 'F') {
        return ElfError{BadMagic};
    }
    if (data[4] != 2) {
        return ElfError{Not64Bit};
    }
    if (data[5] != 1) {
        return ElfError{NotLittleEndian};
    }

    elf.ehdr_.e_type      = rd16(data, 16);
    elf.ehdr_.e_machine   = rd16(data, 18);
    elf.ehdr_.e_version   = rd32(data, 20);
    elf.ehdr_.e_entry     = rd64(data, 24);
    elf.ehdr_.e_phoff     = rd64(data, 32);
    elf.ehdr_.e_shoff     = rd64(data, 40);
    elf.ehdr_.e_flags     = rd32(data, 48);
    elf.ehdr_.e_ehsize    = rd16(data, 52);
    elf.ehdr_.e_phentsize = rd16(data, 54);
    elf.ehdr_.e_phnum     = rd16(data, 56);
    elf.ehdr_.e_shentsize = rd16(data, 58);
    elf.ehdr_.e_shnum     = rd16(data, 60);
    elf.ehdr_.e_shstrndx  = rd16(data, 62);

    if (elf.ehdr_.e_machine != EM_RISCV) {
        return ElfError{NotRiscV};
    }

    std::span<const std::uint8_t> phdr_bytes;
    if (!read_table(data, elf.ehdr_.e_phoff, elf.ehdr_.e_phentsize,
                    elf.ehdr_.e_phnum, phdr_bytes)) {
        return ElfError{BadPhdrs};
    }
    elf.phdrs_.reserve(elf.ehdr_.e_phnum);
    for (std::size_t i = 0; i < elf.ehdr_.e_phnum; ++i) {
        const std::size_t o = i * elf.ehdr_.e_phentsize;
        if (elf.ehdr_.e_phentsize < 56) {
            return ElfError{BadPhdrs};
        }
        elf.phdrs_.push_back(Phdr{
            .p_type   = rd32(phdr_bytes, o),
            .p_flags  = rd32(phdr_bytes, o + 4),
            .p_offset = rd64(phdr_bytes, o + 8),
            .p_vaddr  = rd64(phdr_bytes, o + 16),
            .p_paddr  = rd64(phdr_bytes, o + 24),
            .p_filesz = rd64(phdr_bytes, o + 32),
            .p_memsz  = rd64(phdr_bytes, o + 40),
            .p_align  = rd64(phdr_bytes, o + 48),
        });
    }

    std::span<const std::uint8_t> shdr_bytes;
    if (!read_table(data, elf.ehdr_.e_shoff, elf.ehdr_.e_shentsize,
                    elf.ehdr_.e_shnum, shdr_bytes)) {
        return ElfError{BadShdrs};
    }
    elf.shdrs_.reserve(elf.ehdr_.e_shnum);
    for (std::size_t i = 0; i < elf.ehdr_.e_shnum; ++i) {
        const std::size_t o = i * elf.ehdr_.e_shentsize;
        if (elf.ehdr_.e_shentsize < 64) {
            return ElfError{BadShdrs};
        }
        elf.shdrs_.push_back(Shdr{
            .sh_name      = rd32(shdr_bytes, o),
            .sh_type      = rd32(shdr_bytes, o + 4),
            .sh_flags     = rd64(shdr_bytes, o + 8),
            .sh_addr      = rd64(shdr_bytes, o + 16),
            .sh_offset    = rd64(shdr_bytes, o + 24),
            .sh_size      = rd64(shdr_bytes, o + 32),
            .sh_link      = rd32(shdr_bytes, o + 40),
            .sh_info      = rd32(shdr_bytes, o + 44),
            .sh_addralign = rd64(shdr_bytes, o + 48),
            .sh_entsize   = rd64(shdr_bytes, o + 56),
        });
    }

    elf.shstrtab_ = {};
    if (elf.ehdr_.e_shstrndx != SHN_UNDEF &&
        elf.ehdr_.e_shstrndx < elf.ehdr_.e_shnum) {
        const Shdr &strtab = elf.shdrs_[elf.ehdr_.e_shstrndx];
        std::span<const std::uint8_t> bytes;
        if (!read_table(data, strtab.sh_offset, 1, strtab.sh_size, bytes)) {
            return ElfError{BadShstrtab};
        }
        elf.shstrtab_ = bytes;
    }

    return elf;
}

// ── section_name ─────────────────────────────────────────────────────────

std::string_view Elf::section_name(const Shdr &sh) const {
    if (shstrtab_.empty() || sh.sh_name >= shstrtab_.size()) {
        return {};
    }
    const std::size_t end = shstrtab_.size();
    std::size_t i = sh.sh_name;
    while (i < end && shstrtab_[i] != 0) {
        ++i;
    }
    return std::string_view(
        reinterpret_cast<const char *>(shstrtab_.data()) + sh.sh_name,
        i - sh.sh_name);
}

// ── loadable_in ──────────────────────────────────────────────────────────

bool Elf::loadable_in(std::uint64_t base, std::uint64_t end) const {
    for (const Phdr &p : phdrs_) {
        if (p.p_type != PT_LOAD) {
            continue;
        }
        if (p.p_vaddr < base || p.p_vaddr + p.p_memsz > end) {
            return false;
        }
    }
    return true;
}

}  // namespace rv64
