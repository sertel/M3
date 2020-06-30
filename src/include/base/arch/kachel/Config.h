/*
 * Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * This file is part of M3 (Microkernel-based SysteM for Heterogeneous Manycores).
 *
 * M3 is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 *
 * M3 is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License version 2 for more details.
 */

#pragma once

#define FS_IMG_OFFSET       0x0

#define PAGE_BITS           12
#define PAGE_SIZE           (static_cast<size_t>(1) << PAGE_BITS)
#define PAGE_MASK           (PAGE_SIZE - 1)

#define LPAGE_BITS          21
#define LPAGE_SIZE          (static_cast<size_t>(1) << LPAGE_BITS)
#define LPAGE_MASK          (LPAGE_SIZE - 1)

#define FIXED_KMEM          (2 * 1024 * 1024)

#define APP_HEAP_SIZE       (64 * 1024 * 1024)
#define ROOT_HEAP_SIZE      (2 * 1024 * 1024)
#define EPMEM_SIZE          0

#define EP_COUNT            192

#if defined(__riscv)
#   define MEM_OFFSET       0x10000000
#else
#   define MEM_OFFSET       0
#endif

// hw layout:
// +----------------------------+ 0x0
// |         devices etc.       |
// +----------------------------+ 0x10000000
// |           app code         |
// +----------------------------+ 0x100F0000
// |          PEMux code        |
// +----------------------------+ 0x10100000
// |       env + PEMux data     |
// +----------------------------+ 0x10101000
// |          app data          |
// +----------------------------+ 0x101E0000
// |          app stack         |
// +----------------------------+ 0x101F0000
// |         serial buf         |
// +----------------------------+ 0x101F1000
// |      app recv buffers      |
// +----------------------------+ 0x101FF000
// |     PEMux recv buffers     |
// +----------------------------+ 0x10200000
// |            ...             |
// +----------------------------+ 0xF0000000
// |          TCU MMIO          |
// +----------------------------+ 0xF0002000

// gem5 layout:
// +----------------------------+ 0x0
// |            ...             |
// +----------------------------+ 0x10100000
// |            env             |
// +----------------------------+ 0x10101000
// |            ...             |
// +----------------------------+ 0x10200000
// |      PEMux code+data       |
// +----------------------------+ 0x102FF000
// |     PEMux recv buffers     |
// +----------------------------+ 0x10300000
// |          app stack         |
// +----------------------------+ 0x10310000
// |       app code+data        |
// |            ...             |
// +----------------------------+ 0xD0000000
// |      std recv buffers      |
// +----------------------------+ 0xD0001000
// |        recv buffers        |
// |            ...             |
// +----------------------------+ 0xE0000000
// |      PE's own phys mem     |
// +----------------------------+ 0xF0000000
// |          TCU MMIO          |
// +----------------------------+ 0xF0002000

#define ENV_START           (MEM_OFFSET + 0x100000)
#define ENV_SIZE            PAGE_SIZE
#define ENV_END             (ENV_START + ENV_SIZE)

#define STACK_SIZE          0x10000
#define STACK_TOP           (STACK_BOTTOM + STACK_SIZE)

#define PEMUX_RBUF_PHYS     0x2000
#define PEMUX_RBUF_SIZE     0x1000

#define RBUF_STD_ADDR       0xD0000000
#define RBUF_STD_SIZE       PAGE_SIZE
#define RBUF_ADDR           (RBUF_STD_ADDR + RBUF_STD_SIZE)
#define RBUF_SIZE           (0x10000000 - RBUF_STD_SIZE)
#define RBUF_SIZE_SPM       0xE000

#define SERIAL_SIGNAL       (MEM_OFFSET + 0x1F0000)
#define SERIAL_BUF          (MEM_OFFSET + 0x1F0008)
#define SERIAL_SIZE         0x1000

#define PE_MEM_BASE         0xE0000000

#if defined(__hw__)
#   define APP_CODE_START   MEM_OFFSET
#   define APP_CODE_SIZE    (PEMUX_CODE_START - APP_CODE_START)
#   define APP_DATA_START   (MEM_OFFSET + 0x101000)
#   define APP_DATA_SIZE    (STACK_BOTTOM - APP_DATA_START)

#   define PEMUX_CODE_START (MEM_OFFSET + 0xF0000)
#   define PEMUX_CODE_SIZE  (ENV_START - PEMUX_CODE_START)
#   define PEMUX_DATA_START (ENV_START + 0x800)
#   define PEMUX_DATA_SIZE  (APP_DATA_START - PEMUX_DATA_START)

#   define STACK_BOTTOM     (MEM_OFFSET + 0x1E0000)

#   define PEMUX_RBUF_SPACE (MEM_OFFSET + 0x1FF000)
#else
#   define STACK_BOTTOM     (MEM_OFFSET + 0x300000)

#   define PEMUX_CODE_START (MEM_OFFSET + 0x200000)

#   define PEMUX_RBUF_SPACE (MEM_OFFSET + 0x2FF000)
#endif

#define MAX_RB_SIZE         32

#define KPEX_RBUF_ORDER     6
#define KPEX_RBUF_SIZE      (1 << KPEX_RBUF_ORDER)

#define PEXUP_RBUF_ORDER    6
#define PEXUP_RBUF_SIZE     (1 << PEXUP_RBUF_ORDER)

#define SYSC_RBUF_ORDER     9
#define SYSC_RBUF_SIZE      (1 << SYSC_RBUF_ORDER)

#define UPCALL_RBUF_ORDER   6
#define UPCALL_RBUF_SIZE    (1 << UPCALL_RBUF_ORDER)

#define DEF_RBUF_ORDER      8
#define DEF_RBUF_SIZE       (1 << DEF_RBUF_ORDER)

#define VMA_RBUF_ORDER      6
#define VMA_RBUF_SIZE       (1 << VMA_RBUF_ORDER)

#define MEMCAP_END          RBUF_STD_ADDR