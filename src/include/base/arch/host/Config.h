/*
 * Copyright (C) 2015-2018 Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * Copyright (C) 2019-2022 Nils Asmussen, Barkhausen Institut
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

#define TILE_COUNT        18
#define MAX_ACTS          (TILE_COUNT - 1)

#define PAGE_BITS         12
#define PAGE_SIZE         (static_cast<size_t>(4096))
#define PAGE_MASK         (PAGE_SIZE - 1)

#define LOCAL_MEM_SIZE    (4 * 1024 * 1024)
#define EPMEM_SIZE        (1 * 1024 * 1024)

#define STACK_SIZE        0x1000

#define MEM_OFFSET        0

#define RBUF_STD_ADDR     0
#define RBUF_STD_SIZE     PAGE_SIZE
#define RBUF_ADDR         (RBUF_STD_ADDR + RBUF_STD_SIZE)
#define RBUF_SIZE         16384U
#define RBUF_SIZE_SPM     16384U

#define MAX_RB_SIZE       32

#define SYSC_RBUF_ORDER   9
#define SYSC_RBUF_SIZE    (1 << SYSC_RBUF_ORDER)

#define UPCALL_RBUF_ORDER 8
#define UPCALL_RBUF_SIZE  (1 << UPCALL_RBUF_ORDER)

#define DEF_RBUF_ORDER    8
#define DEF_RBUF_SIZE     (1 << DEF_RBUF_ORDER)
