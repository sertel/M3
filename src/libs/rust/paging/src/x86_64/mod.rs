/*
 * Copyright (C) 2020-2022 Nils Asmussen, Barkhausen Institut
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

use base::cfg;
use base::cpu;
use base::kif::PageFlags;

use bitflags::bitflags;

use core::arch::asm;

pub type MMUPTE = u64;
pub type Phys = u64;

pub const PTE_BITS: usize = 3;

pub const LEVEL_CNT: usize = 4;
pub const LEVEL_BITS: usize = cfg::PAGE_BITS - PTE_BITS;
pub const LEVEL_MASK: usize = (1 << LEVEL_BITS) - 1;

bitflags! {
    pub struct MMUFlags : MMUPTE {
        const P     = 0b0000_0001;
        const W     = 0b0000_0010;
        const U     = 0b0000_0100;
        const L     = 0b1000_0000;
        const NX    = 0x8000_0000_0000_0000;

        const RW    = Self::P.bits | Self::W.bits | Self::NX.bits;
        const RWX   = Self::P.bits | Self::W.bits;

        const FLAGS = cfg::PAGE_MASK as MMUPTE | Self::NX.bits;
    }
}

impl MMUFlags {
    pub fn has_empty_perm(self) -> bool {
        !self.contains(MMUFlags::P)
    }

    pub fn is_leaf(self, level: usize) -> bool {
        level == 0 || self.contains(Self::L)
    }

    pub fn perms_missing(self, perms: Self) -> bool {
        !self.contains(Self::P)
            || (!self.contains(Self::W) && perms.contains(Self::W))
            || (self.contains(Self::NX) && !perms.contains(Self::NX))
    }
}

pub fn build_pte(phys: MMUPTE, perm: MMUFlags, level: usize, leaf: bool) -> MMUPTE {
    let pte = phys | perm.bits();
    if leaf {
        if level > 0 {
            pte | MMUFlags::L.bits()
        }
        else {
            pte
        }
    }
    else {
        pte | (MMUFlags::RWX | MMUFlags::U).bits()
    }
}

pub fn pte_to_phys(pte: MMUPTE) -> Phys {
    pte & !MMUFlags::FLAGS.bits()
}

pub fn needs_invalidate(new_flags: MMUFlags, old_flags: MMUFlags) -> bool {
    old_flags.bits() != 0 && new_flags.perms_missing(old_flags)
}

pub fn to_page_flags(_level: usize, pte: MMUFlags) -> PageFlags {
    let mut res = PageFlags::empty();
    if pte.contains(MMUFlags::P) {
        res |= PageFlags::R;
    }
    if pte.contains(MMUFlags::W) {
        res |= PageFlags::W;
    }
    if pte.contains(MMUFlags::U) {
        res |= PageFlags::U;
    }
    if !pte.contains(MMUFlags::NX) {
        res |= PageFlags::X;
    }
    if pte.contains(MMUFlags::L) {
        res |= PageFlags::L;
    }
    res
}

pub fn to_mmu_perms(flags: PageFlags) -> MMUFlags {
    let mut res = MMUFlags::empty();
    if flags.intersects(PageFlags::RWX) {
        res |= MMUFlags::P;
    }
    if flags.contains(PageFlags::W) {
        res |= MMUFlags::W;
    }
    if flags.contains(PageFlags::U) {
        res |= MMUFlags::U;
    }
    if !flags.contains(PageFlags::X) {
        res |= MMUFlags::NX;
    }
    res
}

pub fn enable_paging() {
    // already enabled by gem5
}

pub fn disable_paging() {
    // not possible/necessary
}

pub fn invalidate_page(_id: crate::ActId, virt: usize) {
    unsafe {
        asm!(
            "invlpg [{0}]",
            in(reg) virt,
            options(nostack),
        );
    }
}

pub fn invalidate_tlb() {
    // nothing to do
}

pub fn set_root_pt(_id: crate::ActId, root: Phys) {
    cpu::write_cr3(root as usize);
}
