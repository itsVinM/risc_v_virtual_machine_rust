#include "sbi.hpp"

#include "types.hpp"

namespace kernel::sbi {
namespace {

constexpr types::u64 SBI_EXT_TIME = 0x54494D45;
constexpr types::u64 SBI_EXT_IPI  = 0x00735049;
constexpr types::u64 SBI_EXT_HSM  = 0x0048534D;

constexpr types::u64 SBI_FID_SET_TIMER   = 0;
constexpr types::u64 SBI_FID_SEND_IPI    = 0;
constexpr types::u64 SBI_FID_HART_START  = 0;

} // namespace

SbiRet call(types::u64 ext, types::u64 fid,
            types::u64 a0, types::u64 a1, types::u64 a2) noexcept
{
    register types::u64 a0_ asm("a0") = a0;
    register types::u64 a1_ asm("a1") = a1;
    register types::u64 a2_ asm("a2") = a2;
    register types::u64 a6_ asm("a6") = fid;
    register types::u64 a7_ asm("a7") = ext;

    asm volatile("ecall"
                 : "+r"(a0_), "+r"(a1_)
                 : "r"(a2_), "r"(a6_), "r"(a7_)
                 : "memory");

    return SbiRet{a0_, a1_};
}

SbiRet set_timer(types::u64 stime) noexcept
{
    return call(SBI_EXT_TIME, SBI_FID_SET_TIMER, stime);
}

SbiRet send_ipi(types::u64 hmask) noexcept
{
    return call(SBI_EXT_IPI, SBI_FID_SEND_IPI, hmask);
}

SbiRet hart_start(types::u64 hartid, types::u64 entry, types::u64 priv) noexcept
{
    return call(SBI_EXT_HSM, SBI_FID_HART_START, hartid, entry, priv);
}

} // namespace kernel::sbi
