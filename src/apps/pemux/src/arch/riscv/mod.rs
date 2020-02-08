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

use base::errors::Error;
use base::libc;
use core::fmt;

type IsrFunc = extern "C" fn(state: &mut State) -> *mut libc::c_void;

extern "C" {
    fn isr_init();
    fn isr_reg(idx: usize, func: IsrFunc);
    fn isr_enable();

    static idle_stack: libc::c_void;
}

int_enum! {
    struct Vector : usize {
        // exceptions
        const INSTR_MISALIGNED = 0;
        const INSTR_ACC_FAULT = 1;
        const ILLEGAL_INSTR = 2;
        const BREAKPOINT = 3;
        const LOAD_MISALIGNED = 4;
        const LOAD_ACC_FAULT = 5;
        const STORE_MISALIGNED = 6;
        const STORE_ACC_FAULT = 7;
        const ENV_UCALL = 8;
        const ENV_SCALL = 9;
        const ENV_MCALL = 11;
        const INSTR_PAGEFAULT = 12;
        const LOAD_PAGEFAULT = 13;
        const STORE_PAGEFAULT = 15;

        // interrupts
        const USER_SW_IRQ = 16;
        const SUPER_SW_IRQ = 17;
        const MACH_SW_IRQ = 19;
        const USER_TIMER_IRQ = 20;
        const SUPER_TIMER_IRQ = 21;
        const MACH_TIMER_IRQ = 23;
        const USER_EXT_IRQ = 24;
        const SUPER_EXT_IRQ = 25;
        const MACH_EXT_IRQ = 27;
    }
}

#[repr(C, packed)]
pub struct State {
    // general purpose registers
    pub r: [usize; 31],
    pub cause: usize,
    pub sepc: usize,
}

pub const PEXC_ARG0: usize = 9; // a0 = x10
pub const PEXC_ARG1: usize = 10; // a1 = x11

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let vec = if (self.cause & 0x80000000) != 0 {
            16 + (self.cause & 0xF)
        }
        else {
            self.cause & 0xF
        };

        writeln!(fmt, "State @ {:#x}", self as *const State as usize)?;
        writeln!(fmt, "  vec: {:#x} ({})", vec, Vector::from(vec))?;
        for (idx, r) in { self.r }.iter().enumerate() {
            writeln!(fmt, "  r[{:02}]:  {:#x}", idx + 1, r)?;
        }
        writeln!(fmt, "  cause:  {:#x}", { self.cause })?;
        Ok(())
    }
}

impl State {
    pub fn came_from_user(&self) -> bool {
        unimplemented!();
    }

    pub fn init(&mut self, entry: usize, sp: usize) {
        self.r[9] = 0xDEADBEEF; // a0; don't set the stackpointer in crt0
        self.sepc = entry;
        self.r[1] = sp;
        // TODO self.cpsr = 0x10; // user mode
    }

    pub fn stop(&mut self) {
        self.sepc = crate::sleep as *const fn() as usize;
        self.r[1] = unsafe { &idle_stack as *const libc::c_void as usize };
        // TODO self.cpsr = 0x13; // supervisor mode
    }
}

pub fn enable_ints() -> bool {
    let prev: u64;
    unsafe {
        asm!(
            "csrr $0, sie; csrs sie, $1; fence.i"
            : "=r"(prev)
            : "r"(1 << 9)
            : "memory"
        );
    }
    (prev & (1 << 9)) != 0
}

pub fn restore_ints(prev: bool) {
    if !prev {
        unsafe { asm!("fence.i; csrc sie, $0" : : "r"(1 << 9)) };
    }
}

pub fn init() {
    unsafe {
        isr_init();
        for i in 0..=31 {
            match Vector::from(i) {
                Vector::ENV_SCALL => isr_reg(i, crate::pexcall),
                // Vector::PREFETCH_ABORT => isr_reg(i, crate::mmu_pf),
                // Vector::DATA_ABORT => isr_reg(i, crate::mmu_pf),
                Vector::SUPER_EXT_IRQ => isr_reg(i, crate::dtu_irq),
                _ => isr_reg(i, crate::unexpected_irq),
            }
        }
        isr_enable();
    }
}

pub fn handle_mmu_pf(_state: &mut State) -> Result<(), Error> {
    unimplemented!();
}
