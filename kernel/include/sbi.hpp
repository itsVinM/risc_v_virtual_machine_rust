#pragma once

#include "types.hpp"

namespace kernel::sbi {
    struct SbiRet {
        types::u64 error;
        types::u64 value;
    };

    [[nodiscard]] SbiRet call(types::u64 ext, types::u64 fid,
                              types::u64 a0 = 0, types::u64 a1 = 0, types::u64 a2 = 0) noexcept;

    [[nodiscard]] SbiRet set_timer(types::u64 stime) noexcept;
    [[nodiscard]] SbiRet send_ipi(types::u64 hmask) noexcept;
    [[nodiscard]] SbiRet hart_start(types::u64 hartid, types::u64 entry, types::u64 priv) noexcept;
} // namespace kernel::sbi
