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

use m3::println;
use m3::tiles::Activity;

#[no_mangle]
pub fn main() -> i32 {
    let (num, _) = Activity::cur()
        .resmng()
        .unwrap()
        .get_activity_count()
        .expect("Unable to get Activity count");
    println!(
        "{:2} {:2} {:>10} {:>24} {:>20} {:>20} {:>12} {}",
        "ID", "Tile", "EPs", "Time", "UserMem", "KernelMem", "PTs", "Name"
    );
    for i in 0..num {
        match Activity::cur().resmng().unwrap().get_activity_info(i) {
            Ok(act) => {
                println!(
                    "{:2} {:2} {:2}:{:3}/{:3} {:4}:{:7}us/{:7}us {:2}:{:7}K/{:7}K {:2}:{:7}K/{:7}K {:4}:{:3}/{:3} {:0l$}{}",
                    act.id,
                    act.tile,
                    act.eps.id(),
                    act.eps.left(),
                    act.eps.total(),
                    act.time.id(),
                    act.time.left() / 1000,
                    act.time.total() / 1000,
                    act.umem.id(),
                    act.umem.left() / 1024,
                    act.umem.total() / 1024,
                    act.kmem.id(),
                    act.kmem.left() / 1024,
                    act.kmem.total() / 1024,
                    act.pts.id(),
                    act.pts.left(),
                    act.pts.total(),
                    "",
                    act.name,
                    l = act.layer as usize * 2,
                );
            },
            Err(e) => println!(
                "Unable to get info about Activity with idx {}: {:?}",
                i,
                e.code()
            ),
        }
    }
    0
}
