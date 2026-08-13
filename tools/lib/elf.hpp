#pragma once

// Minimal little-endian ELF64 reader for RISC-V (rv64vm host tooling).
// C++20; no external dependencies.

#include <cstddef>
#include <cstdint>
#include <span>
#include <string_view>
#include <vector>

namespace rv64 {

enum class ElfError {
    Ok,
    Truncated,     // file too short for the requested structure
    BadMagic,      // not \x7fELF
    Not64Bit,      // EI_CLASS != 2
    NotLittleEndian,  // EI_DATA != 1
    NotRiscV,      // e_machine != EM_RISCV
    BadPhdrs,      // program header table outside file
    BadShdrs,      // section header table outside file
    BadShstrtab,   // section string table missing / out of bounds
};

constexpr uint16_t EM_RISCV = 0xF3;
constexpr uint32_t PT_LOAD = 1;
constexpr uint32_t PF_X = 1;
constexpr uint32_t PF_W = 2;
constexpr uint32_t PF_R = 4;
constexpr uint16_t SHN_UNDEF = 0;

struct Ehdr {
    uint16_t e_type;
    uint16_t e_machine;
    uint32_t e_version;
    uint64_t e_entry;
    uint64_t e_phoff;
    uint64_t e_shoff;
    uint32_t e_flags;
    uint16_t e_ehsize;
    uint16_t e_phentsize;
    uint16_t e_phnum;
    uint16_t e_shentsize;
    uint16_t e_shnum;
    uint16_t e_shstrndx;
};

struct Phdr {
    uint32_t p_type;
    uint32_t p_flags;
    uint64_t p_offset;
    uint64_t p_vaddr;
    uint64_t p_paddr;
    uint64_t p_filesz;
    uint64_t p_memsz;
    uint64_t p_align;
};

struct Shdr {
    uint32_t sh_name;
    uint32_t sh_type;
    uint64_t sh_flags;
    uint64_t sh_addr;
    uint64_t sh_offset;
    uint64_t sh_size;
    uint32_t sh_link;
    uint32_t sh_info;
    uint64_t sh_addralign;
    uint64_t sh_entsize;
};

std::string_view elf_error_str(ElfError e);

class Elf {
  public:
    explicit Elf(std::span<const std::uint8_t> data) : data_(data) {}

    ElfError parse();

    const Ehdr &header() const { return ehdr_; }
    std::span<const Phdr> phdrs() const { return phdrs_; }
    std::span<const Shdr> shdrs() const { return shdrs_; }

    // Section name resolved via the section string table ("" if unavailable).
    std::string_view section_name(const Shdr &sh) const;

    // True iff every PT_LOAD segment lies entirely within [base, end).
    bool loadable_in(std::uint64_t base, std::uint64_t end) const;

  private:
    static std::uint16_t rd16(std::span<const std::uint8_t> d, std::size_t off);
    static std::uint32_t rd32(std::span<const std::uint8_t> d, std::size_t off);
    static std::uint64_t rd64(std::span<const std::uint8_t> d, std::size_t off);

    std::span<const std::uint8_t> data_;
    Ehdr ehdr_{};
    std::vector<Phdr> phdrs_;
    std::vector<Shdr> shdrs_;
    std::span<const std::uint8_t> shstrtab_;
};

}  // namespace rv64
