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

use alloc::{format, vec};
use core::ptr;
use libc;

use crate::arch::envdata;
use crate::arch::tcu::{thread, EpId, Header, PEId, EP_COUNT, PE_COUNT};
use crate::col::Vec;
use crate::util;

pub(crate) struct SocketBackend {
    sock: i32,
    knotify_sock: i32,
    knotify_addr: libc::sockaddr_un,
    localsock: Vec<i32>,
    eps: Vec<libc::sockaddr_un>,
}

#[repr(C, packed)]
#[derive(Default)]
struct KNotifyData {
    pid: libc::pid_t,
    status: i32,
}

impl SocketBackend {
    fn get_sock_addr(addr: &str) -> libc::sockaddr_un {
        let mut sockaddr = libc::sockaddr_un {
            sun_family: libc::AF_UNIX as libc::sa_family_t,
            sun_path: [0; 108],
        };
        sockaddr.sun_path[0..addr.len()]
            .clone_from_slice(unsafe { &*(addr.as_bytes() as *const [u8] as *const [i8]) });
        sockaddr
    }

    fn ep_idx(pe: PEId, ep: EpId) -> usize {
        pe as usize * EP_COUNT as usize + ep as usize
    }

    pub fn new() -> SocketBackend {
        let sock = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM, 0) };
        assert!(sock != -1);

        let knotify_sock = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM, 0) };
        assert!(knotify_sock != -1);
        unsafe {
            assert!(libc::fcntl(knotify_sock, libc::F_SETFD, libc::FD_CLOEXEC) == 0);
        }
        let knotify_name = format!("\0{}/knotify\0", envdata::tmp_dir());
        let knotify_addr = Self::get_sock_addr(&knotify_name);

        let mut eps = vec![];
        for pe in 0..PE_COUNT {
            for ep in 0..EP_COUNT {
                let addr = format!("\0{}/ep_{}.{}\0", envdata::tmp_dir(), pe, ep);
                eps.push(Self::get_sock_addr(&addr));
            }
        }

        let pe = envdata::get().pe_id as PEId;
        let mut localsock = vec![];
        for ep in 0..EP_COUNT {
            unsafe {
                let epsock = libc::socket(libc::AF_UNIX, libc::SOCK_DGRAM, 0);
                assert!(epsock != -1);

                assert!(libc::fcntl(epsock, libc::F_SETFD, libc::FD_CLOEXEC) == 0);

                assert!(
                    libc::bind(
                        epsock,
                        &eps[Self::ep_idx(pe, ep)] as *const libc::sockaddr_un
                            as *const libc::sockaddr,
                        util::size_of::<libc::sockaddr_un>() as u32
                    ) == 0
                );

                localsock.push(epsock);
            }
        }

        SocketBackend {
            sock,
            knotify_sock,
            knotify_addr,
            localsock,
            eps,
        }
    }

    pub fn send(&self, pe: PEId, ep: EpId, buf: &thread::Buffer) -> bool {
        let sock = &self.eps[Self::ep_idx(pe, ep)];
        let res = unsafe {
            libc::sendto(
                self.sock,
                buf as *const thread::Buffer as *const libc::c_void,
                buf.header.length + util::size_of::<Header>(),
                0,
                sock as *const libc::sockaddr_un as *const libc::sockaddr,
                util::size_of::<libc::sockaddr_un>() as u32,
            )
        };
        res != -1
    }

    pub fn receive(&self, ep: EpId, buf: &mut thread::Buffer) -> Option<usize> {
        let res = unsafe {
            libc::recvfrom(
                self.localsock[ep as usize],
                buf as *mut thread::Buffer as *mut libc::c_void,
                util::size_of::<thread::Buffer>(),
                libc::MSG_DONTWAIT,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if res <= 0 {
            None
        }
        else {
            Some(res as usize)
        }
    }

    pub fn notify_kernel(&self, pid: libc::pid_t, status: i32) {
        let data = KNotifyData { pid, status };
        unsafe {
            let res = libc::sendto(
                self.knotify_sock,
                &data as *const KNotifyData as *const libc::c_void,
                util::size_of::<KNotifyData>(),
                0,
                &self.knotify_addr as *const libc::sockaddr_un as *const libc::sockaddr,
                util::size_of::<libc::sockaddr_un>() as u32,
            );
            assert!(res != -1);
        }
    }

    pub fn bind_knotify(&self) {
        unsafe {
            assert!(
                libc::bind(
                    self.knotify_sock,
                    &self.knotify_addr as *const libc::sockaddr_un as *const libc::sockaddr,
                    util::size_of::<libc::sockaddr_un>() as u32
                ) != -1
            );
        }
    }

    pub fn receive_knotify(&self) -> Option<(libc::pid_t, i32)> {
        let mut data = KNotifyData::default();

        let res = unsafe {
            libc::recvfrom(
                self.knotify_sock,
                &mut data as *mut KNotifyData as *mut libc::c_void,
                util::size_of::<KNotifyData>(),
                libc::MSG_DONTWAIT,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if res <= 0 {
            None
        }
        else {
            Some((data.pid, data.status))
        }
    }

    pub fn shutdown(&self) {
        for ep in 0..EP_COUNT {
            unsafe { libc::shutdown(self.localsock[ep as usize], libc::SHUT_RD) };
        }
    }
}

impl Drop for SocketBackend {
    fn drop(&mut self) {
        for ep in 0..EP_COUNT {
            unsafe { libc::close(self.localsock[ep as usize]) };
        }
    }
}
