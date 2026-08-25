#include "trap.hpp"

#include "csr.hpp"
#include "panic.hpp"
#include "types.hpp"

namespace kernel::trap {

const char *exception_name(types::u64 code) noexcept
{
    switch (code) {
    case 0:  return "inst addr misaligned";
    case 1:  return "inst access fault";
    case 2:  return "illegal instruction";
    case 3:  return "breakpoint";
    case 4:  return "load addr misaligned";
    case 5:  return "load access fault";
    case 6:  return "store addr misaligned";
    case 7:  return "store access fault";
    case 8:  return "ecall from U-mode";
    case 9:  return "ecall from S-mode";
    case 11: return "ecall from M-mode";
    case 12: return "inst page fault";
    case 13: return "load page fault";
    case 15: return "store page fault";
    default: return "unknown";
    }
}

__attribute__((interrupt("supervisor")))
void trap_handler() noexcept
{
    namespace csr = arch::rv64vm;

    const auto cause = csr::csr_read<csr::Csr::Scause>();
    const auto epc   = csr::csr_read<csr::Csr::Sepc>();
    const auto tval  = csr::csr_read<csr::Csr::Stval>();

    constexpr types::u64 IntBit = types::u64{1} << 63;

    const char *name = (cause & IntBit) ? "interrupt" : exception_name(cause);

    kernel::panic::panic("trap", 0, "%s cause=%lu epc=%p tval=%p",
                         name, static_cast<unsigned long>(cause & ~IntBit),
                         reinterpret_cast<void *>(static_cast<types::uptr>(epc)),
                         reinterpret_cast<void *>(static_cast<types::uptr>(tval)));
}

} // namespace kernel::trap
