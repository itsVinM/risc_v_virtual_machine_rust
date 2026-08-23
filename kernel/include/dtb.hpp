#pragma once

#include <cstdint>
#include <optional>

namespace kernel::devicetree{
    enum class PhysicalAddress : std::uint64_t {};

    struct Dtb {
        // Static utility namespace/clas
        Dtb() = delete;
        // Return UART addr else std::nullopt if missing
        [[nodiscard]] static std::optional<PhysicalAddress> find_uart(PhysicalAddress dtb_pa) noexcept;
    };
} // namespace kernel::devicetree



