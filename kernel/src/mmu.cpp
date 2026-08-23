#include "mmu.hpp"

#include <array>
#include <cstddef>
#include <cstdint>

#include "csr.hpp"
#include "panic.hpp"
#include "types.hpp"

extern "C" unsigned char _end[];

namespace kernel::rv64vm {

    enum class PhysAddr : std::uint64_t {};
    enum class VirtAddr : std::uint64_t {};

    enum class PageFlags : std::uint64_t {
        Valid    = 1ULL << 0,
        Read     = 1ULL << 1,
        Write    = 1ULL << 2,
        Execute  = 1ULL << 3,
        User     = 1ULL << 4,
        Global   = 1ULL << 5,
        Accessed = 1ULL << 6,
        Dirty    = 1ULL << 7,
    };

    [[nodiscard]] constexpr PageFlags operator|(PageFlags lhs, PageFlags rhs) noexcept {
        return static_cast<PageFlags>(static_cast<std::uint64_t>(lhs) | static_cast<std::uint64_t>(rhs));
    }

    [[nodiscard]] constexpr PageFlags operator&(PageFlags lhs, PageFlags rhs) noexcept {
        return static_cast<PageFlags>(static_cast<std::uint64_t>(lhs) & static_cast<std::uint64_t>(rhs));
    }

    // Sv39 hardware constants
    constexpr std::size_t PageSize = 4096;
    constexpr std::uint64_t SatpModeSv39 = 8ULL << 60;

    // Sv39 page table entry:
    //   bits [9:0]   flags
    //   bits [63:10] PPN = PA >> 12
    struct PageTableEntry {
        std::uint64_t value{0};

        [[nodiscard]] constexpr bool is_valid() const noexcept {
            return (value & static_cast<std::uint64_t>(PageFlags::Valid)) != 0;
        }

        // R/W/X all zero => pointer to next table level, otherwise leaf.
        [[nodiscard]] constexpr bool is_leaf() const noexcept {
            constexpr auto rwx = static_cast<std::uint64_t>(
                PageFlags::Read | PageFlags::Write | PageFlags::Execute);
            return (value & rwx) != 0;
        }

        constexpr void set(PhysAddr pa, PageFlags flags) noexcept {
            const auto raw_pa = static_cast<std::uint64_t>(pa);
            const auto raw_flags = static_cast<std::uint64_t>(flags);
            value = ((raw_pa >> 12) << 10) | raw_flags | static_cast<std::uint64_t>(PageFlags::Valid);
        }

        [[nodiscard]] PhysAddr address() const noexcept {
            return static_cast<PhysAddr>((value >> 10) << 12);
        }

        [[nodiscard]] PageTable* child_table() const noexcept;
    };

    static_assert(sizeof(PageTableEntry) == 8);

    // 512 entries per Sv39 page table; the table itself must be page aligned.
    struct alignas(PageSize) PageTable : std::array<PageTableEntry, 512> {};

    static_assert(sizeof(PageTable) == PageSize);

    inline PageTable* PageTableEntry::child_table() const noexcept {
        return reinterpret_cast<PageTable*>((value >> 10) << 12);
    }

    // Virtual address parser helper (Sv39: VPN[2]=[38:30] VPN[1]=[29:21] VPN[0]=[20:12])
    class VirtAddrIndices {
    public:
        explicit constexpr VirtAddrIndices(VirtAddr va) noexcept : m_va(static_cast<std::uint64_t>(va)) {}

        [[nodiscard]] constexpr std::size_t vpn2() const noexcept { return (m_va >> 30) & 0x1FF; }
        [[nodiscard]] constexpr std::size_t vpn1() const noexcept { return (m_va >> 21) & 0x1FF; }
        [[nodiscard]] constexpr std::size_t vpn0() const noexcept { return (m_va >> 12) & 0x1FF; }

    private:
        std::uint64_t m_va;
    };

    namespace {

        PageTable* s_root_table{nullptr};

        // Bump allocator for page-table frames above the kernel image (_end).
        class FrameAllocator {
        public:
            FrameAllocator() = delete;

            [[nodiscard]] static PageTable* allocate_page_table() noexcept {
                const auto base =
                    (reinterpret_cast<std::uintptr_t>(_end) + PageSize - 1) & ~(PageSize - 1);

                if (s_bytes_used + PageSize > ArenaBytes)
                    kernel::panic::panic("mmu", __LINE__, "frame arena exhausted");

                auto* ptr = reinterpret_cast<PageTable*>(base + s_bytes_used);
                s_bytes_used += PageSize;

                ptr->fill(PageTableEntry{});
                return ptr;
            }

            [[nodiscard]] static std::size_t bytes_used() noexcept { return s_bytes_used; }

        private:
            static constexpr std::size_t ArenaBytes = 1024 * 1024;
            static inline std::size_t s_bytes_used{0};
        };

        // Walks down a level, allocating the intermediate table if needed.
        [[nodiscard]] PageTable* resolve_next_level(PageTableEntry& entry) noexcept {
            if (!entry.is_valid()) {
                PageTable* new_table = FrameAllocator::allocate_page_table();
                entry.set(static_cast<PhysAddr>(reinterpret_cast<std::uintptr_t>(new_table)),
                          PageFlags::Valid);
                return new_table;
            }
            return entry.child_table();
        }

        void map_page(PageTable& root, VirtAddr va, PhysAddr pa, PageFlags flags) noexcept {
            const VirtAddrIndices idx(va);

            PageTable* l1 = resolve_next_level(root[idx.vpn2()]);
            PageTable* l0 = resolve_next_level((*l1)[idx.vpn1()]);

            (*l0)[idx.vpn0()].set(pa, flags | PageFlags::Accessed | PageFlags::Dirty);
        }

        [[maybe_unused]] void map_range(PageTable& root, PhysAddr base, std::size_t len, PageFlags flags) noexcept {
            for (std::size_t offset = 0; offset < len; offset += PageSize) {
                const auto phys = static_cast<PhysAddr>(static_cast<std::uint64_t>(base) + offset);
                const auto virt = static_cast<VirtAddr>(static_cast<std::uint64_t>(base) + offset);
                map_page(root, virt, phys, flags);
            }
        }

    } // namespace

    /*
     * Enable Sv39 with a flat identity map built from two leaf gigapages in
     * the root table:
     *   vpn2 = 0 -> [0x0000000000000000 .. 0x000000003FFFFFFF] UART/CLINT/PLIC MMIO
     *   vpn2 = 2 -> [0x0000000080000000 .. 0x00000000BFFFFFFF] DRAM (kernel, heap, DTB)
     */
    void Mmu::init() noexcept {
        s_root_table = FrameAllocator::allocate_page_table();

        const auto map_flags = PageFlags::Read | PageFlags::Write | PageFlags::Execute |
                               PageFlags::Global | PageFlags::Accessed | PageFlags::Dirty;

        (*s_root_table)[0].set(static_cast<PhysAddr>(0x00000000ULL), map_flags);
        (*s_root_table)[2].set(static_cast<PhysAddr>(0x80000000ULL), map_flags);

        const auto satp = SatpModeSv39 | (reinterpret_cast<std::uint64_t>(s_root_table) >> 12);

        arch::rv64vm::csr_write<arch::rv64vm::Csr::Satp>(satp);
        asm volatile("sfence.vma zero, zero" ::: "memory");
    }

    PageTable* Mmu::root() noexcept {
        return s_root_table;
    }

} // namespace kernel::rv64vm
