// elfdump — inspect a little-endian RISC-V ELF64 and optionally validate it
// against the rv64vm DRAM window.
//
//   elfdump <file>              print header, program headers, sections
//   elfdump --check <file>      also verify loadability into DRAM (nonzero
//                               exit code on violation)

#include <algorithm>
#include <cstdio>
#include <cstring>
#include <format>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

#include "elf.hpp"

namespace {

constexpr std::uint64_t DRAM_BASE = 0x8000'0000ULL;
constexpr std::uint64_t DRAM_END  = 0x8800'0000ULL;

// ── 2. std::format helper (std::print is C++23) ──────────────────────────

template <typename... Args>
void println(std::format_string<Args...> fmt, Args &&...args) {
    std::cout << std::format(fmt, std::forward<Args>(args)...) << '\n';
}

// ── 6. using enum for phdr/shdr type names ──────────────────────────────

using enum rv64::ElfError::Code;

const char *phdr_type(std::uint32_t t) {
    switch (t) {
    case 0:  return "NULL";
    case 1:  return "LOAD";
    case 2:  return "DYNAMIC";
    case 3:  return "INTERP";
    case 4:  return "NOTE";
    case 5:  return "SHLIB";
    case 6:  return "PHDR";
    case 7:  return "TLS";
    default: return "OTHER";
    }
}

const char *shdr_type(std::uint32_t t) {
    switch (t) {
    case 0:          return "NULL";
    case 1:          return "PROGBITS";
    case 2:          return "SYMTAB";
    case 3:          return "STRTAB";
    case 4:          return "RELA";
    case 5:          return "HASH";
    case 6:          return "DYNAMIC";
    case 7:          return "NOTE";
    case 8:          return "NOBITS";
    case 9:          return "REL";
    case 0x7000'0003:return "RISCV_ATTRIBUTE";
    default:         return "OTHER";
    }
}

std::string flags_str(std::uint32_t f) {
    return std::format("{}{}{}",
                       f & rv64::PF_R ? 'R' : '-',
                       f & rv64::PF_W ? 'W' : '-',
                       f & rv64::PF_X ? 'X' : '-');
}

bool load_file(const std::string &path, std::vector<std::uint8_t> &out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        std::fprintf(stderr, "elfdump: cannot open '%s'\n", path.c_str());
        return false;
    }
    out.assign(std::istreambuf_iterator<char>(in),
               std::istreambuf_iterator<char>());
    return true;
}

int dump(const std::vector<std::uint8_t> &bytes, bool check) {
    // ── 1. Result wrapper: Elf::open returns Result<Elf, ElfError> ──────
    auto result = rv64::Elf::open(bytes);
    if (!result) {
        const auto &err = result.error();
        std::fprintf(stderr, "elfdump: %s [at %s:%d]\n",
                     std::string(rv64::elf_error_str(err.code)).c_str(),
                     err.loc.file_name(),
                     static_cast<int>(err.loc.line()));
        return 1;
    }

    const auto &elf = *result;
    const rv64::Ehdr &h = elf.header();

    // ── 2. std::format for all output ───────────────────────────────────
    println("ELF header");
    println("  type      {} (ET_EXEC)", h.e_type);
    println("  machine   {} (RISC-V)", h.e_machine);
    println("  entry     {:#018x}", h.e_entry);
    println("  phoff     {:#018x}  shoff {:#018x}", h.e_phoff, h.e_shoff);
    println("  phnum     {}  shnum {}  shstrndx {}", h.e_phnum, h.e_shnum,
            h.e_shstrndx);

    println("\nProgram headers");
    println("  {:<8} {:<8} {:<16} {:<16} {:<10} {:<10} {:<6} {}",
            "TYPE", "OFFSET", "VADDR", "PADDR", "FILESZ", "MEMSZ",
            "FLAGS", "ALIGN");

    // ── 3. <ranges> — use load_segments() ──────────────────────────────
    for (const rv64::Phdr &p : elf.load_segments()) {
        println("  {:<8} {:<8x} {:#016x} {:#016x} {:<10x} {:<10x} {} {:#x}",
                phdr_type(p.p_type), p.p_offset, p.p_vaddr, p.p_paddr,
                p.p_filesz, p.p_memsz, flags_str(p.p_flags), p.p_align);
    }
    // Also print non-LOAD segments
    for (const rv64::Phdr &p : elf.phdrs()) {
        if (p.p_type != rv64::PT_LOAD) {
            println("  {:<8} {:<8x} {:#016x} {:#016x} {:<10x} {:<10x} {} {:#x}",
                    phdr_type(p.p_type), p.p_offset, p.p_vaddr, p.p_paddr,
                    p.p_filesz, p.p_memsz, flags_str(p.p_flags), p.p_align);
        }
    }

    println("\nSections");
    println("  [{:<3}] {:<16} {:<14} {:<16} {:<10} {:<10} {}", "Nr", "NAME",
            "TYPE", "ADDR", "OFFSET", "SIZE", "FLAGS");

    // ── 3. <ranges> — use named_sections() ─────────────────────────────
    std::size_t i = 0;
    for (const rv64::Shdr &s : elf.shdrs()) {
        const auto name = elf.section_name(s);
        const auto fl = std::format("{}{}{}",
                                    s.sh_flags & 1 ? "X" : "-",
                                    s.sh_flags & 2 ? "W" : "-",
                                    s.sh_flags & 4 ? "A" : "-");
        println("  [{:>3}] {:<16} {:<14} {:#016x} {:#010x} {:#010x} {}",
                i++, name, shdr_type(s.sh_type), s.sh_addr, s.sh_offset,
                s.sh_size, fl);
    }

    if (check) {
        const bool ok = elf.loadable_in(DRAM_BASE, DRAM_END);
        println("\nVM check (DRAM {:#016x}..{:#016x}): {}",
                DRAM_BASE, DRAM_END, ok ? "OK" : "FAIL");
        return ok ? 0 : 1;
    }
    return 0;
}

}  // namespace

int main(int argc, char **argv) {
    bool check = false;
    const char *path = nullptr;
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--check") == 0) {
            check = true;
        } else if (path == nullptr) {
            path = argv[i];
        }
    }
    if (path == nullptr) {
        std::fprintf(stderr, "usage: elfdump [--check] <elf-file>\n");
        return 2;
    }
    std::vector<std::uint8_t> bytes;
    if (!load_file(path, bytes)) {
        return 2;
    }
    return dump(bytes, check);
}
