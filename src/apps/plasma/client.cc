/*
 * Copyright (C) 2015-2018 Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * Copyright (C) 2019-2020 Nils Asmussen, Barkhausen Institut
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

#include <base/Env.h>

#include <m3/com/GateStream.h>
#include <m3/session/ClientSession.h>
#include <m3/session/arch/host/Keyboard.h>
#include <m3/session/arch/host/Plasma.h>

using namespace m3;

static void kb_event(Plasma *plasma, GateIStream &is) {
    Keyboard::Event ev;
    is >> ev;
    if(ev.isbreak)
        return;
    switch(ev.keycode) {
        case Keyboard::VK_LEFT: plasma->left(); break;
        case Keyboard::VK_RIGHT: plasma->right(); break;
        case Keyboard::VK_UP: plasma->colup(); break;
        case Keyboard::VK_DOWN: plasma->coldown(); break;
    }
}

int main() {
    WorkLoop wl;

    // create event gate
    Keyboard kb("keyb");
    Plasma plasma("plasma");
    kb.rgate().start(&wl, std::bind(kb_event, &plasma, std::placeholders::_1));

    wl.run();
    return 0;
}
