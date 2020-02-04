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

use cap::Selector;
use cell::RefCell;
use col::Vec;
use com::{RecvGate, SendGate, SliceSource, VecSink};
use core::any::Any;
use core::cmp;
use core::fmt;
use core::mem::MaybeUninit;
use errors::Error;
use goff;
use kif;
use pes::VPE;
use rc::{Rc, Weak};
use serialize::Sink;
use session::ClientSession;
use vfs::{
    FSHandle, FSOperation, FileHandle, FileInfo, FileMode, FileSystem, GenericFile, OpenFlags,
};

/// The type of extent ids.
pub type ExtId = u16;

/// Represents a session at m3fs.
pub struct M3FS {
    self_weak: Weak<RefCell<M3FS>>,
    sess: ClientSession,
    sgate: Rc<SendGate>,
}

impl M3FS {
    fn create(sess: ClientSession, sgate: SendGate) -> FSHandle {
        let inst = Rc::new(RefCell::new(M3FS {
            self_weak: Weak::new(),
            sess,
            sgate: Rc::new(sgate),
        }));
        inst.borrow_mut().self_weak = Rc::downgrade(&inst);
        inst
    }

    /// Creates a new session at the m3fs server with given name.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> Result<FSHandle, Error> {
        let sels = VPE::cur().alloc_sels(2);
        let sess = ClientSession::new_with_sel(name, sels + 1)?;

        let crd = kif::CapRngDesc::new(kif::CapType::OBJECT, sels + 0, 1);
        let mut args = kif::syscalls::ExchangeArgs::default();
        sess.obtain_for(VPE::cur().sel(), crd, &mut args)?;
        let sgate = SendGate::new_bind(sels + 0);
        Ok(Self::create(sess, sgate))
    }

    /// Binds a new m3fs-session to selectors `sels`..`sels+1`.
    pub fn new_bind(sels: Selector) -> FSHandle {
        Self::create(
            ClientSession::new_bind(sels + 0),
            SendGate::new_bind(sels + 1),
        )
    }

    /// Returns a reference to the underlying [`ClientSession`]
    pub fn sess(&self) -> &ClientSession {
        &self.sess
    }

    pub fn get_mem(sess: &ClientSession, off: goff) -> Result<(goff, goff, Selector), Error> {
        let mut args = kif::syscalls::ExchangeArgs::default();
        // safety: we initialize the value below
        unsafe { args.set_count(1) };
        args.set_ival(0, off as u64);
        let crd = sess.obtain(1, &mut args)?;
        Ok((args.ival(0) as goff, args.ival(1) as goff, crd.start()))
    }
}

impl FileSystem for M3FS {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, Error> {
        #[allow(clippy::uninit_assumed_init)]
        let mut args = kif::syscalls::ExchangeArgs::new(1, kif::syscalls::ExchangeUnion {
            s: kif::syscalls::ExchangeUnionStr {
                i: [u64::from(flags.bits()), 0],
                // safety: will be initialized via set_str below
                s: unsafe { MaybeUninit::uninit().assume_init() },
            },
        });
        args.set_str(path);

        let crd = self.sess.obtain(2, &mut args)?;
        Ok(Rc::new(RefCell::new(GenericFile::new(flags, crd.start()))))
    }

    fn stat(&self, path: &str) -> Result<FileInfo, Error> {
        let mut reply = send_recv_res!(&self.sgate, RecvGate::def(), FSOperation::STAT, path)?;
        Ok(reply.pop())
    }

    fn mkdir(&self, path: &str, mode: FileMode) -> Result<(), Error> {
        send_recv_res!(&self.sgate, RecvGate::def(), FSOperation::MKDIR, path, mode).map(|_| ())
    }

    fn rmdir(&self, path: &str) -> Result<(), Error> {
        send_recv_res!(&self.sgate, RecvGate::def(), FSOperation::RMDIR, path).map(|_| ())
    }

    fn link(&self, old_path: &str, new_path: &str) -> Result<(), Error> {
        send_recv_res!(
            &self.sgate,
            RecvGate::def(),
            FSOperation::LINK,
            old_path,
            new_path
        )
        .map(|_| ())
    }

    fn unlink(&self, path: &str) -> Result<(), Error> {
        send_recv_res!(&self.sgate, RecvGate::def(), FSOperation::UNLINK, path).map(|_| ())
    }

    fn fs_type(&self) -> u8 {
        b'M'
    }

    fn exchange_caps(
        &self,
        vpe: Selector,
        dels: &mut Vec<Selector>,
        max_sel: &mut Selector,
    ) -> Result<(), Error> {
        dels.push(self.sess.sel());

        let crd = kif::CapRngDesc::new(kif::CapType::OBJECT, self.sess.sel() + 1, 1);
        let mut args = kif::syscalls::ExchangeArgs::default();
        self.sess.obtain_for(vpe, crd, &mut args)?;
        *max_sel = cmp::max(*max_sel, self.sess.sel() + 2);
        Ok(())
    }

    fn serialize(&self, s: &mut VecSink) {
        s.push(&self.sess.sel());
        s.push(&self.sgate.sel());
    }
}

impl M3FS {
    pub fn unserialize(s: &mut SliceSource) -> FSHandle {
        let sels: Selector = s.pop();
        M3FS::new_bind(sels)
    }
}

impl fmt::Debug for M3FS {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "M3FS[sess={:?}, sgate={:?}]", self.sess, self.sgate)
    }
}