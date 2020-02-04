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

//! Contains session-related abstractions.

mod clisession;
mod m3fs;
mod pager;
mod pipe;
mod resmng;
mod srvsession;

pub use self::clisession::ClientSession;
pub use self::m3fs::{ExtId, M3FS};
pub use self::pager::{MapFlags, Pager, PagerDelOp, PagerOp};
pub use self::pipe::{Pipe, Pipes};
pub use self::resmng::{ResMng, ResMngOperation};
pub use self::srvsession::ServerSession;