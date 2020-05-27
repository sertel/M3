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

use m3::boxed::Box;
use m3::cap::Selector;
use m3::cell::{RefCell, StaticCell};
use m3::cfg::PAGE_SIZE;
use m3::col::{String, ToString, Vec};
use m3::com::MemGate;
use m3::errors::{Code, Error};
use m3::goff;
use m3::kif::{boot, CapRngDesc, CapType, PEDesc, PEType, FIRST_FREE_SEL};
use m3::math;
use m3::pes::{PE, VPE};
use m3::rc::Rc;
use m3::tcu::PEId;
use m3::util;

use childs;
use config;
use memory;
use pes;
use sems;
use services;

//
// Our parent/kernel initializes our cap space as follows:
// +-----------+-------+-----+-----------+------+-----+----------+-------+-----+-----------+
// | boot info | mod_0 | ... | mod_{n-1} | pe_0 | ... | pe_{n-1} | mem_0 | ... | mem_{n-1} |
// +-----------+-------+-----+-----------+------+-----+----------+-------+-----+-----------+
// ^-- FIRST_FREE_SEL
//
const SUBSYS_SELS: Selector = FIRST_FREE_SEL;

static OUR_PE: StaticCell<Option<Rc<pes::PEUsage>>> = StaticCell::new(None);
static DELAYED: StaticCell<Vec<Box<childs::OwnChild>>> = StaticCell::new(Vec::new());

#[derive(Default)]
struct Arguments {
    share_pe: bool,
    share_kmem: bool,
}

pub struct Subsystem {
    info: boot::Info,
    mods: Vec<boot::Mod>,
    pes: Vec<boot::PE>,
    mems: Vec<boot::Mem>,
    servs: Vec<boot::Service>,
    cfg_str: String,
    cfg: config::AppConfig,
}

impl Subsystem {
    pub fn new() -> Result<Self, Error> {
        let mgate = MemGate::new_bind(SUBSYS_SELS);
        let mut off: goff = 0;

        let info: boot::Info = mgate.read_obj(0)?;
        off += util::size_of::<boot::Info>() as goff;

        let mods = mgate.read_into_vec::<boot::Mod>(info.mod_count as usize, off)?;
        off += util::size_of::<boot::Mod>() as goff * info.mod_count;

        let pes = mgate.read_into_vec::<boot::PE>(info.pe_count as usize, off)?;
        off += util::size_of::<boot::PE>() as goff * info.pe_count;

        let mems = mgate.read_into_vec::<boot::Mem>(info.mem_count as usize, off)?;
        off += util::size_of::<boot::Mem>() as goff * info.mem_count;

        let servs = mgate.read_into_vec::<boot::Service>(info.serv_count as usize, off)?;

        let cfg = Self::parse_config(&mods)?;

        let sub = Self {
            info,
            mods,
            pes,
            mems,
            servs,
            cfg_str: cfg.0,
            cfg: cfg.1,
        };
        sub.init();
        Ok(sub)
    }

    fn init(&self) {
        log!(crate::LOG_SUBSYS, "Boot modules:");
        for m in self.mods() {
            log!(crate::LOG_SUBSYS, "  {:?}", m);
        }

        log!(crate::LOG_SUBSYS, "Available PEs:");
        for (i, pe) in self.pes().iter().enumerate() {
            log!(crate::LOG_SUBSYS, "  {:?}", pe);
            pes::get().add(pe.id as PEId, self.get_pe(i));
        }

        log!(crate::LOG_SUBSYS, "Available memory:");
        for (i, mem) in self.mems().iter().enumerate() {
            let mem_mod = Rc::new(memory::MemMod::new(
                self.get_mem(i),
                mem.addr(),
                mem.size(),
                mem.reserved(),
            ));
            log!(crate::LOG_SUBSYS, "  {:?}", mem_mod);
            memory::container().add(mem_mod);
        }

        if !self.services().is_empty() {
            log!(crate::LOG_SUBSYS, "Services:");
            for (i, s) in self.services().iter().enumerate() {
                let sel = self.get_service(i);
                log!(
                    crate::LOG_SUBSYS,
                    "  Service[name={}, sessions={}]",
                    s.name(),
                    s.sessions()
                );
                services::get()
                    .add_service(sel, sel + 1, s.name().to_string(), s.sessions(), false)
                    .unwrap();
            }
        }

        if VPE::cur().resmng().is_none() {
            log!(crate::LOG_CFG, "Parsed {:?}", self.cfg);
        }
    }

