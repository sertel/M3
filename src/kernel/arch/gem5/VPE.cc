/*
 * Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#include <base/util/Math.h>
#include <base/log/Kernel.h>
#include <base/ELF.h>

#include "mem/MainMemory.h"
#include "pes/PEManager.h"
#include "pes/PEMux.h"
#include "pes/VPE.h"
#include "TCU.h"
#include "Platform.h"

namespace kernel {

static uint64_t loaded = 0;

static const m3::BootInfo::Mod *get_mod(const char *name, bool *first) {
    size_t i = 0;
    size_t namelen = strlen(name);
    for(auto mod = Platform::mods_begin(); mod != Platform::mods_end(); ++mod, ++i) {
        if(strncmp(mod->name, name, namelen) == 0 &&
           (mod->name[namelen] == '\0' || mod->name[namelen] == ' ')) {
            *first = (loaded & (static_cast<uint64_t>(1) << i)) == 0;
            loaded |= static_cast<uint64_t>(1) << i;
            return &*mod;
        }
    }
    return nullptr;
}

static m3::GlobAddr alloc_mem(size_t size, size_t align) {
    MainMemory::Allocation alloc = MainMemory::get().allocate(size, align);
    if(!alloc)
        PANIC("Not enough memory");
    return alloc.addr();
}

static void read_from_mod(const m3::BootInfo::Mod *mod, void *data, size_t size, size_t offset) {
    if(offset + size < offset || offset + size > mod->size)
        PANIC("Invalid ELF file: offset invalid");

    m3::GlobAddr global(mod->addr + offset);
    TCU::read_mem(VPEDesc(global.pe(), VPE::INVALID_ID), global.offset(), data, size);
}

static void copy_clear(const VPEDesc &vpe, uintptr_t virt, m3::GlobAddr global,
                       size_t size, bool clear) {
    TCU::copy_clear(vpe, virt, VPEDesc(global.pe(), VPE::INVALID_ID), global.offset(), size, clear);
}

static void map_segment(VPE &vpe, goff_t virt, m3::GlobAddr global, size_t size, uint perms) {
    if(Platform::pe(vpe.peid()).has_virtmem() || (perms & MapCapability::EXCL)) {
        capsel_t dst = virt >> PAGE_BITS;
        size_t pages = m3::Math::round_up(size, PAGE_SIZE) >> PAGE_BITS;
        vpe.kmem()->alloc(vpe, sizeof(MapObject) + sizeof(MapCapability));
        if(perms & MapCapability::EXCL)
            vpe.kmem()->alloc(vpe, pages * PAGE_SIZE);
        // these mappings cannot be changed or revoked by applications
        perms |= MapCapability::KERNEL;
        auto mapcap = new MapCapability(&vpe.mapcaps(), dst, pages, new MapObject(global, perms));
        if(Platform::pe(vpe.peid()).has_virtmem())
            mapcap->remap(global, perms);
        vpe.mapcaps().set(dst, mapcap);
    }

    if(!Platform::pe(vpe.peid()).has_virtmem())
        copy_clear(vpe.desc(), static_cast<uintptr_t>(virt), global, size, false);
}

static goff_t load_mod(VPE &vpe, const m3::BootInfo::Mod *mod, bool copy) {
    // load and check ELF header
    m3::ElfEh header;
    read_from_mod(mod, &header, sizeof(header), 0);

    if(header.e_ident[0] != '\x7F' || header.e_ident[1] != 'E' || header.e_ident[2] != 'L' ||
        header.e_ident[3] != 'F')
        PANIC("Invalid ELF file: invalid magic number");

    // map load segments
    goff_t end = 0;
    size_t off = header.e_phoff;
    for(uint i = 0; i < header.e_phnum; ++i, off += header.e_phentsize) {
        /* load program header */
        m3::ElfPh pheader;
        read_from_mod(mod, &pheader, sizeof(pheader), off);

        // we're only interested in non-empty load segments
        if(pheader.p_type != m3::PT_LOAD || pheader.p_memsz == 0)
            continue;

        uint perms = 0;
        if(pheader.p_flags & m3::PF_R)
            perms |= m3::KIF::PageFlags::R;
        if(pheader.p_flags & m3::PF_W)
            perms |= m3::KIF::PageFlags::W;
        if(pheader.p_flags & m3::PF_X)
            perms |= m3::KIF::PageFlags::X;

        goff_t offset = m3::Math::round_dn(static_cast<size_t>(pheader.p_offset), PAGE_SIZE);
        goff_t virt = m3::Math::round_dn(static_cast<size_t>(pheader.p_vaddr), PAGE_SIZE);

        // do we need new memory for this segment?
        if((copy && (perms & m3::KIF::PageFlags::W)) || pheader.p_filesz == 0) {
            // allocate memory
            size_t size = static_cast<size_t>((pheader.p_vaddr & PAGE_BITS) + pheader.p_memsz);
            size = m3::Math::round_up(size, PAGE_SIZE);
            m3::GlobAddr global = alloc_mem(size, PAGE_SIZE);

            // map it
            map_segment(vpe, virt, global, size, perms | MapCapability::EXCL);
            end = virt + size;

            // initialize it
            copy_clear(vpe.desc(), virt, m3::GlobAddr(mod->addr + offset),
                       size, pheader.p_filesz == 0);
        }
        else {
            assert(pheader.p_memsz == pheader.p_filesz);
            size_t size = (pheader.p_offset & PAGE_BITS) + pheader.p_filesz;
            map_segment(vpe, virt, m3::GlobAddr(mod->addr + offset), size, perms);
            end = virt + size;
        }
    }

    // create initial heap
    m3::GlobAddr global = alloc_mem(ROOT_HEAP_SIZE, LPAGE_SIZE);
    goff_t virt = m3::Math::round_up(end, static_cast<goff_t>(LPAGE_SIZE));
    map_segment(vpe, virt, global, ROOT_HEAP_SIZE, m3::KIF::PageFlags::RW | MapCapability::EXCL);

    return header.e_entry;
}

