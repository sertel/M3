/*
 * Copyright (C) 2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#include <base/stream/Serial.h>
#include <base/util/Time.h>

#include <m3/accel/StreamAccel.h>
#include <m3/stream/Standard.h>
#include <m3/pipe/IndirectPipe.h>
#include <m3/vfs/VFS.h>
#include <m3/Syscalls.h>

#include "imgproc.h"

using namespace m3;

static constexpr bool VERBOSE           = 1;
static constexpr size_t PIPE_SHM_SIZE   = 512 * 1024;

static const char *names[] = {
    "FFT",
    "MUL",
    "IFFT",
};

class DirectChain {
public:
    static const size_t ACCEL_COUNT     = 3;

    explicit DirectChain(Pipes &pipesrv, size_t id, Reference<File> in, Reference<File> out, Mode _mode)
        : mode(_mode),
          vpes(),
          accels(),
          pipes(),
          mems() {
        // create VPEs
        for(size_t i = 0; i < ACCEL_COUNT; ++i) {
            OStringStream name;
            name << names[i] << id;

            if(VERBOSE) Serial::get() << "Creating VPE " << name.str() << "\n";

            pes[i] = PE::alloc(PEDesc(PEType::COMP_IMEM, PEISA::ACCEL_COPY));
            vpes[i] = std::make_unique<VPE>(pes[i], name.str());

            accels[i] = std::make_unique<StreamAccel>(vpes[i], ACCEL_TIMES[i]);

            if(mode == Mode::DIR_SIMPLE && i + 1 < ACCEL_COUNT) {
                mems[i] = std::make_unique<MemGate>(
                    MemGate::create_global(PIPE_SHM_SIZE, MemGate::RW));
                pipes[i] = std::make_unique<IndirectPipe>(
                    pipesrv, *mems[i], PIPE_SHM_SIZE);
            }
        }

        if(VERBOSE) Serial::get() << "Connecting input and output...\n";

        // connect input/output
        accels[0]->connect_input(static_cast<GenericFile*>(in.get()));
        accels[ACCEL_COUNT - 1]->connect_output(static_cast<GenericFile*>(out.get()));
        for(size_t i = 0; i < ACCEL_COUNT; ++i) {
            if(i > 0) {
                if(mode == Mode::DIR_SIMPLE) {
                    auto rd = VPE::self().fds()->get(pipes[i - 1]->reader_fd());
                    accels[i]->connect_input(static_cast<GenericFile*>(rd.get()));
                }
                else
                    accels[i]->connect_input(accels[i - 1].get());
            }
            if(i + 1 < ACCEL_COUNT) {
                if(mode == Mode::DIR_SIMPLE) {
                    auto wr = VPE::self().fds()->get(pipes[i]->writer_fd());
                    accels[i]->connect_output(static_cast<GenericFile*>(wr.get()));
                }
                else
                    accels[i]->connect_output(accels[i + 1].get());
            }
        }
    }

    void start() {
        for(size_t i = 0; i < ACCEL_COUNT; ++i) {
            vpes[i]->start();
            running[i] = true;
        }
    }

    void add_running(capsel_t *sels, size_t *count) {
        for(size_t i = 0; i < ACCEL_COUNT; ++i) {
            if(running[i])
                sels[(*count)++] = vpes[i]->sel();
        }
    }
    void terminated(capsel_t vpe, int exitcode) {
        for(size_t i = 0; i < ACCEL_COUNT; ++i) {
            if(running[i] && vpes[i]->sel() == vpe) {
                if(exitcode != 0) {
                    cerr << "chain" << i
                         << " terminated with exit code " << exitcode << "\n";
                }
                if(mode == Mode::DIR_SIMPLE) {
                    if(pipes[i])
                        pipes[i]->close_writer();
                    if(i > 0 && pipes[i - 1])
                        pipes[i - 1]->close_reader();
                }
                running[i] = false;
                break;
            }
        }
    }

private:
    Mode mode;
    Reference<PE> pes[ACCEL_COUNT];
    std::unique_ptr<VPE> vpes[ACCEL_COUNT];
    std::unique_ptr<StreamAccel> accels[ACCEL_COUNT];
    std::unique_ptr<IndirectPipe> pipes[ACCEL_COUNT];
    std::unique_ptr<MemGate> mems[ACCEL_COUNT];
    bool running[ACCEL_COUNT];
};

static void wait_for(std::unique_ptr<DirectChain> *chains, size_t num) {
    for(size_t rem = num * DirectChain::ACCEL_COUNT; rem > 0; --rem) {
        size_t count = 0;
        capsel_t sels[num * DirectChain::ACCEL_COUNT];
        for(size_t i = 0; i < num; ++i)
            chains[i]->add_running(sels, &count);

        capsel_t vpe;
        int exitcode = Syscalls::vpe_wait(sels, rem, 0, &vpe);
        for(size_t i = 0; i < num; ++i)
            chains[i]->terminated(vpe, exitcode);
    }
}

cycles_t chain_direct(const char *in, size_t num, Mode mode) {
    Pipes pipes("pipes");
    std::unique_ptr<DirectChain> chains[num];
    fd_t infds[num];
    fd_t outfds[num];

    // create <num> chains
    for(size_t i = 0; i < num; ++i) {
        OStringStream outpath;
        outpath << "/tmp/res-" << i;

        infds[i] = VFS::open(in, FILE_R);
        outfds[i] = VFS::open(outpath.str(), FILE_W | FILE_TRUNC | FILE_CREATE);

        chains[i] = std::make_unique<DirectChain>(pipes,
                                                  i,
                                                  VPE::self().fds()->get(infds[i]),
                                                  VPE::self().fds()->get(outfds[i]),
                                                  mode);
    }

    if(VERBOSE) Serial::get() << "Starting chain...\n";

    cycles_t start = Time::start(0);

    if(mode == Mode::DIR) {
        for(size_t i = 0; i < num; ++i)
            chains[i]->start();
        wait_for(chains, num);
    }
    else {
        for(size_t i = 0; i < num / 2; ++i)
            chains[i]->start();
        wait_for(chains, num / 2);
        for(size_t i = num / 2; i < num; ++i)
            chains[i]->start();
        wait_for(chains + num / 2, num / 2);
    }

    cycles_t end = Time::stop(0);

    // cleanup
    for(size_t i = 0; i < num; ++i) {
        VFS::close(infds[i]);
        VFS::close(outfds[i]);
    }

    return end - start;
}