    fn parse_config(mods: &[boot::Mod]) -> Result<(String, config::AppConfig), Error> {
        let mut cfg_mem: Option<(usize, goff)> = None;

        // find boot config
        for (id, m) in mods.iter().enumerate() {
            if m.name() == "boot.xml" {
                cfg_mem = Some((id, m.size));
                continue;
            }
        }

        // read boot config
        let cfg_mem = cfg_mem.unwrap();
        let memgate = MemGate::new_bind(SUBSYS_SELS + 1 + cfg_mem.0 as Selector);
        let xml = memgate.read_into_vec::<u8>(cfg_mem.1 as usize, 0)?;

        // parse boot config
        let xml_str = String::from_utf8(xml).map_err(|_| Error::new(Code::InvArgs))?;
        let cfg = config::AppConfig::parse(&xml_str)?;
        Ok((xml_str, cfg))
    }

    pub fn cfg_str(&self) -> &String {
        &self.cfg_str
    }

    pub fn cfg(&self) -> &config::AppConfig {
        &self.cfg
    }

    pub fn info(&self) -> &boot::Info {
        &self.info
    }

    pub fn mods(&self) -> &Vec<boot::Mod> {
        &self.mods
    }

    pub fn pes(&self) -> &Vec<boot::PE> {
        &self.pes
    }

    pub fn mems(&self) -> &Vec<boot::Mem> {
        &self.mems
    }

    pub fn services(&self) -> &Vec<boot::Service> {
        &self.servs
    }

    pub fn get_mod(&self, idx: usize) -> MemGate {
        MemGate::new_bind(SUBSYS_SELS + 1 + idx as Selector)
    }

    pub fn get_pe(&self, idx: usize) -> Rc<PE> {
        Rc::new(PE::new_bind(
            self.pes[idx].desc,
            SUBSYS_SELS + 1 + (self.mods.len() + idx) as Selector,
        ))
    }

    pub fn get_mem(&self, idx: usize) -> MemGate {
        MemGate::new_bind(SUBSYS_SELS + 1 + (self.mods.len() + self.pes.len() + idx) as Selector)
    }

    pub fn get_service(&self, idx: usize) -> Selector {
        SUBSYS_SELS + 1 + (self.mods.len() + self.pes.len() + self.mems.len() + idx * 2) as Selector
    }

