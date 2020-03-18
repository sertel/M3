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

#if defined(__gem5__)
#   include "arch/gem5/TCURegs.h"
#elif defined(__host__)
#   include "arch/host/TCURegs.h"
#endif

#include "Types.h"

namespace kernel {

class VPEDesc;

class TCUState {
    friend class TCU;

public:
    explicit TCUState() : _regs() {
    }

    void *get_ep(epid_t ep);
    void restore(const VPEDesc &vpe);

    void invalidate_ep(epid_t ep);

    void config_recv(epid_t ep, vpeid_t vpe, goff_t buf, uint order, uint msgorder, uint reply_eps);
    void config_send(epid_t ep, vpeid_t vpe, label_t lbl, peid_t pe, epid_t dstep, uint msgorder, uint crd);
    void config_mem(epid_t ep, vpeid_t vpe, peid_t pe, vpeid_t tvpe, goff_t addr, size_t size, uint perm);
    bool config_mem_cached(epid_t ep, peid_t pe, vpeid_t tvpe);

#if defined(__host__)
    void update_recv(epid_t ep, goff_t base);
#endif

private:
    TCURegs _regs;
};

}
