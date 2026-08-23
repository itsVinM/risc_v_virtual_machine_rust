#pragma once

#include "types.hpp"

namespace kernel::trap {
    struct TrapFrame {
        types::u64 gpr[31]; /* x1-x31 (x0 is hardwired zero) */
        types::u64 sstatus;
        types::u64 sepc;
    };

    [[nodiscard]] const char *exception_name(types::u64 code) noexcept;

    // Installed into mtvec at boot; attribute interrupt("machine") at def-site.
    void trap_handler() noexcept;
} // namespace kernel::trap