    pub fn start<S>(&self, mut spawn: S) -> Result<(), Error>
    where
        S: FnMut(&mut childs::OwnChild) -> Result<(), Error>,
    {
        let root = self.cfg();
        if VPE::cur().resmng().is_none() {
            root.check();
        }

        let args = parse_args(&root);

        // keep our own PE to make sure that we allocate a different one for the next domain in case
        // our domain contains just ourself.
        if !args.share_pe {
            OUR_PE.set(Some(Rc::new(
                pes::get().find_and_alloc(VPE::cur().pe_desc())?,
            )));
        }
        else if !VPE::cur().pe_desc().has_virtmem() {
            panic!("Can't share root's PE without VM support");
        }

        // determine default mem and kmem per child
        let (def_kmem, def_umem) = split_mem(&root)?;

        for (dom_idx, d) in root.domains().iter().enumerate() {
            // we need virtual memory support for multiple apps per domain
            let cur_desc = VPE::cur().pe_desc();
            let pe_desc = if d.apps().len() > 1 {
                PEDesc::new(PEType::COMP_EMEM, cur_desc.isa(), 0)
            }
            else {
                cur_desc
            };

            // allocate new PE; root allocates from its own set, others ask their resmng
            let pe_usage = if (args.share_pe && dom_idx == 0) || VPE::cur().resmng().is_none() {
                Rc::new(pes::get().find_and_alloc(pe_desc)?)
            }
            else {
                Rc::new(pes::PEUsage::new_obj(PE::new(pe_desc)?))
            };

            let def_eps = split_eps(&pe_usage.pe_obj(), &d)?;

            for cfg in d.apps() {
                // determine PE object with potentially reduced number of EPs
                let pe_usage = if cfg.eps().is_none() {
                    pe_usage.clone()
                }
                else {
                    Rc::new(pe_usage.derive(cfg.eps().unwrap_or(def_eps))?)
                };

                // kernel memory for child
                let kmem = if cfg.kernel_mem().is_none() && args.share_kmem {
                    VPE::cur().kmem().clone()
                }
                else {
                    let kmem_bytes = cfg.kernel_mem().unwrap_or(def_kmem);
                    VPE::cur().kmem().derive(kmem_bytes)?
                };

                // determine user and child memory
                let mut user_mem = cfg.user_mem().unwrap_or(def_umem as usize) as goff;
                let child_mem = cfg.split_child_mem(&mut user_mem);

                // memory pool for child
                let mem_pool = Rc::new(RefCell::new(memory::container().alloc_pool(user_mem)?));
                // add requested physical memory regions to pool
                for mem in cfg.phys_mems() {
                    let mslice = memory::container().find_mem(mem.phys(), mem.size())?;
                    mem_pool.borrow_mut().add(mslice);
                }

                let sub = if !cfg.domains().is_empty() {
                    // create MemGate for config substring
                    let cfg_range = cfg.cfg_range();
                    let cfg_len = cfg_range.1 - cfg_range.0;
                    let cfg_slice = memory::container().alloc_mem(cfg_len as goff)?;
                    let cfg_mem = cfg_slice.derive()?;
                    cfg_mem.write(&self.cfg_str()[cfg_range.0..cfg_range.1].as_bytes(), 0)?;

                    let mut sub = SubsystemBuilder::new((cfg_mem, cfg_len));

                    // add PEs
                    sub.add_pe(pe_usage.pe_id(), pe_usage.pe_obj());
                    pass_down_pes(&mut sub, &cfg);

                    // add memory
                    let mut sub_pool = memory::container().alloc_pool(child_mem)?;
                    for s in sub_pool.slices_mut() {
                        let size = s.available();
                        let addr = s.allocate(size, 1)?;
                        let mgate = s.derive()?;
                        sub.add_mem(mgate, addr, size);
                    }
                    pass_down_mem(&mut sub, &cfg)?;

                    // add services
                    for s in cfg.sess_creators() {
                        let (sess_frac, sess_fixed) = split_sessions(root, s.serv_name());
                        sub.add_serv(s.serv_name().clone(), sess_frac, sess_fixed, s.sess_count());
                    }

                    Some(sub)
                }
                else {
                    None
                };

                let mut child = Box::new(childs::OwnChild::new(
                    childs::get().alloc_id(),
                    pe_usage,
                    // TODO either remove args and daemon from config or remove the clones from OwnChild
                    cfg.args().clone(),
                    cfg.daemon(),
                    kmem,
                    mem_pool,
                    cfg.clone(),
                    sub,
                ));
                log!(crate::LOG_CHILD, "Created {:?}", child);

                if child.has_unmet_reqs() {
                    DELAYED.get_mut().push(child);
                }
                else {
                    spawn(&mut child)?;
                    childs::get().add(child);
                }
            }
        }
        Ok(())
    }
}

pub struct SubsystemBuilder {
    desc: Option<MemGate>,
    cfg: (MemGate, usize),
    pes: Vec<(PEId, Rc<PE>)>,
    mems: Vec<(MemGate, goff, goff)>,
    servs: Vec<(String, u32, u32, Option<u32>)>,
    serv_objs: Vec<services::Service>,
}

