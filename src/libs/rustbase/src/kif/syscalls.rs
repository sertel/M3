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

//! The system call interface

use core::mem::MaybeUninit;

/// The maximum message length that can be used
pub const MAX_MSG_SIZE: usize = 440;

/// The maximum size of strings in system calls
pub const MAX_STR_SIZE: usize = 32;

/// The maximum number of arguments for the exchange syscalls
pub const MAX_EXCHG_ARGS: usize = 8;

int_enum! {
    /// The system calls
    pub struct Operation : u64 {
        // sent by the DTU if the PF handler is not reachable
        const PAGEFAULT         = 0;

        // capability creations
        const CREATE_SRV        = 1;
        const CREATE_SESS       = 2;
        const CREATE_RGATE      = 3;
        const CREATE_SGATE      = 4;
        const CREATE_MAP        = 5;
        const CREATE_VPEGRP     = 6;
        const CREATE_VPE        = 7;
        const CREATE_SEM        = 8;

        // capability operations
        const ACTIVATE          = 9;
        const VPE_CTRL          = 10;
        const VPE_WAIT          = 11;
        const DERIVE_MEM        = 12;
        const DERIVE_KMEM       = 13;
        const KMEM_QUOTA        = 14;
        const SEM_CTRL          = 15;

        // capability exchange
        const DELEGATE          = 16;
        const OBTAIN            = 17;
        const EXCHANGE          = 18;
        const REVOKE            = 19;

        // forwarding
        const FORWARD_MSG       = 20;
        const FORWARD_MEM       = 21;
        const FORWARD_REPLY     = 22;

        // misc
        const NOOP              = 23;
    }
}

