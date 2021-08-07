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

#include <base/Common.h>
#include <base/stream/OStream.h>
#include <base/TCU.h>
#include <base/Errors.h>

#include <utility>

namespace m3 {

/**
 * The kernel interface
 */
struct KIF {
    KIF() = delete;

    /**
     * Represents an invalid selector
     */
    static const capsel_t INV_SEL       = 0xFFFF;

    /**
     * Represents unlimited credits
     */
    static const uint UNLIM_CREDITS     = TCU::UNLIM_CREDITS;

    /**
     * The maximum message length that can be used
     */
    static const size_t MAX_MSG_SIZE    = 440;

    /**
     * The maximum string length in messages
     */
    static const size_t MAX_STR_SIZE    = 64;

    static const capsel_t SEL_PE        = 0;
    static const capsel_t SEL_KMEM      = 1;
    static const capsel_t SEL_VPE       = 2;

    /**
     * The first selector for the endpoint capabilities
     */
    static const uint FIRST_FREE_SEL    = SEL_VPE + 1;

    /**
     * The VPE id of PEMux
     */
    static const uint PEMUX_VPE_ID      = 0xFFFF;

    /**
     * The permissions for MemGate
     */
    struct Perm {
        static const uint R = 1;
        static const uint W = 2;
        static const uint X = 4;
        static const uint RW = R | W;
        static const uint RWX = R | W | X;
    };

    /**
     * The flags for virtual mappings
     */
    struct PageFlags {
        static const uint R = Perm::R;
        static const uint W = Perm::W;
        static const uint X = Perm::X;
        static const uint RW = R | W;
        static const uint RX = R | X;
        static const uint RWX = R | W | X;
    };

    enum VPEFlags {
        // whether the PE can be shared with others
        MUXABLE     = 1,
        // whether this VPE gets pinned on one PE
        PINNED      = 2,
    };

    struct CapRngDesc {
        typedef xfer_t value_type;

        enum Type {
            OBJ,
            MAP,
        };

        explicit CapRngDesc() : CapRngDesc(OBJ, 0, 0) {
        }
        explicit CapRngDesc(const value_type raw[2])
            : _start(raw[0]),
              _count(raw[1]) {
        }
        explicit CapRngDesc(Type type, capsel_t start, capsel_t count = 1)
            : _start(start),
              _count(static_cast<value_type>(type) | (count << 1)) {
        }

        Type type() const {
            return static_cast<Type>(_count & 1);
        }
        capsel_t start() const {
            return _start;
        }
        capsel_t count() const {
            return _count >> 1;
        }

        void to_raw(value_type *raw) const {
            raw[0] = _start;
            raw[1] = _count;
        }

        friend OStream &operator <<(OStream &os, const CapRngDesc &crd) {
            os << "CRD[" << (crd.type() == OBJ ? "OBJ" : "MAP") << ":"
               << crd.start() << ":" << crd.count() << "]";
            return os;
        }

    private:
        value_type _start;
        value_type _count;
    };

    struct DefaultReply {
        xfer_t error;
    } PACKED;

    struct DefaultRequest {
        xfer_t opcode;
    } PACKED;

    struct ExchangeArgs {
        xfer_t bytes;
        unsigned char data[64];
    } PACKED;

    /**
     * System calls
     */
    struct Syscall {
        enum Operation {
            // capability creations
            CREATE_SRV,
            CREATE_SESS,
            CREATE_MGATE,
            CREATE_RGATE,
            CREATE_SGATE,
            CREATE_MAP,
            CREATE_VPE,
            CREATE_SEM,
            ALLOC_EPS,

            // capability operations
            ACTIVATE,
            SET_PMP,
            VPE_CTRL,
            VPE_WAIT,
            DERIVE_MEM,
            DERIVE_KMEM,
            DERIVE_PE,
            DERIVE_SRV,
            GET_SESS,
            KMEM_QUOTA,
            PE_QUOTA,
            SEM_CTRL,

            // capability exchange
            DELEGATE,
            OBTAIN,
            EXCHANGE,
            REVOKE,

            // misc
            RESET_STATS,
            NOOP,

            COUNT
        };

        enum VPEOp {
            VCTRL_INIT,
            VCTRL_START,
            VCTRL_STOP,
        };

        enum SemOp {
            SCTRL_UP,
            SCTRL_DOWN,
        };

        struct CreateSrv : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t rgate_sel;
            xfer_t creator;
            xfer_t namelen;
            char name[MAX_STR_SIZE];
        } PACKED;

        struct CreateSess : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t srv_sel;
            xfer_t creator;
            xfer_t ident;
            xfer_t auto_close;
        } PACKED;

