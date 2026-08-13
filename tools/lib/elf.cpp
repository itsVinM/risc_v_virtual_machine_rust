#include "elf.hpp"

namespace rv64 {

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

std::string_view elf_error_str(ElfError e) {
    switch (e) {
    case ElfError::Ok:
        return "ok";
    case ElfError::Truncated:
        return "file is truncated";
    case ElfError::BadMagic:
        return "not an ELF (bad magic)";
    case ElfError::Not64Bit:
        return "not a 64-bit ELF";
    case ElfError::NotLittleEndian:
        return "not little-endian";
    case ElfError::NotRiscV:
        return "not RISC-V (e_machine != 0xF3)";
    case ElfError::BadPhdrs:
        return "program header table out of bounds";
    case ElfError::BadShdrs:
        return "section header table out of bounds";
    case ElfError::BadShstrtab:
        return "section string table out of bounds";
    }
    return "unknown error";
}

ElfError Elf::parse() {
    if (data_.size() < 64) {
        return ElfError::Truncated;
    }
    if (data_[0] != 0x7F || data_[1] != 'E' || data_[2] != 'L' || data_[3] != 'F') {
        return ElfError::BadMagic;
    }
    if (data_[4] != 2) {
        return ElfError::Not64Bit;
    }
    if (data_[5] != 1) {
        return ElfError::NotLittleEndian;
    }

    ehdr_.e_type = rd16(data_, 16);
    ehdr_.e_machine = rd16(data_, 18);
    ehdr_.e_version = rd32(data_, 20);
    ehdr_.e_entry = rd64(data_, 24);
    ehdr_.e_phoff = rd64(data_, 32);
    ehdr_.e_shoff = rd64(data_, 40);
    ehdr_.e_flags = rd32(data_, 48);
    ehdr_.e_ehsize = rd16(data_, 52);
    ehdr_.e_phentsize = rd16(data_, 54);
    ehdr_.e_phnum = rd16(data_, 56);
    ehdr_.e_shentsize = rd16(data_, 58);
    ehdr_.e_shnum = rd16(data_, 60);
    ehdr_.e_shstrndx = rd16(data_, 62);

    if (ehdr_.e_machine != EM_RISCV) {
        return ElfError::NotRiscV;
    }

    std::span<const std::uint8_t> phdr_bytes;
    if (!read_table(data_, ehdr_.e_phoff, ehdr_.e_phentsize, ehdr_.e_phnum,
                    phdr_bytes)) {
        return ElfError::BadPhdrs;
    }
    phdrs_.reserve(ehdr_.e_phnum);
    for (std::size_t i = 0; i < ehdr_.e_phnum; ++i) {
        const std::size_t o = i * ehdr_.e_phentsize;
        if (ehdr_.e_phentsize < 56) {
            return ElfError::BadPhdrs;
        }
        Phdr p{};
        p.p_type = rd32(phdr_bytes, o);
        p.p_flags = rd32(phdr_bytes, o + 4);
        p.p_offset = rd64(phdr_bytes, o + 8);
        p.p_vaddr = rd64(phdr_bytes, o + 16);
        p.p_paddr = rd64(phdr_bytes, o + 24);
        p.p_filesz = rd64(phdr_bytes, o + 32);
        p.p_memsz = rd64(phdr_bytes, o + 40);
        p.p_align = rd64(phdr_bytes, o + 48);
        phdrs_.push_back(p);
    }

    std::span<const std::uint8_t> shdr_bytes;
    if (!read_table(data_, ehdr_.e_shoff, ehdr_.e_shentsize, ehdr_.e_shnum,
                    shdr_bytes)) {
        return ElfError::BadShdrs;
    }
    shdrs_.reserve(ehdr_.e_shnum);
    for (std::size_t i = 0; i < ehdr_.e_shnum; ++i) {
        const std::size_t o = i * ehdr_.e_shentsize;
        if (ehdr_.e_shentsize < 64) {
            return ElfError::BadShdrs;
        }
        Shdr s{};
        s.sh_name = rd32(shdr_bytes, o);
        s.sh_type = rd32(shdr_bytes, o + 4);
        s.sh_flags = rd64(shdr_bytes, o + 8);
        s.sh_addr = rd64(shdr_bytes, o + 16);
        s.sh_offset = rd64(shdr_bytes, o + 24);
        s.sh_size = rd64(shdr_bytes, o + 32);
        s.sh_link = rd32(shdr_bytes, o + 40);
        s.sh_info = rd32(shdr_bytes, o + 44);
        s.sh_addralign = rd64(shdr_bytes, o + 48);
        s.sh_entsize = rd64(shdr_bytes, o + 56);
        shdrs_.push_back(s);
    }

    shstrtab_ = {};
    if (ehdr_.e_shstrndx != SHN_UNDEF && ehdr_.e_shstrndx < ehdr_.e_shnum) {
        const Shdr &strtab = shdrs_[ehdr_.e_shstrndx];
        std::span<const std::uint8_t> bytes;
        if (!read_table(data_, strtab.sh_offset, 1, strtab.sh_size, bytes)) {
            return ElfError::BadShstrtab;
        }
        shstrtab_ = bytes;
    }

    return ElfError::Ok;
}

std::string_view Elf::section_name(const Shdr &sh) const {
    if (shstrtab_.empty() || sh.sh_name >= shstrtab_.size()) {
        return {};
    }
    const std::size_t end = shstrtab_.size();
    std::size_t i = sh.sh_name;
    while (i < end && shstrtab_[i] != 0) {
        ++i;
    }
    return std::string_view(reinterpret_cast<const char *>(shstrtab_.data()) + sh.sh_name,
                            i - sh.sh_name);
}

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

std::uint16_t Elf::rd16(std::span<const std::uint8_t> d, std::size_t off) {
    return static_cast<std::uint16_t>(d[off]) |
           static_cast<std::uint16_t>(d[off + 1]) << 8;
}

std::uint32_t Elf::rd32(std::span<const std::uint8_t> d, std::size_t off) {
    return static_cast<std::uint32_t>(d[off]) |
           static_cast<std::uint32_t>(d[off + 1]) << 8 |
           static_cast<std::uint32_t>(d[off + 2]) << 16 |
           static_cast<std::uint32_t>(d[off + 3]) << 24;
}

std::uint64_t Elf::rd64(std::span<const std::uint8_t> d, std::size_t off) {
    std::uint64_t v = 0;
    for (int i = 7; i >= 0; --i) {
        v = (v << 8) | d[off + i];
    }
    return v;
}

}  // namespace rv64
