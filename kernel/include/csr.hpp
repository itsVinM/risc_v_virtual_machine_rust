#pragma once

#include <concepts>
#include <cstdint>

namespace kernel::arch::rv64vm {

    // Raw register width
    using RegVal = std::uint64_t;

    // Register definitions
    enum class Csr : std::uint16_t {
        Sstatus = 0x100,
        Stvec   = 0x105,
        Sip     = 0x144,
        Satp    = 0x180,
        Mstatus = 0x300,
        Mtvec   = 0x305,
        Mscratch = 0x340,
        Mepc    = 0x341,
        Mcause  = 0x342,
        Mtval   = 0x343,
        Mhartid = 0xF14,
    };

    template <Csr Register>
    struct CsrRegister {
        static constexpr auto TargetCsr = static_cast<std::uint16_t>(Register);

        [[nodiscard]] static inline RegVal read() noexcept {
            RegVal val;
            asm volatile("csrr %0, %1" : "=r"(val) : "i"(TargetCsr));
            return val;
        }

        static inline void write(RegVal val) noexcept {
            asm volatile("csrw %1, %0" :: "r"(val), "i"(TargetCsr) : "memory");
        }

        static inline void set(RegVal bits) noexcept {
            asm volatile("csrs %1, %0" :: "r"(bits), "i"(TargetCsr) : "memory");
        }

        static inline void clear(RegVal bits) noexcept {
            asm volatile("csrc %1, %0" :: "r"(bits), "i"(TargetCsr) : "memory");
        }
    };

    template <Csr Register>
    [[nodiscard]] inline RegVal csr_read() noexcept {
        return CsrRegister<Register>::read();
    }

    template <Csr Register, std::unsigned_integral T>
    inline void csr_write(T val) noexcept {
        CsrRegister<Register>::write(static_cast<RegVal>(val));
    }

    template <Csr Register, std::unsigned_integral T>
    inline void csr_set(T bits) noexcept {
        CsrRegister<Register>::set(static_cast<RegVal>(bits));
    }

    template <Csr Register, std::unsigned_integral T>
    inline void csr_clear(T bits) noexcept {
        CsrRegister<Register>::clear(static_cast<RegVal>(bits));
    }

} // namespace kernel::arch::rv64vm
