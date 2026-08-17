#ifndef DTB_H
#define DTB_H

#include "types.h"

/* Parse DTB, return UART base address or 0 if not found. */
u64 dtb_find_uart(u64 dtb_pa);

#endif
