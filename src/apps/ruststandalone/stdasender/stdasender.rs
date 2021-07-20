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

#![feature(llvm_asm)]
#![no_std]

extern crate heap;

#[path = "../vmtest/helper.rs"]
mod helper;
#[path = "../vmtest/paging.rs"]
mod paging;

use base::log;
use base::math;
use base::mem::MsgBuf;
use base::tcu::{EpId, PEId, FIRST_USER_EP, TCU};

const LOG_DEF: bool = true;
const LOG_DETAIL: bool = false;
const LOG_PEXCALLS: bool = false;

const OWN_VPE: u16 = 0xFFFF;

const DST_PE: PEId = 0;
const DST_EP: EpId = FIRST_USER_EP;

const REP: EpId = FIRST_USER_EP;
const SEP: EpId = FIRST_USER_EP + 1;

const MSG_SIZE: usize = 64;

static RBUF: [u64; 64] = [0; 64];

#[no_mangle]
pub extern "C" fn env_run() {
    helper::init("stdasender");

    let msg_size = math::next_log2(MSG_SIZE);
    helper::config_local_ep(SEP, |regs| {
        TCU::config_send(regs, OWN_VPE, 0x1234, DST_PE, DST_EP, msg_size, 1);
    });

    let buf_ord = math::next_log2(RBUF.len());
    let (rbuf_virt, rbuf_phys) = helper::virt_to_phys(RBUF.as_ptr() as usize);
    helper::config_local_ep(REP, |regs| {
        TCU::config_recv(regs, OWN_VPE, rbuf_phys, buf_ord, buf_ord, None);
    });

    let mut msg = MsgBuf::new();
    msg.set::<u64>(0);

    log!(crate::LOG_DEF, "Hello World from sender!");

    // initial send; wait until receiver is ready
    while let Err(e) = TCU::send(SEP, &msg, 0x2222, REP) {
        log!(crate::LOG_DEF, "send failed: {}", e);
        // get credits back
        helper::config_local_ep(SEP, |regs| {
            TCU::config_send(regs, OWN_VPE, 0x1234, DST_PE, DST_EP, 6, 1);
        });
    }

    for _ in 0..100000 {
        // wait for reply
        let rmsg = loop {
            if let Some(m) = helper::fetch_msg(REP, rbuf_virt) {
                break m;
            }
        };
        assert_eq!({ rmsg.header.label }, 0x2222);
        log!(crate::LOG_DETAIL, "got reply {}", *rmsg.get_data::<u64>());

        // ack reply
        TCU::ack_msg(REP, TCU::msg_to_offset(rbuf_virt, rmsg)).unwrap();

        // send message
        TCU::send(SEP, &msg, 0x2222, REP).unwrap();
        msg.set(msg.get::<u64>() + 1);
    }

    // wait for ever
    loop {}
}