void VPE::load_root() {
    bool appFirst;
    const m3::BootInfo::Mod *mod = get_mod("root", &appFirst);
    if(!mod)
        PANIC("Unable to find boot module 'root'");

    if(Platform::pe(peid()).has_virtmem()) {
        // map stack for root
        goff_t virt = STACK_BOTTOM;
        m3::GlobAddr global = alloc_mem(STACK_TOP - virt, PAGE_SIZE);
        map_segment(*this, virt, global, STACK_TOP - virt,
                    m3::KIF::PageFlags::RW | MapCapability::EXCL);
    }

    // load app
    goff_t entry = load_mod(*this, mod, !appFirst);

    // copy arguments and arg pointers to buffer
    static const char *uargv[] = {"root"};
    char buffer[64];
    uint64_t *argptr = reinterpret_cast<uint64_t*>(buffer);
    char *args = buffer + 1 * sizeof(uint64_t);
    size_t off = static_cast<size_t>(args - buffer);
    *argptr++ = ENV_SPACE_START + off;
    strcpy(args, uargv[0]);

    // write buffer to the target PE
    size_t argssize = off + sizeof("root");
    TCU::write_mem(desc(), ENV_SPACE_START, buffer, argssize);

    // write env to targetPE
    m3::Env senv;
    memset(&senv, 0, sizeof(senv));

    senv.argc = 1;
    senv.argv = ENV_SPACE_START;
    senv.sp = STACK_TOP - sizeof(word_t);
    senv.entry = entry;
    senv.pe_desc = Platform::pe(peid()).value();
    senv.heap_size = ROOT_HEAP_SIZE;
    senv.rmng_sel = m3::KIF::INV_SEL;
    senv.first_sel = _first_sel;
    senv.first_std_ep = _eps_start;

    TCU::write_mem(desc(), ENV_START, &senv, sizeof(senv));
}

void VPE::init_memory() {
    // let PEMux load the address space
    if(Platform::pe(peid()).supports_pemux())
        PEManager::get().pemux(peid())->vpe_ctrl(this, m3::KIF::PEXUpcalls::VCTRL_INIT);

    _state = VPE::RUNNING;

    // root is loaded by us
    if(_flags & F_ROOT)
        load_root();
}

void VPE::init_eps() {
    auto pemux = PEManager::get().pemux(peid());
    vpeid_t vpe = Platform::is_shared(peid()) ? id() : VPE::INVALID_ID;

    RGateObject rgate(SYSC_MSGSIZE_ORD, SYSC_MSGSIZE_ORD);
    rgate.pe = Platform::kernel_pe();
    rgate.addr = 1;  // has to be non-zero
    rgate.ep = syscall_ep();
    rgate.add_ref(); // don't free this (on destruction of SGateObject)

    // configure syscall endpoint
    UNUSED m3::Errors::Code res;
    SGateObject mobj(&rgate, m3::ptr_to_label(this), 1);
    res = pemux->config_snd_ep(_eps_start + m3::TCU::SYSC_SEP_OFF, vpe, mobj);
    assert(res == m3::Errors::NONE);

    // attach syscall receive endpoint
    rgate.order = m3::nextlog2<SYSC_RBUF_SIZE>::val;
    rgate.msgorder = SYSC_RBUF_ORDER;
    rgate.addr = Platform::rbuf_std(peid(), id());
    res = pemux->config_rcv_ep(_eps_start + m3::TCU::SYSC_REP_OFF, vpe,
                               m3::TCU::NO_REPLIES, rgate);
    assert(res == m3::Errors::NONE);

    // attach upcall receive endpoint
    rgate.order = m3::nextlog2<UPCALL_RBUF_SIZE>::val;
    rgate.msgorder = UPCALL_RBUF_ORDER;
    rgate.addr += SYSC_RBUF_SIZE;
    res = pemux->config_rcv_ep(_eps_start + m3::TCU::UPCALL_REP_OFF, vpe,
                               _eps_start + m3::TCU::UPCALL_RPLEP_OFF, rgate);
    assert(res == m3::Errors::NONE);

    // attach default receive endpoint
    rgate.order = m3::nextlog2<DEF_RBUF_SIZE>::val;
    rgate.msgorder = DEF_RBUF_ORDER;
    rgate.addr += UPCALL_RBUF_SIZE;
    res = pemux->config_rcv_ep(_eps_start + m3::TCU::DEF_REP_OFF, vpe,
                               m3::TCU::NO_REPLIES, rgate);
    assert(res == m3::Errors::NONE);
}

}
