/*
 * Copyright (C) 2018 Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * Copyright (C) 2019-2022 Nils Asmussen, Barkhausen Institut
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

use base::backtrace;
use base::int_enum;
use base::libc;
use base::tcu;

use core::arch::asm;
use core::fmt;

use crate::IRQSource;

pub const ISR_COUNT: usize = 8;
pub const TCU_ISR: usize = Vector::IRQ.val;

pub const TMC_ARG0: usize = 0; // r0
pub const TMC_ARG1: usize = 1; // r1
pub const TMC_ARG2: usize = 2; // r2
pub const TMC_ARG3: usize = 3; // r3
pub const TMC_ARG4: usize = 4; // r4

#[derive(Default)]
// for some reason, we need to specify the alignment here. actually, this struct needs to be packed,
// but unfortunately, we cannot specify both packed and align. but without packed seems to be fine,
// because there are no holes between the fields.
#[repr(C, align(4))]
pub struct State {
    pub sp: usize,
    pub lr: usize,
    pub vec: usize,
    pub r: [usize; 13], // r0 .. r12
    pub klr: usize,
    pub pc: usize,
    pub cpsr: usize,
}

impl State {
    pub fn base_pointer(&self) -> usize {
        self.r[11]
    }

    #[allow(clippy::verbose_bit_mask)]
    pub fn came_from_user(&self) -> bool {
        (self.cpsr & 0x0F) == 0x0
    }
}

int_enum! {
    pub struct Vector : usize {
        const RESET = 0;
        const UNDEF_INSTR = 1;
        const SWI = 2;
        const PREFETCH_ABORT = 3;
        const DATA_ABORT = 4;
        const RESERVED = 5;
        const IRQ = 6;
        const FIQ = 7;
    }
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(fmt, "  lr:     {:#x}", { self.lr })?;
        writeln!(fmt, "  sp:     {:#x}", { self.sp })?;
        writeln!(
            fmt,
            "  vec:    {:#x} ({})",
            { self.vec },
            Vector::from(self.vec)
        )?;
        writeln!(fmt, "  klr:    {:#x}", { self.klr })?;
        for (idx, r) in { self.r }.iter().enumerate() {
            writeln!(fmt, "  r[{:02}]:  {:#x}", idx, r)?;
        }
        writeln!(fmt, "  pc:     {:#x}", { self.pc })?;
        writeln!(fmt, "  cpsr:   {:#x}", { self.cpsr })?;

        writeln!(fmt, "\nUser backtrace:")?;
        let mut bt = [0usize; 16];
        let bt_len = backtrace::collect_for(self.base_pointer(), &mut bt);
        for addr in bt.iter().take(bt_len) {
            writeln!(fmt, "  {:#x}", addr)?;
        }
        Ok(())
    }
}

#[no_mangle]
pub extern "C" fn isr_handler(state: &mut State) -> *mut libc::c_void {
    // repeat last instruction
    if state.vec == 4 {
        state.pc -= 8;
    }
    // repeat last instruction, except for SWIs
    else if state.vec != 2 {
        state.pc -= 4;
    }

    crate::ISRS.borrow()[state.vec](state)
}

pub fn init(_state: &mut State) {
    // nothing to do
}

pub fn init_tmcalls(handler: crate::IsrFunc) {
    crate::reg(Vector::SWI.val, handler);
}

pub fn set_entry_sp(_sp: usize) {
    // nothing to do
}

pub fn enable_irqs() {
    unsafe { asm!("msr cpsr, 0x53") };
}

pub fn get_irq() -> IRQSource {
    let irq = tcu::TCU::get_irq();
    tcu::TCU::clear_irq(irq);
    IRQSource::TCU(irq)
}

pub fn register_ext_irq(_irq: u32) {
}

pub fn enable_ext_irqs(_mask: u32) {
}

pub fn disable_ext_irqs(_mask: u32) {
}
