// elfdump — inspect a little-endian RISC-V ELF64 and optionally validate it
// against the rv64vm DRAM window.
//
//   elfdump <file>              print header, program headers, sections
//   elfdump --check <file>      also verify loadability into DRAM (nonzero
//                               exit code on violation)

#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>
#include <vector>

#include "elf.hpp"

namespace {

constexpr std::uint64_t DRAM_BASE = 0x8000'0000ULL;
constexpr std::uint64_t DRAM_END = 0x8800'0000ULL;

const char *phdr_type(std::uint32_t t) {
    switch (t) {
    case 0:
        return "NULL";
    case 1:
        return "LOAD";
    case 2:
        return "DYNAMIC";
    case 3:
        return "INTERP";
    case 4:
        return "NOTE";
    case 5:
        return "SHLIB";
    case 6:
        return "PHDR";
    case 7:
        return "TLS";
    default:
        return "OTHER";
    }
}

const char *shdr_type(std::uint32_t t) {
    switch (t) {
    case 0:
        return "NULL";
    case 1:
        return "PROGBITS";
    case 2:
        return "SYMTAB";
    case 3:
        return "STRTAB";
    case 4:
        return "RELA";
    case 5:
        return "HASH";
    case 6:
        return "DYNAMIC";
    case 7:
        return "NOTE";
    case 8:
        return "NOBITS";
    case 9:
        return "REL";
    case 0x7000'0003:
        return "RISCV_ATTRIBUTE";
    default:
        return "OTHER";
    }
}

void print_flags(std::uint32_t f) {
    std::putchar(f & rv64::PF_R ? 'R' : '-');
    std::putchar(f & rv64::PF_W ? 'W' : '-');
    std::putchar(f & rv64::PF_X ? 'X' : '-');
}

bool load_file(const std::string &path, std::vector<std::uint8_t> &out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        std::fprintf(stderr, "elfdump: cannot open '%s'\n", path.c_str());
        return false;
    }
    out.assign(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
    return true;
}

int dump(const std::vector<std::uint8_t> &bytes, bool check) {
    rv64::Elf elf(bytes);
    const rv64::ElfError err = elf.parse();
    if (err != rv64::ElfError::Ok) {
        std::fprintf(stderr, "elfdump: %s\n",
                     std::string(rv64::elf_error_str(err)).c_str());
        return 1;
    }

    const rv64::Ehdr &h = elf.header();
    std::printf("ELF header\n");
    std::printf("  type      %u (ET_EXEC)\n", h.e_type);
    std::printf("  machine   %u (RISC-V)\n", h.e_machine);
    std::printf("  entry     0x%016llx\n",
                static_cast<unsigned long long>(h.e_entry));
    std::printf("  phoff     0x%016llx  shoff 0x%016llx\n",
                static_cast<unsigned long long>(h.e_phoff),
                static_cast<unsigned long long>(h.e_shoff));
    std::printf("  phnum     %u  shnum %u  shstrndx %u\n", h.e_phnum, h.e_shnum,
                h.e_shstrndx);

    std::printf("\nProgram headers\n");
    std::printf("  %-8s %-8s %-16s %-16s %-10s %-10s %-6s %s\n", "TYPE",
                "OFFSET", "VADDR", "PADDR", "FILESZ", "MEMSZ", "FLAGS",
                "ALIGN");
    for (const rv64::Phdr &p : elf.phdrs()) {
        std::printf("  %-8s %-8llx 0x%014llx 0x%014llx %-10llx %-10llx ", phdr_type(p.p_type),
                    static_cast<unsigned long long>(p.p_offset),
                    static_cast<unsigned long long>(p.p_vaddr),
                    static_cast<unsigned long long>(p.p_paddr),
                    static_cast<unsigned long long>(p.p_filesz),
                    static_cast<unsigned long long>(p.p_memsz));
        print_flags(p.p_flags);
        std::printf("  0x%llx\n", static_cast<unsigned long long>(p.p_align));
    }

    std::printf("\nSections\n");
    std::printf("  [%-3s] %-16s %-14s %-16s %-10s %-10s %s\n", "Nr", "NAME",
                "TYPE", "ADDR", "OFFSET", "SIZE", "FLAGS");
    std::size_t i = 0;
    for (const rv64::Shdr &s : elf.shdrs()) {
        std::printf("  [%3zu] %-16s %-14s 0x%014llx 0x%08llx 0x%08llx ",
                    i++, std::string(elf.section_name(s)).c_str(), shdr_type(s.sh_type),
                    static_cast<unsigned long long>(s.sh_addr),
                    static_cast<unsigned long long>(s.sh_offset),
                    static_cast<unsigned long long>(s.sh_size));
        std::printf("%s%s%s\n", s.sh_flags & 1 ? "X" : "-",
                    s.sh_flags & 2 ? "W" : "-", s.sh_flags & 4 ? "A" : "-");
    }

    if (check) {
        const bool ok = elf.loadable_in(DRAM_BASE, DRAM_END);
        std::printf("\nVM check (DRAM 0x%016llx..0x%016llx): %s\n",
                    static_cast<unsigned long long>(DRAM_BASE),
                    static_cast<unsigned long long>(DRAM_END),
                    ok ? "OK" : "FAIL");
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
