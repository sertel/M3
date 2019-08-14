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

use base::libc;
use core::fmt;
use isr;

type IsrFunc = extern "C" fn(state: &mut isr::State) -> *mut libc::c_void;

extern "C" {
    fn isr_init();
    fn isr_reg(idx: usize, func: IsrFunc);
    fn isr_enable();

    static isr_stack: libc::c_void;
}

pub const DPL_KERNEL: u64 = 0;
pub const DPL_USER: u64 = 3;

pub const SEG_KCODE: u64 = 1;
pub const SEG_KDATA: u64 = 2;
pub const SEG_UCODE: u64 = 3;
pub const SEG_UDATA: u64 = 4;

#[repr(C, packed)]
pub struct State {
    /* general purpose registers */
    pub r: [u64; 15],
    /* interrupt-number */
    pub irq: u64,
    /* error-code (for exceptions); default = 0 */
    pub error: u64,
    /* pushed by the CPU */
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

fn vec_name(vec: u64) -> &'static str {
    match vec {
        0x00 => "Divide by zero",
        0x01 => "Single step",
        0x02 => "Non maskable",
        0x03 => "Breakpoint",
        0x04 => "Overflow",
        0x05 => "Bounds check",
        0x06 => "Invalid opcode",
        0x07 => "Co-proc. n/a",
        0x08 => "Double fault",
        0x09 => "Co-proc seg. overrun",
        0x0A => "Invalid TSS",
        0x0B => "Segment not present",
        0x0C => "Stack exception",
        0x0D => "Gen. prot. fault",
        0x0E => "Page fault",
        0x10 => "Co-processor error",
        _ => "<unknown>",
    }
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(fmt, "State @ {:#x}", self as *const State as usize)?;
        writeln!(fmt, "  vec: {:#x} ({})", { self.irq }, vec_name(self.irq))?;
        writeln!(fmt, "  error:  {:#x}", { self.error })?;
        writeln!(fmt, "  rip:    {:#x}", { self.rip })?;
        writeln!(fmt, "  rflags: {:#x}", { self.rflags })?;
        writeln!(fmt, "  rsp:    {:#x}", { self.rsp })?;
        writeln!(fmt, "  cs:     {:#x}", { self.cs })?;
        writeln!(fmt, "  ss:     {:#x}", { self.ss })?;
        for (idx, r) in { self.r }.iter().enumerate() {
            writeln!(fmt, "  r[{:02}]:  {:#x}", idx, r)?;
        }
        Ok(())
    }
}

impl State {
    pub fn init(&mut self, entry: usize, sp: usize) {
        self.rip = entry as u64;
        self.rsp = sp as u64;
        self.r[8] = 0; // rbp
        self.r[14] = 0xDEADBEEF; // set rax to tell crt0 that we've set the SP

        self.rflags = 0x200; // enable interrupts
                             // run in user mode
        self.cs = (isr::SEG_UCODE << 3) | DPL_USER;
        self.ss = (isr::SEG_UDATA << 3) | DPL_USER;
    }

    pub fn stop(&mut self) {
        self.rip = crate::sleep as *const fn() as u64;
        self.rsp = unsafe { &isr_stack as *const libc::c_void as u64 };
        self.r[8] = self.rsp; // rbp and rsp

        self.rflags = 0x200; // enable interrupts
        self.cs = (isr::SEG_KCODE << 3) | DPL_KERNEL;
        self.ss = (isr::SEG_KDATA << 3) | DPL_KERNEL;
    }
}

pub fn init() {
    unsafe {
        isr_init();
        for i in 0..=64 {
            match i {
                14 => isr_reg(i, crate::mmu_pf),
                63 => isr_reg(i, crate::stop_irq),
                64 => isr_reg(i, crate::dtu_irq),
                i => isr_reg(i, crate::unexpected_irq),
            }
        }
        isr_enable();
    }
}