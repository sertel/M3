/*
 * Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * This file is part of M3 (Microkernel for Minimalist Manycores).
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

#include <base/Common.h>

namespace m3 {

/* the stack frame for the interrupt-handler */
struct ExceptionState {
    word_t regs[31];
    word_t cause;
    word_t sepc;
    word_t sstatus;
} PACKED;

class ISRBase {
    ISRBase() = delete;

public:
    static const size_t ISR_COUNT       = 32;

    static const size_t DTU_ISR         = 16 + 9;
};

}