impl SubsystemBuilder {
    pub fn new(cfg: (MemGate, usize)) -> Self {
        Self {
            desc: None,
            cfg,
            pes: Vec::new(),
            mems: Vec::new(),
            servs: Vec::new(),
            serv_objs: Vec::new(),
        }
    }

    pub fn add_pe(&mut self, id: PEId, pe: Rc<PE>) {
        self.pes.push((id, pe));
    }

    pub fn add_mem(&mut self, mem: MemGate, addr: goff, size: goff) {
        self.mems.push((mem, addr, size));
    }

    pub fn add_serv(&mut self, name: String, sess_frac: u32, sess_fixed: u32, quota: Option<u32>) {
        if self.servs.iter().find(|s| s.0 == name).is_none() {
            self.servs.push((name, sess_frac, sess_fixed, quota));
        }
    }

    pub fn desc_size(&self) -> usize {
        util::size_of::<boot::Info>()
            + util::size_of::<boot::Mod>() * 1
            + util::size_of::<boot::PE>() * self.pes.len()
            + util::size_of::<boot::Mem>() * self.mems.len()
            + util::size_of::<boot::Service>() * self.servs.len()
    }

    pub fn finalize(&mut self, vpe: &mut VPE) -> Result<(), Error> {
        let mut sel = SUBSYS_SELS;
        let mut off: goff = 0;

        let mem = memory::container()
            .alloc_mem(self.desc_size() as goff)?
            .derive()?;

        // boot info
        let info = boot::Info {
            mod_count: 1,
            pe_count: self.pes.len() as u64,
            mem_count: self.mems.len() as u64,
            serv_count: self.servs.len() as u64,
        };
        mem.write_obj(&info, off)?;
        vpe.delegate_to(CapRngDesc::new(CapType::OBJECT, mem.sel(), 1), sel)?;
        off += util::size_of::<boot::Info>() as goff;
        sel += 1;

        // boot module for config
        let m = boot::Mod::new(0, self.cfg.1 as u64, "boot.xml");
        mem.write_obj(&m, off)?;
        vpe.delegate_to(CapRngDesc::new(CapType::OBJECT, self.cfg.0.sel(), 1), sel)?;
        off += util::size_of::<boot::Mod>() as goff;
        sel += 1;

        // PEs
        for (id, pe) in &self.pes {
            let boot_pe = boot::PE::new(*id as u32, pe.desc());
            mem.write_obj(&boot_pe, off)?;

            vpe.delegate_to(CapRngDesc::new(CapType::OBJECT, pe.sel(), 1), sel)?;

            off += util::size_of::<boot::PE>() as goff;
            sel += 1;
        }

        // memory regions
        for (mgate, addr, size) in &self.mems {
            let boot_mem = boot::Mem::new(*addr, *size, false);
            mem.write_obj(&boot_mem, off)?;

            vpe.delegate_to(CapRngDesc::new(CapType::OBJECT, mgate.sel(), 1), sel)?;

            off += util::size_of::<boot::Mem>() as goff;
            sel += 1;
        }

        // services
        for (name, sess_frac, sess_fixed, sess_quota) in &self.servs {
            let serv = services::get().get(name).unwrap();
            let sessions = if let Some(quota) = sess_quota {
                *quota
            }
            else {
                if *sess_frac > (serv.sessions() - sess_fixed) {
                    return Err(Error::new(Code::NoSpace));
                }
                (serv.sessions() - sess_fixed) / sess_frac
            };
            let subserv = serv.derive(sessions)?;
            let boot_serv = boot::Service::new(name, sessions);
            mem.write_obj(&boot_serv, off)?;

            vpe.delegate_to(CapRngDesc::new(CapType::OBJECT, subserv.sel(), 1), sel)?;
            vpe.delegate_to(
                CapRngDesc::new(CapType::OBJECT, subserv.sgate_sel(), 1),
                sel + 1,
            )?;

            off += util::size_of::<boot::Service>() as goff;
            sel += 2;

            self.serv_objs.push(subserv);
        }

        self.desc = Some(mem);
        Ok(())
    }
}

