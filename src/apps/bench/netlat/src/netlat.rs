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

#![no_std]
#![feature(core_intrinsics)]

mod budplat;

use m3::cell::LazyStaticCell;
use m3::col::Vec;
use m3::env;
use m3::net::{IpAddr, Port};
use m3::test::WvTester;
use m3::{println, wv_run_suite};

pub static DST_IP: LazyStaticCell<IpAddr> = LazyStaticCell::default();
pub static DST_PORT: LazyStaticCell<Port> = LazyStaticCell::default();

struct MyTester {}

impl WvTester for MyTester {
    fn run_suite(&mut self, name: &str, f: &dyn Fn(&mut dyn WvTester)) {
        println!("Running benchmark suite {} ...\n", name);
        f(self);
        println!();
    }

    fn run_test(&mut self, name: &str, file: &str, f: &dyn Fn()) {
        println!("Testing \"{}\" in {}:", name, file);
        f();
        println!();
    }
}

#[no_mangle]
pub fn main() -> i32 {
    let args: Vec<&str> = env::args().collect();
    if args.len() != 3 {
        println!("Usage: {} <ip> <port>", args[0]);
        m3::exit(1);
    }

    DST_IP.set(
        args[1]
            .parse::<IpAddr>()
            .expect(&m3::format!("Invalid IP address: {}", args[1])),
    );
    DST_PORT.set(
        args[2]
            .parse::<Port>()
            .expect(&m3::format!("Invalid port number: {}", args[2])),
    );

    let mut tester = MyTester {};
    wv_run_suite!(tester, budplat::run);

    println!("\x1B[1;32mAll tests successful!\x1B[0;m");
    0
}