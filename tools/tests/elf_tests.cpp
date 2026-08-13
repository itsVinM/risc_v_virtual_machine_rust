// Unit tests for the C++20 ELF reader. Fixtures are assembled in memory so
// the suite is self-contained (no cross-compiler needed).

#include <cstdint>
#include <cstdio>
#include <span>
#include <string>
#include <vector>

#include "elf.hpp"

namespace {

int failures = 0;

#define CHECK(cond)                                                        \
    do {                                                                   \
        if (!(cond)) {                                                     \
            std::fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__,   \
                         #cond);                                           \
            ++failures;                                                    \
        }                                                                  \
    } while (0)

void put16(std::vector<std::uint8_t> &v, std::size_t off, std::uint16_t x) {
    v[off] = static_cast<std::uint8_t>(x);
    v[off + 1] = static_cast<std::uint8_t>(x >> 8);
}

void put32(std::vector<std::uint8_t> &v, std::size_t off, std::uint32_t x) {
    for (int i = 0; i < 4; ++i) {
        v[off + i] = static_cast<std::uint8_t>(x >> (8 * i));
    }
}

void put64(std::vector<std::uint8_t> &v, std::size_t off, std::uint64_t x) {
    for (int i = 0; i < 8; ++i) {
        v[off + i] = static_cast<std::uint8_t>(x >> (8 * i));
    }
}

struct Fixture {
    std::vector<std::uint8_t> bytes;
    // Program header fields (single PT_LOAD).
    std::uint64_t vaddr = 0x8020'0000ULL;
    std::uint64_t memsz = 0x1000;
    // Optional section header string table.
    bool with_shstrtab = false;
};

std::vector<std::uint8_t> build(const Fixture &f) {
    std::vector<std::uint8_t> v(64, 0);
    v[0] = 0x7F;
    v[1] = 'E';
    v[2] = 'L';
    v[3] = 'F';
    v[4] = 2;  // EI_CLASS: 64-bit
    v[5] = 1;  // EI_DATA: little-endian
    v[6] = 1;  // EI_VERSION
    put16(v, 16, 2);                    // e_type: ET_EXEC
    put16(v, 18, 0xF3);                 // e_machine: RISC-V
    put32(v, 20, 1);                    // e_version
    put64(v, 24, f.vaddr);              // e_entry
    put64(v, 32, 64);                   // e_phoff
    put64(v, 40, 0);                    // e_shoff (filled below if sections)
    put16(v, 52, 64);                   // e_ehsize
    put16(v, 54, 56);                   // e_phentsize
    put16(v, 56, 1);                    // e_phnum
    put16(v, 58, 64);                   // e_shentsize
    put16(v, 60, 0);                    // e_shnum
    put16(v, 62, 0);                    // e_shstrndx

    // One PT_LOAD, filesz == memsz, backed by 16 bytes of payload.
    v.resize(64 + 56 + 16);
    std::size_t p = 64;
    put32(v, p, 1);              // p_type: PT_LOAD
    put32(v, p + 4, 5);          // p_flags: R | X
    put64(v, p + 8, 64 + 56);    // p_offset
    put64(v, p + 16, f.vaddr);   // p_vaddr
    put64(v, p + 24, f.vaddr);   // p_paddr
    put64(v, p + 32, 16);        // p_filesz
    put64(v, p + 40, f.memsz);   // p_memsz
    put64(v, p + 48, 0x1000);    // p_align

    if (f.with_shstrtab) {
        const std::size_t strtab_off = v.size();      // names blob
        const std::string names = std::string("\0.text\0.bss\0", 13);
        v.insert(v.end(), names.begin(), names.end());
        const std::size_t shdr_off = v.size();        // section header table
        put64(v, 40, shdr_off);
        put16(v, 58, 64);
        put16(v, 60, 2);
        put16(v, 62, 1);
        for (int idx = 0; idx < 2; ++idx) {
            std::size_t sh = v.size();
            v.resize(sh + 64, 0);
            put32(v, sh, idx == 0 ? 1 : 7);       // sh_name offset in strtab
            put32(v, sh + 4, idx == 0 ? 1 : 8);   // sh_type: PROGBITS/NOBITS
            put64(v, sh + 8, idx == 0 ? 6 : 2);   // sh_flags: A + X / A + W
            put64(v, sh + 16, f.vaddr + idx * 0x800);
            put64(v, sh + 24, idx == 1 ? strtab_off : 0);  // shstrtab -> names
            put64(v, sh + 32, idx == 1 ? names.size() : 0x800);
        }
    }
    return v;
}

void test_parse_ok() {
    const Fixture f{};
    const auto bytes = build(f);
    rv64::Elf elf(bytes);
    CHECK(elf.parse() == rv64::ElfError::Ok);
    const rv64::Ehdr &h = elf.header();
    CHECK(h.e_machine == rv64::EM_RISCV);
    CHECK(h.e_entry == 0x8020'0000ULL);
    CHECK(h.e_phnum == 1);
    CHECK(elf.phdrs().size() == 1);
    CHECK(elf.phdrs()[0].p_type == rv64::PT_LOAD);
    CHECK(elf.phdrs()[0].p_memsz == 0x1000);
}

void test_loadable_window_ok() {
    const Fixture f{};
    const auto bytes = build(f);
    rv64::Elf elf(bytes);
    CHECK(elf.parse() == rv64::ElfError::Ok);
    CHECK(elf.loadable_in(0x8000'0000ULL, 0x8800'0000ULL));
}

void test_loadable_window_fail() {
    Fixture f{};
    f.memsz = 0x0801'0000ULL;  // overruns DRAM end
    const auto bytes = build(f);
    rv64::Elf elf(bytes);
    CHECK(elf.parse() == rv64::ElfError::Ok);
    CHECK(!elf.loadable_in(0x8000'0000ULL, 0x8800'0000ULL));
}

void test_bad_magic() {
    auto bytes = build(Fixture{});
    bytes[0] = 0x00;
    CHECK(rv64::Elf(bytes).parse() == rv64::ElfError::BadMagic);
}

void test_not_64bit() {
    auto bytes = build(Fixture{});
    bytes[4] = 1;
    CHECK(rv64::Elf(bytes).parse() == rv64::ElfError::Not64Bit);
}

void test_not_little_endian() {
    auto bytes = build(Fixture{});
    bytes[5] = 2;
    CHECK(rv64::Elf(bytes).parse() == rv64::ElfError::NotLittleEndian);
}

void test_not_riscv() {
    auto bytes = build(Fixture{});
    put16(bytes, 18, 0x3E);  // x86-64
    CHECK(rv64::Elf(bytes).parse() == rv64::ElfError::NotRiscV);
}

void test_truncated_short() {
    const std::vector<std::uint8_t> tiny{0x7F, 'E', 'L', 'F'};
    CHECK(rv64::Elf(tiny).parse() == rv64::ElfError::Truncated);
}

void test_truncated_phdrs() {
    auto bytes = build(Fixture{});
    bytes.resize(64 + 8);  // phdr table claims 56 bytes, only 8 remain
    CHECK(rv64::Elf(bytes).parse() == rv64::ElfError::BadPhdrs);
}

void test_section_names() {
    Fixture f{};
    f.with_shstrtab = true;
    const auto bytes = build(f);
    rv64::Elf elf(bytes);
    CHECK(elf.parse() == rv64::ElfError::Ok);
    CHECK(elf.shdrs().size() == 2);
    CHECK(elf.section_name(elf.shdrs()[0]) == ".text");
    CHECK(elf.section_name(elf.shdrs()[1]) == ".bss");
}

}  // namespace

int main() {
    test_parse_ok();
    test_loadable_window_ok();
    test_loadable_window_fail();
    test_bad_magic();
    test_not_64bit();
    test_not_little_endian();
    test_not_riscv();
    test_truncated_short();
    test_truncated_phdrs();
    test_section_names();

    if (failures == 0) {
        std::printf("elf_tests: all passed\n");
        return 0;
    }
    std::fprintf(stderr, "elf_tests: %d FAILED\n", failures);
    return 1;
}
