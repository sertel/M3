/*
 * Copyright (C) 2015-2018 Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * Copyright (C) 2019 Nils Asmussen, Barkhausen Institut
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

#include <m3/stream/FStream.h>

#include <cstdlib>

namespace m3 {

static const fd_t STDIN_FD = 0;
static const fd_t STDOUT_FD = 1;
static const fd_t STDERR_FD = 2;

extern FStream cin;
extern FStream cout;
extern FStream cerr;

#define errmsg(expr)              \
    do {                          \
        m3::cerr << expr << "\n"; \
    }                             \
    while(0)

#define exitmsg(expr) \
    do {              \
        errmsg(expr); \
        ::exit(1);    \
    }                 \
    while(0)

}
