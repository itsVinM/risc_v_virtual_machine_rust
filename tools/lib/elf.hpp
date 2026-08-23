#pragma once

// Minimal little-endian ELF64 reader for RISC-V (rv64vm host tooling).
// C++20; no external dependencies.

#include <array>
#include <bit>
#include <compare>
#include <concepts>
#include <cstddef>
#include <cstdint>
#include <ranges>
#include <source_location>
#include <span>
#include <string_view>
#include <utility>
#include <variant>
#include <vector>

namespace rv64 {

// ── 9. std::source_location ──────────────────────────────────────────────

struct ElfError {
    enum class Code {
        Truncated,
        BadMagic,
        Not64Bit,
        NotLittleEndian,
        NotRiscV,
        BadPhdrs,
        BadShdrs,
        BadShstrtab,
    };

    Code code;
    std::source_location loc = std::source_location::current();

    friend bool operator==(const ElfError &a, const ElfError &b) {
        return a.code == b.code;
    }
};

std::string_view elf_error_str(ElfError::Code e);

// ── 1. Result<T,E> wrapper ───────────────────────────────────────────────

template <typename T, typename E>
class Result {
public:
    Result(T v) : val_(std::move(v)) {}   // NOLINT implicit
    Result(E e) : val_(std::move(e)) {}   // NOLINT implicit

    explicit operator bool() const { return std::holds_alternative<T>(val_); }
    const T &operator*() const & { return std::get<T>(val_); }
    T &&operator*() && { return std::get<T>(std::move(val_)); }
    const E &error() const & { return std::get<E>(val_); }

private:
    std::variant<T, E> val_;
};

// ── Constants ────────────────────────────────────────────────────────────

constexpr std::uint16_t EM_RISCV    = 0xF3;
constexpr std::uint32_t PT_LOAD     = 1;
constexpr std::uint32_t PF_X        = 1;
constexpr std::uint32_t PF_W        = 2;
constexpr std::uint32_t PF_R        = 4;
constexpr std::uint16_t SHN_UNDEF   = 0;

// ── 5. Three-way comparison on ELF structs ───────────────────────────────

struct Ehdr {
    std::uint16_t e_type;
    std::uint16_t e_machine;
    std::uint32_t e_version;
    std::uint64_t e_entry;
    std::uint64_t e_phoff;
    std::uint64_t e_shoff;
    std::uint32_t e_flags;
    std::uint16_t e_ehsize;
    std::uint16_t e_phentsize;
    std::uint16_t e_phnum;
    std::uint16_t e_shentsize;
    std::uint16_t e_shnum;
    std::uint16_t e_shstrndx;

    auto operator<=>(const Ehdr &) const = default;
};

struct Phdr {
    std::uint32_t p_type;
    std::uint32_t p_flags;
    std::uint64_t p_offset;
    std::uint64_t p_vaddr;
    std::uint64_t p_paddr;
    std::uint64_t p_filesz;
    std::uint64_t p_memsz;
    std::uint64_t p_align;

    auto operator<=>(const Phdr &) const = default;
};

struct Shdr {
    std::uint32_t sh_name;
    std::uint32_t sh_type;
    std::uint64_t sh_flags;
    std::uint64_t sh_addr;
    std::uint64_t sh_offset;
    std::uint64_t sh_size;
    std::uint32_t sh_link;
    std::uint32_t sh_info;
    std::uint64_t sh_addralign;
    std::uint64_t sh_entsize;

    auto operator<=>(const Shdr &) const = default;
};

// ── 7. Concepts ──────────────────────────────────────────────────────────

template <typename T>
concept ByteSpan = requires(T t) {
    { t.data() } -> std::convertible_to<const std::uint8_t *>;
    { t.size() } -> std::convertible_to<std::size_t>;
};

// ── 8. consteval ELF magic check ─────────────────────────────────────────

consteval bool is_elf_magic(std::array<std::uint8_t, 4> m) {
    return m[0] == 0x7F && m[1] == 'E' && m[2] == 'L' && m[3] == 'F';
}

// ── Elf class ────────────────────────────────────────────────────────────

class Elf {
public:
    explicit Elf(std::span<const std::uint8_t> data) : data_(data) {}

    // Static factory returning Result (1. Result wrapper)
    static Result<Elf, ElfError> open(std::span<const std::uint8_t> data);

    // ── 3. <ranges> helpers (defined inline — auto return needs visibility)
    auto load_segments() const {
        return phdrs_ | std::views::filter(
            [](const Phdr &p) { return p.p_type == PT_LOAD; });
    }

    auto named_sections() const {
        return shdrs_ | std::views::filter([this](const Shdr &s) {
            return !section_name(s).empty();
        });
    }

    const Ehdr &header() const { return ehdr_; }
    std::span<const Phdr> phdrs() const { return phdrs_; }
    std::span<const Shdr> shdrs() const { return shdrs_; }

    std::string_view section_name(const Shdr &sh) const;
    bool loadable_in(std::uint64_t base, std::uint64_t end) const;

private:
    // ── 4. <bit> for little-endian reads ─────────────────────────────────
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