pub(crate) fn start_delayed<S>(mut spawn: S) -> Result<(), Error>
where
    S: FnMut(&mut childs::OwnChild) -> Result<(), Error>,
{
    let mut new_wait = false;
    let mut idx = 0;
    let delayed = DELAYED.get_mut();
    while idx < delayed.len() {
        if delayed[idx].has_unmet_reqs() {
            idx += 1;
            continue;
        }

        let mut child = delayed.remove(idx);
        spawn(&mut child)?;
        childs::get().add(child);
        new_wait = true;
    }

    if new_wait {
        childs::get().start_waiting(1);
    }
    Ok(())
}

fn pass_down_pes(sub: &mut SubsystemBuilder, app: &config::AppConfig) {
    for d in app.domains() {
        for child in d.apps() {
            for pe in child.pes() {
                for _ in 0..pe.count() {
                    if let Some(idx) = pes::get().find(|p| pe.matches(p.desc())) {
                        pes::get().alloc(idx);
                        sub.add_pe(pes::get().id(idx), pes::get().get(idx));
                    }
                }
            }

            pass_down_pes(sub, child);
        }
    }
}

fn pass_down_mem(sub: &mut SubsystemBuilder, app: &config::AppConfig) -> Result<(), Error> {
    for d in app.domains() {
        for child in d.apps() {
            for pmem in child.phys_mems() {
                let slice = memory::container().find_mem(pmem.phys(), pmem.size())?;
                let mgate = slice.derive()?;
                sub.add_mem(mgate, pmem.phys(), pmem.size());
            }

            pass_down_mem(sub, child)?;
        }
    }
    Ok(())
}

fn parse_args(cfg: &config::AppConfig) -> Arguments {
    let mut args = Arguments::default();
    for arg in cfg.args() {
        if arg == "sharekmem" {
            args.share_kmem = true;
        }
        else if arg == "sharepe" {
            args.share_pe = true;
        }
        else if arg.starts_with("sem=") {
            sems::get()
                .add_sem(arg[4..].to_string())
                .expect("Unable to add semaphore");
        }
    }
    args
}

fn split_mem(cfg: &config::AppConfig) -> Result<(usize, goff), Error> {
    let mut total_umem = memory::container().capacity();
    let mut total_kmem = VPE::cur().kmem().quota()?;

    let mut total_kparties = cfg.count_apps() + 1;
    let mut total_mparties = total_kparties;
    for d in cfg.domains() {
        for a in d.apps() {
            if let Some(kmem) = a.kernel_mem() {
                total_kmem -= kmem;
                total_kparties -= 1;
            }

            if let Some(amem) = a.user_mem() {
                total_umem -= amem as goff;
                total_mparties -= 1;
            }
        }
    }

    let def_kmem = total_kmem / total_kparties;
    let def_umem = math::round_dn(total_umem / total_mparties as goff, PAGE_SIZE as goff);
    Ok((def_kmem, def_umem))
}

fn split_sessions(cfg: &config::AppConfig, name: &str) -> (u32, u32) {
    let mut frac = 0;
    let mut fixed = 0;
    for d in cfg.domains() {
        for a in d.apps() {
            for sess in a.sessions() {
                if sess.serv_name() == name {
                    frac += 1;
                }
            }
            for sess in a.sess_creators() {
                if sess.serv_name() == name {
                    if let Some(n) = sess.sess_count() {
                        fixed += n;
                    }
                    else {
                        frac += 1;
                    }
                }
            }
        }
    }
    (frac, fixed)
}

fn split_eps(pe: &Rc<PE>, d: &config::Domain) -> Result<u32, Error> {
    let mut total_eps = pe.quota()?;
    let mut total_parties = d.apps().len();
    for cfg in d.apps() {
        if let Some(eps) = cfg.eps() {
            total_eps -= eps;
            total_parties -= 1;
        }
    }

    Ok(total_eps.checked_div(total_parties as u32).unwrap_or(0))
}
