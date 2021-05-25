/*
 * Copyright (C) 2015-2016, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#include <m3/com/RecvGate.h>
#include <m3/com/GateStream.h>
#include <m3/stream/Standard.h>

using namespace m3;

int main() {
    RecvGate rgate = RecvGate::create_named("chan");
    rgate.activate();

    uint64_t val;
    while(1) {
        auto is = receive_msg(rgate);
        is >> val;
        cout << "Got " << fmt(val, "x") << " from " << is.label<int>() << "\n";
        reply_vmsg(is, 0);
    }
    return 0;
}