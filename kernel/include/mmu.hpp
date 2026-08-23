#pragma once

#include "types.hpp"

namespace kernel::rv64vm {
    struct PageTable;

    // Static driver class - no instantiation
    class Mmu {
    public:
        Mmu() = delete;
        // Build identity mapping and enable Sv39
        static void init() noexcept;
        // Pointer to active root page table
        [[nodiscard]] static PageTable* root() noexcept;
    };
} // namespace kernel::rv64vm