        struct CreateMGate : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t vpe_sel;
            xfer_t addr;
            xfer_t size;
            xfer_t perms;
        } PACKED;

        struct CreateRGate : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t order;
            xfer_t msgorder;
        } PACKED;

        struct CreateSGate : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t rgate_sel;
            xfer_t label;
            xfer_t credits;
        } PACKED;

        struct CreateMap : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t vpe_sel;
            xfer_t mgate_sel;
            xfer_t first;
            xfer_t pages;
            xfer_t perms;
        } PACKED;

        struct CreateVPE : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t pg_sg_sel;
            xfer_t pg_rg_sel;
            xfer_t pe_sel;
            xfer_t kmem_sel;
            xfer_t namelen;
            char name[MAX_STR_SIZE];
        } PACKED;

        struct CreateVPEReply : public DefaultReply {
            xfer_t eps_start;
        } PACKED;

        struct CreateSem : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t value;
        } PACKED;

        struct AllocEP : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t vpe_sel;
            xfer_t epid;
            xfer_t replies;
        } PACKED;

        struct AllocEPReply : public DefaultReply {
            xfer_t ep;
        } PACKED;

        struct Activate : public DefaultRequest {
            xfer_t ep_sel;
            xfer_t gate_sel;
            xfer_t rbuf_mem;
            xfer_t rbuf_off;
        } PACKED;

        struct SetPMP : public DefaultRequest {
            xfer_t pe_sel;
            xfer_t mgate_sel;
            xfer_t epid;
        } PACKED;

        struct VPECtrl : public DefaultRequest {
            xfer_t vpe_sel;
            xfer_t op;
            xfer_t arg;
        } PACKED;

        struct VPEWait : public DefaultRequest {
            xfer_t vpe_count;
            xfer_t event;
            xfer_t sels[48];
        } PACKED;

        struct VPEWaitReply : public DefaultReply {
            xfer_t vpe_sel;
            xfer_t exitcode;
        } PACKED;

        struct DeriveMem : public DefaultRequest {
            xfer_t vpe_sel;
            xfer_t dst_sel;
            xfer_t src_sel;
            xfer_t offset;
            xfer_t size;
            xfer_t perms;
        } PACKED;

        struct DeriveKMem : public DefaultRequest {
            xfer_t kmem_sel;
            xfer_t dst_sel;
            xfer_t quota;
        } PACKED;

        struct DerivePE : public DefaultRequest {
            xfer_t pe_sel;
            xfer_t dst_sel;
            xfer_t eps;
        } PACKED;

        struct DeriveSrv : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t srv_sel;
            xfer_t sessions;
            xfer_t event;
        } PACKED;

        struct GetSession : public DefaultRequest {
            xfer_t dst_sel;
            xfer_t srv_sel;
            xfer_t vpe_sel;
            xfer_t sid;
        } PACKED;

        struct KMemQuota : public DefaultRequest {
            xfer_t kmem_sel;
        } PACKED;

        struct KMemQuotaReply : public DefaultReply {
            xfer_t amount;
        } PACKED;

        struct PEQuota : public DefaultRequest {
            xfer_t pe_sel;
        } PACKED;

        struct PEQuotaReply : public DefaultReply {
            xfer_t amount;
        } PACKED;

        struct SemCtrl : public DefaultRequest {
            xfer_t sem_sel;
            xfer_t op;
        } PACKED;

        struct Exchange : public DefaultRequest {
            xfer_t vpe_sel;
            xfer_t own_caps[2];
            xfer_t other_sel;
            xfer_t obtain;
        } PACKED;

        struct ExchangeSess : public DefaultRequest {
            xfer_t vpe_sel;
            xfer_t sess_sel;
            xfer_t caps[2];
            ExchangeArgs args;
        } PACKED;

        struct ExchangeSessReply : public DefaultReply {
            ExchangeArgs args;
        } PACKED;

        struct Revoke : public DefaultRequest {
            xfer_t vpe_sel;
            xfer_t caps[2];
            xfer_t own;
        } PACKED;

        struct ResetStats : public DefaultRequest {
        } PACKED;

        struct Noop : public DefaultRequest {
        } PACKED;
    };

    /**
     * Service calls
     */
    struct Service {
        enum Operation {
            OPEN,
            DERIVE_CRT,
            OBTAIN,
            DELEGATE,
            CLOSE,
            SHUTDOWN
        };

        struct Open : public DefaultRequest {
            xfer_t arglen;
            char arg[MAX_STR_SIZE];
        } PACKED;

        struct OpenReply : public DefaultReply {
            xfer_t sess;
            xfer_t ident;
        } PACKED;

        struct DeriveCreator : public DefaultRequest {
            xfer_t sessions;
        } PACKED;

        struct DeriveCreatorReply : public DefaultReply {
            xfer_t creator;
            xfer_t sgate_sel;
        } PACKED;

        struct ExchangeData {
            xfer_t caps[2];
            ExchangeArgs args;
        } PACKED;

        struct Exchange : public DefaultRequest {
            xfer_t sess;
            ExchangeData data;
        } PACKED;

        struct ExchangeReply : public DefaultReply {
            ExchangeData data;
        } PACKED;

        struct Close : public DefaultRequest {
            xfer_t sess;
        } PACKED;

        struct Shutdown : public DefaultRequest {
        } PACKED;
    };

    /**
     * Upcalls
     */
    struct Upcall {
        enum Operation {
            DERIVE_SRV,
            VPEWAIT,
        };

        struct DefaultUpcall : public DefaultRequest {
            xfer_t event;
        } PACKED;

        struct VPEWait : public DefaultUpcall {
            xfer_t error;
            xfer_t vpe_sel;
            xfer_t exitcode;
        } PACKED;
    };
};

}
