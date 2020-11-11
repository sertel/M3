/*
 * Copyright (C) 2015-2020, Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Copyright (C) 2019-2020, Tendsin Mende <tendsin@protonmail.com>
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

use crate::backend::{Backend, SuperBlock};
use crate::buf::{LoadLimit, MetaBufferBlock};
use crate::data::{BlockNo, BlockRange, Extent};

use m3::cap::Selector;
use m3::com::{MGateArgs, MemGate, Perm};
use m3::errors::Error;
use m3::goff;
use m3::syscalls::derive_mem;
use thread::Event;

pub struct MemBackend {
    mem: MemGate,
    blocksize: usize,
}

impl MemBackend {
    pub fn new(fsoff: goff, fssize: usize) -> Self {
        MemBackend {
            mem: MemGate::new_with(MGateArgs::new(fssize, Perm::RWX).addr(fsoff))
                .expect("Could not create MemGate for memory backend"),
            blocksize: 0, // gets set when the superblock is read
        }
    }
}

impl Backend for MemBackend {
    fn load_meta(
        &self,
        dst: &mut MetaBufferBlock,
        _dst_off: usize,
        bno: BlockNo,
        _unlock: Event,
    ) -> Result<(), Error> {
        self.mem.read_bytes(
            dst.data_mut().as_mut_ptr(),
            self.blocksize,
            (bno as usize * self.blocksize) as u64,
        )
    }

    fn load_data(
        &self,
        _mem: &MemGate,
        _blocks: BlockRange,
        _init: bool,
        _unlock: Event,
    ) -> Result<(), Error> {
        // unused
        Ok(())
    }

    fn store_meta(
        &self,
        src: &MetaBufferBlock,
        _src_off: usize,
        bno: BlockNo,
        _unlock: Event,
    ) -> Result<(), Error> {
        let slice: &[u8] = src.data();

        self.mem.write(slice, bno as u64 * self.blocksize as u64)
    }

    fn store_data(&self, _blocks: BlockRange, _unlock: Event) -> Result<(), Error> {
        // unused
        Ok(())
    }

    fn sync_meta(&self, bno: BlockNo) -> Result<(), Error> {
        crate::hdl().metabuffer().write_back(bno)
    }

    fn get_filedata(
        &self,
        ext: Extent,
        extoff: usize,
        perms: Perm,
        sel: Selector,
        _load: Option<&mut LoadLimit>,
    ) -> Result<usize, Error> {
        let first_block = extoff / self.blocksize;
        let bytes: usize = (ext.length as usize - first_block) * self.blocksize as usize;
        let size = ((ext.start as usize + first_block) * self.blocksize) as u64;
        derive_mem(
            m3::pes::VPE::cur().sel(),
            sel,
            self.mem.sel(),
            size,
            bytes,
            perms,
        )?;
        Ok(bytes)
    }

    fn clear_extent(&self, ext: Extent) -> Result<(), Error> {
        let zeros = vec![0; self.blocksize];
        for block in ext.blocks() {
            self.mem
                .write(&zeros, (block as usize * self.blocksize) as u64)?;
        }
        Ok(())
    }

    fn load_sb(&mut self) -> Result<SuperBlock, Error> {
        let block = self.mem.read_obj::<SuperBlock>(0)?;
        self.blocksize = block.block_size as usize;
        Ok(block)
    }

    fn store_sb(&self, super_block: &SuperBlock) -> Result<(), Error> {
        self.mem.write_obj(super_block, 0)
    }
}