/// The default reply message that only contains the error code
#[repr(C, packed)]
pub struct DefaultReply {
    pub error: u64,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct ExchangeUnionStr {
    pub i: [u64; 2],
    pub s: [u8; 48],
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub union ExchangeUnion {
    pub i: [u64; MAX_EXCHG_ARGS],
    pub s: ExchangeUnionStr,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct ExchangeArgs {
    pub count: u64,
    pub vals: ExchangeUnion,
}

impl ExchangeArgs {
    pub fn ival(&self, idx: usize) -> u64 {
        unsafe { self.vals.i[idx] }
    }
    pub fn sval(&self, idx: usize) -> u64 {
        unsafe { self.vals.s.i[idx] }
    }
    pub fn str(&self) -> &[u8] {
        unsafe { &self.vals.s.s }
    }
}

impl Default for ExchangeArgs {
    fn default() -> Self {
        ExchangeArgs {
            count: 0,
            vals: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }
}

/// The pagefault request message
#[repr(C, packed)]
pub struct Pagefault {
    pub opcode: u64,
    pub virt: u64,
    pub access: u64,
}

/// The create service request message
#[repr(C, packed)]
pub struct CreateSrv {
    pub opcode: u64,
    pub dst_sel: u64,
    pub vpe_sel: u64,
    pub rgate_sel: u64,
    pub namelen: u64,
    pub name: [u8; MAX_STR_SIZE],
}

/// The create session request message
#[repr(C, packed)]
pub struct CreateSess {
    pub opcode: u64,
    pub dst_sel: u64,
    pub srv_sel: u64,
    pub ident: u64,
}

/// The create receive gate request message
#[repr(C, packed)]
pub struct CreateRGate {
    pub opcode: u64,
    pub dst_sel: u64,
    pub order: u64,
    pub msgorder: u64,
}

/// The create send gate request message
#[repr(C, packed)]
pub struct CreateSGate {
    pub opcode: u64,
    pub dst_sel: u64,
    pub rgate_sel: u64,
    pub label: u64,
    pub credits: u64,
}

/// The create mapping request message
#[repr(C, packed)]
pub struct CreateMap {
    pub opcode: u64,
    pub dst_sel: u64,
    pub vpe_sel: u64,
    pub mgate_sel: u64,
    pub first: u64,
    pub pages: u64,
    pub perms: u64,
}

/// The create VPE group request message
#[repr(C, packed)]
pub struct CreateVPEGrp {
    pub opcode: u64,
    pub dst_sel: u64,
}

/// The create VPE request message
#[repr(C, packed)]
pub struct CreateVPE {
    pub opcode: u64,
    pub dst_crd: u64,
    pub sgate_sel: u64,
    pub pe: u64,
    pub sep: u64,
    pub rep: u64,
    pub muxable: u64,
    pub group_sel: u64,
    pub kmem_sel: u64,
    pub namelen: u64,
    pub name: [u8; MAX_STR_SIZE],
}

/// The create VPE reply message
#[repr(C, packed)]
pub struct CreateVPEReply {
    pub error: u64,
    pub pe: u64,
}

/// The create semaphore request message
#[repr(C, packed)]
pub struct CreateSem {
    pub opcode: u64,
    pub dst_sel: u64,
    pub value: u64,
}

/// The activate request message
#[repr(C, packed)]
pub struct Activate {
    pub opcode: u64,
    pub ep_sel: u64,
    pub gate_sel: u64,
    pub addr: u64,
}

int_enum! {
    /// The operations for the `vpe_ctrl` system call
    pub struct VPEOp : u64 {
        const INIT  = 0x0;
        const START = 0x1;
        const YIELD = 0x2;
        const STOP  = 0x3;
    }
}

/// The VPE control request message
#[repr(C, packed)]
pub struct VPECtrl {
    pub opcode: u64,
    pub vpe_sel: u64,
    pub op: u64,
    pub arg: u64,
}

/// The VPE wait request message
#[repr(C, packed)]
pub struct VPEWait {
    pub opcode: u64,
    pub vpe_count: u64,
    pub event: u64,
    pub sels: [u64; 48],
}

/// The VPE wait reply message
#[repr(C, packed)]
pub struct VPEWaitReply {
    pub error: u64,
    pub vpe_sel: u64,
    pub exitcode: u64,
}

/// The derive memory request message
#[repr(C, packed)]
pub struct DeriveMem {
    pub opcode: u64,
    pub vpe_sel: u64,
    pub dst_sel: u64,
    pub src_sel: u64,
    pub offset: u64,
    pub size: u64,
    pub perms: u64,
}

/// The derive kernel memory request message
#[repr(C, packed)]
pub struct DeriveKMem {
    pub opcode: u64,
    pub kmem_sel: u64,
    pub dst_sel: u64,
    pub quota: u64,
}

/// The kernel memory quota request message
#[repr(C, packed)]
pub struct KMemQuota {
    pub opcode: u64,
    pub kmem_sel: u64,
}

/// The kernel memory quota reply message
#[repr(C, packed)]
pub struct KMemQuotaReply {
    pub error: u64,
    pub amount: u64,
}

int_enum! {
    /// The operations for the `sem_ctrl` system call
    pub struct SemOp : u64 {
        const UP   = 0x0;
        const DOWN = 0x1;
    }
}

/// The semaphore control request message
#[repr(C, packed)]
pub struct SemCtrl {
    pub opcode: u64,
    pub sem_sel: u64,
    pub op: u64,
}

/// The exchange request message
#[repr(C, packed)]
pub struct Exchange {
    pub opcode: u64,
    pub vpe_sel: u64,
    pub own_crd: u64,
    pub other_sel: u64,
    pub obtain: u64,
}

/// The delegate/obtain request message
#[repr(C, packed)]
pub struct ExchangeSess {
    pub opcode: u64,
    pub vpe_sel: u64,
    pub sess_sel: u64,
    pub crd: u64,
    pub args: ExchangeArgs,
}

/// The delegate/obtain reply message
#[repr(C, packed)]
pub struct ExchangeSessReply {
    pub error: u64,
    pub args: ExchangeArgs,
}

/// The revoke request message
#[repr(C, packed)]
pub struct Revoke {
    pub opcode: u64,
    pub vpe_sel: u64,
    pub crd: u64,
    pub own: u64,
}

/// The forward message request message
#[repr(C, packed)]
pub struct ForwardMsg {
    pub opcode: u64,
    pub sgate_sel: u64,
    pub rgate_sel: u64,
    pub len: u64,
    pub rlabel: u64,
    pub event: u64,
    pub msg: [u8; MAX_MSG_SIZE],
}

bitflags! {
    /// The flags for the `forward_mem` system call
    pub struct ForwardMemFlags : u32 {
        const NOPF      = 0x1;
        const WRITE     = 0x2;
    }
}

/// The forward memory request message
#[repr(C, packed)]
pub struct ForwardMem {
    pub opcode: u64,
    pub mgate_sel: u64,
    pub len: u64,
    pub offset: u64,
    pub flags: u64,
    pub event: u64,
    pub data: [u8; MAX_MSG_SIZE],
}

/// The forward memory reply message
#[repr(C, packed)]
pub struct ForwardMemReply {
    pub error: u64,
    pub data: [u8; MAX_MSG_SIZE],
}

/// The forward reply request message
#[repr(C, packed)]
pub struct ForwardReply {
    pub opcode: u64,
    pub rgate_sel: u64,
    pub msgaddr: u64,
    pub len: u64,
    pub event: u64,
    pub msg: [u8; MAX_MSG_SIZE],
}

/// The noop request message
#[repr(C, packed)]
pub struct Noop {
    pub opcode: u64,
}
