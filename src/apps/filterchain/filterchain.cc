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

#include <base/util/Random.h>
#include <base/stream/IStringStream.h>

#include <m3/com/MemGate.h>
#include <m3/com/SendGate.h>
#include <m3/com/RecvGate.h>
#include <m3/com/GateStream.h>
#include <m3/stream/Standard.h>
#include <m3/pes/VPE.h>

using namespace m3;

static const size_t BUF_SIZE    = 4096;

int main(int argc, char **argv) {
    size_t memSize = 8 * 1024 * 1024;
    if(argc > 1)
        memSize = Math::round_up(IStringStream::read_from<size_t>(argv[1]), BUF_SIZE);

    Random::init(0x1234);

    MemGate mem = MemGate::create_global(memSize, MemGate::RW);

    cout << "Initializing memory...\n";

    // init memory with random numbers
    uint *buffer = new uint[BUF_SIZE / sizeof(uint)];
    size_t rem = memSize;
    size_t offset = 0;
    while(rem > 0) {
        for(size_t i = 0; i < BUF_SIZE / sizeof(uint); ++i)
            buffer[i] = static_cast<uint>(Random::get());
        mem.write(buffer, BUF_SIZE, offset);
        offset += BUF_SIZE;
        rem -= BUF_SIZE;
    }
    mem.deactivate();

    cout << "Starting filter chain...\n";

    // create receiver
    auto pe2 = PE::alloc("receiver");
    VPE t2(pe2, "receiver");

    // create a gate the sender can send to (at the receiver)
    RecvGate rgate = RecvGate::create(nextlog2<512>::val, nextlog2<64>::val);
    SendGate sgate = SendGate::create(&rgate, SendGateArgs().credits(1));
    MemGate resmem = MemGate::create_global(BUF_SIZE, MemGate::RW);

    t2.fds(VPE::self().fds());
    t2.obtain_fds();
    t2.delegate_obj(rgate.sel());
    t2.run([&rgate] {
        size_t count, total = 0;
        int finished = 0;
        while(!finished) {
            GateIStream is = receive_vmsg(rgate, count, finished);

            cout << "Got " << count << " data items\n";

            reply_vmsg(is, 0);
            total += count;
        }
        cout << "Got " << total << " items in total\n";
        return 0;
    });

    auto pe1 = PE::alloc("sender");
    VPE t1(pe1, "sender");
    t1.fds(VPE::self().fds());
    t1.obtain_fds();
    t1.delegate_obj(mem.sel());
    t1.delegate_obj(resmem.sel());
    t1.delegate_obj(sgate.sel());
    t1.run([buffer, &mem, &sgate, &resmem, memSize] {
        uint *result = new uint[BUF_SIZE / sizeof(uint)];
        size_t c = 0;

        size_t rem = memSize;
        size_t offset = 0;
        while(rem > 0) {
            mem.read(buffer, BUF_SIZE, offset);
            for(size_t i = 0; i < BUF_SIZE / sizeof(uint); ++i) {
                // condition that selects the data item
                if(buffer[i] % 10 == 0) {
                    result[c++] = buffer[i];
                    // if the result buffer is full, send it over to the receiver and notify him
                    if(c == BUF_SIZE / sizeof(uint)) {
                        resmem.write(result, c * sizeof(uint), 0);
                        send_receive_vmsg(sgate, c, 0);
                        c = 0;
                    }
                }
            }

            offset += BUF_SIZE;
            rem -= BUF_SIZE;
        }

        // any data left to send?
        if(c > 0) {
            resmem.write(result, c * sizeof(uint), 0);
            send_receive_vmsg(sgate, c, 1);
        }
        return 0;
    });

    t1.wait();
    t2.wait();

    cout << "Done.\n";
    return 0;
}
