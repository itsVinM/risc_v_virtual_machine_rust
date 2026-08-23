#pragma once

#include <cstddef>

// Global C-linkage versions: GCC may emit implicit calls to these even in
// freestanding mode (e.g. for block copies / zero-initialisation).
extern "C" {
void *memset(void *s, int c, std::size_t n);
void *memcpy(void *dst, const void *src, std::size_t n);
}

namespace kernel::mem {
using ::memset;
using ::memcpy;
} // namespace kernel::mem
