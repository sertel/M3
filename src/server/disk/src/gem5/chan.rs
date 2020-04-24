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

use m3::col::Vec;
use m3::com::MemGate;
use m3::errors::{Code, Error};
use m3::goff;
use m3::rc::Rc;
use m3::tcu;
use m3::time::Time;
use pci;

use super::ctrl::IDE_CTRL_BAR;
use super::device::{ATAReg, BMIReg, CommandStatus, DevOp, Device};
use super::PartDesc;
use Operation;

pub struct Channel {
    id: u8,
    use_irq: bool,
    use_dma: bool,
    port_base: u16,
    bmr_base: u16,
    pci_dev: Rc<pci::Device>,
    devs: Vec<Device>,
}

impl Channel {
    pub fn new(
        pci_dev: Rc<pci::Device>,
        ide_ctrl: &pci::Info,
        use_irq: bool,
        use_dma: bool,
        id: u8,
        port_base: u16,
    ) -> Result<Self, Error> {
        let mut chan = Self {
            id,
            use_irq,
            use_dma,
            port_base,
            bmr_base: ide_ctrl.bars[IDE_CTRL_BAR].addr as u16,
            pci_dev,
            devs: Vec::new(),
        };
        log!(
            crate::LOG_ALL,
            "chan[{}] initializing with ports={}, bmr={}",
            id,
            port_base,
            chan.bmr_base,
        );

        chan.check_bus()?;

        // init DMA
        if use_dma && chan.bmr_base != 0 {
            chan.bmr_base += id as u16 * 0x8;
            log!(crate::LOG_ALL, "chan[{}] using DMA", chan.id);
        }
        else {
            chan.use_dma = false;
        }

        // init attached devices; begin with slave
        for i in (0..2).rev() {
            let did = id * 2 + i;
            match Device::new(did, &chan) {
                Err(e) => log!(
                    crate::LOG_DEF,
                    "chan[{}] ignoring device {}: {}",
                    id,
                    did,
                    e
                ),
                Ok(d) => {
                    log!(
                        crate::LOG_DEF,
                        "chan[{}] found device {}: {} MiB",
                        id,
                        did,
                        d.size() / (1024 * 1024)
                    );
                    for p in d.partitions() {
                        log!(
                            crate::LOG_DEF,
                            "chan[{}] registered partition {}: {}, {}",
                            id,
                            p.id,
                            p.start * 512,
                            p.size * 512
                        );
                    }
                    chan.devs.push(d)
                },
            }
        }

        Ok(chan)
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn use_dma(&self) -> bool {
        self.use_dma
    }

    pub fn use_irq(&self) -> bool {
        self.use_irq
    }

    pub fn devices(&self) -> &Vec<Device> {
        &self.devs
    }

    pub fn read_write(
        &self,
        desc: PartDesc,
        op: Operation,
        buf: &MemGate,
        buf_off: usize,
        disk_off: usize,
        bytes: usize,
    ) -> Result<(), Error> {
        let dev = &self.devs[desc.device as usize];

        // check arguments
        let part_size = desc.part.size as usize * dev.sector_size();
        if disk_off.checked_add(bytes).is_none() || disk_off + bytes > part_size {
            log!(
                crate::LOG_DEF,
                "Invalid request: disk_off={}, bytes={}, part-size: {}",
                disk_off,
                bytes,
                part_size
            );
            return Err(Error::new(Code::InvArgs));
        }

        let lba = desc.part.start as u64 + disk_off as u64 / dev.sector_size() as u64;
        let count = bytes / dev.sector_size();

        let dev_op = match op {
            Operation::READ => DevOp::READ,
            _ => DevOp::WRITE,
        };

        log!(
            crate::LOG_DEF,
            "chan[{}] {:?} {} sectors at {}",
            self.id,
            dev_op,
            count,
            lba,
        );

        self.set_dma_buffer(buf)?;

        dev.read_write(self, dev_op, buf, buf_off, lba, dev.sector_size(), count)
    }

    pub fn set_dma_buffer(&self, mgate: &MemGate) -> Result<(), Error> {
        self.pci_dev.set_dma_buffer(mgate)
    }

    pub fn select(&self, id: u8, extra: u8) -> Result<(), Error> {
        log!(
            crate::LOG_ALL,
            "chan[{}] selecting device {:x} with {:x}",
            self.id,
            id,
            extra
        );
        self.write_pio(ATAReg::DRIVE_SELECT, extra | ((id & 0x1) << 4))
            .and_then(|_| {
                self.wait();
                Ok(())
            })
    }

    pub fn wait(&self) {
        for _ in 0..4 {
            self.pci_dev
                .read_config::<u8>((self.port_base + ATAReg::STATUS.val) as goff)
                .unwrap();
        }
    }

    pub fn wait_irq(&self) -> Result<(), Error> {
        if self.use_irq {
            log!(crate::LOG_ALL, "chan[{}] waiting for IRQ...", self.id);
            self.pci_dev.wait_for_irq()
        }
        else {
            Ok(())
        }
    }

    pub fn wait_until(
        &self,
        timeout: Time,
        sleep: Time,
        set: CommandStatus,
        unset: CommandStatus,
    ) -> Result<(), Error> {
        log!(
            crate::LOG_ALL,
            "chan[{}] waiting for set={:?}, unset={:?}",
            self.id,
            set,
            unset
        );

        let mut elapsed = 0;
        while elapsed < timeout {
            let status: u8 = self.read_pio(ATAReg::STATUS)?;
            if (status & CommandStatus::ERROR.bits()) != 0 {
                // TODO convert error code
                self.read_pio(ATAReg::ERROR)?;
                return Err(Error::new(Code::InvArgs));
            }
            if (status & set.bits()) == set.bits() && (status & unset.bits()) == 0 {
                return Ok(());
            }
            if sleep > 0 {
                tcu::TCUIf::sleep_for(sleep)?;
                elapsed += sleep;
            }
            else {
                elapsed += 1;
            }
        }

        Err(Error::new(Code::Timeout))
    }

    pub fn read_pio<T>(&self, reg: ATAReg) -> Result<T, Error> {
        self.pci_dev.read_reg((self.port_base + reg.val) as goff)
    }

    pub fn write_pio<T>(&self, reg: ATAReg, val: T) -> Result<(), Error> {
        self.pci_dev
            .write_reg((self.port_base + reg.val) as goff, val)
    }

    pub fn read_pio_words(&self, reg: ATAReg, buf: &mut [u16]) -> Result<(), Error> {
        for b in buf.iter_mut() {
            *b = self.read_pio(reg)?;
        }
        Ok(())
    }

    pub fn write_pio_words(&self, reg: ATAReg, buf: &[u16]) -> Result<(), Error> {
        for b in buf.iter() {
            self.write_pio(reg, b)?;
        }
        Ok(())
    }

    pub fn read_bmr<T>(&self, reg: BMIReg) -> Result<T, Error> {
        self.pci_dev.read_reg((self.bmr_base + reg.val) as goff)
    }

    pub fn write_bmr<T>(&self, reg: BMIReg, val: T) -> Result<(), Error> {
        self.pci_dev
            .write_reg((self.bmr_base + reg.val) as goff, val)
    }

    fn check_bus(&self) -> Result<(), Error> {
        for i in (0..2).rev() {
            // begin with slave. master should respond if there is no slave
            self.write_pio::<u8>(ATAReg::DRIVE_SELECT, i << 4)?;
            self.wait();

            // write some arbitrary values to some registers
            self.write_pio(ATAReg::ADDRESS1, 0xF1u8)?;
            self.write_pio(ATAReg::ADDRESS2, 0xF2u8)?;
            self.write_pio(ATAReg::ADDRESS3, 0xF3u8)?;

            // if we can read them back, the bus is present
            // check for value, one must not be floating
            if self.read_pio::<u8>(ATAReg::ADDRESS1)? == 0xF1
                && self.read_pio::<u8>(ATAReg::ADDRESS2)? == 0xF2
                && self.read_pio::<u8>(ATAReg::ADDRESS3)? == 0xF3
            {
                return Ok(());
            }
        }
        Err(Error::new(Code::NotFound))
    }
}
