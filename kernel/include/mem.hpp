#pragma once

#include "types.hpp"

namespace kernel::mem {
    void *kmalloc(types::usize n) noexcept;
    void *kzalloc(types::usize n) noexcept;
    void kfree(void *p) noexcept;

    [[nodiscard]] types::usize used() noexcept;
    [[nodiscard]] types::usize capacity() noexcept;
    [[nodiscard]] types::usize allocs() noexcept;
} // namespace kernel::mem
