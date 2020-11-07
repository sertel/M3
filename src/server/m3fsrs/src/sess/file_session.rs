use crate::data::INodes;
use crate::internal::{InodeNo, LoadedExtent, LoadedInode, OpenFlags, SeekMode};
use crate::sess::M3FSSession;
use crate::{Extent, FileInfo};

use m3::{
    cap::Selector,
    cell::RefCell,
    col::{String, ToString, Vec},
    com::{GateIStream, SendGate},
    errors::{Code, Error},
    kif::{CapRngDesc, CapType, Perm, INVALID_SEL},
    rc::Rc,
    serialize::Sink,
    server::{CapExchange, SessId},
    session::ServerSession,
    syscalls, tcu,
};

struct Entry {
    sel: Selector,
}

impl Drop for Entry {
    fn drop(&mut self) {
        // On drop, revoke all capabilities
        m3::pes::VPE::cur()
            .revoke(
                m3::kif::CapRngDesc::new(m3::kif::CapType::OBJECT, self.sel, 1),
                false,
            )
            .unwrap();
    }
}

struct CapContainer {
    caps: Vec<Entry>,
}

impl CapContainer {
    pub fn add(&mut self, sel: Selector) {
        self.caps.push(Entry { sel });
    }
}

pub struct FileSession {
    extent: usize,
    lastext: usize,

    extoff: usize,
    lastoff: usize,

    extlen: usize,
    fileoff: usize,

    lastbytes: usize,

    accessed: usize,

    appending: bool,
    pub(crate) append_ext: Option<LoadedExtent>,

    pub(crate) last: Selector,
    epcap: Selector,
    #[allow(dead_code)] // keeps the send gate alive
    sgate: Option<SendGate>,

    oflags: OpenFlags,
    filename: String,
    ino: InodeNo,

    /// the selector this session was created for
    sel: Selector,
    creator: usize,
    /// The id of the parent meta session
    pub(crate) meta_session: SessId,

    capscon: CapContainer,
    #[allow(dead_code)] // keeps the server session alive
    server_session: ServerSession,
}

impl Drop for FileSession {
    fn drop(&mut self) {
        log!(crate::LOG_DEF, "file:close(path={})", self.filename);
    }
}

impl FileSession {
    pub fn new(
        srv_sel: Selector,
        crt: usize,
        meta_rgate: &m3::com::RecvGate,
        file_session_id: SessId,
        meta_session_id: SessId,
        filename: &str,
        flags: OpenFlags,
        ino: InodeNo,
    ) -> Result<Rc<RefCell<Self>>, Error> {
        log!(
            crate::LOG_DEF,
            "Creating File Session (filename={}, inode={}, file_session_id={})",
            filename,
            ino,
            file_session_id
        );

        // The server session for this file
        let sel = if srv_sel == m3::kif::INVALID_SEL {
            srv_sel
        }
        else {
            m3::pes::VPE::cur().alloc_sels(2)
        };

        let server_session =
            ServerSession::new_with_sel(srv_sel, sel, crt, file_session_id as u64, false)?;

        let send_gate = if srv_sel == m3::kif::INVALID_SEL {
            None
        }
        else {
            Some(m3::com::SendGate::new_with(
                m3::com::SGateArgs::new(meta_rgate)
                    // We use the file session id as identifier when the session is called again.
                    // The olf impl used the pointer to this session, but this is not as easy in rust and I guess
                    // kinda unsafe as well
                    .label(file_session_id as tcu::Label)
                    .credits(1)
                    .sel(sel + 1),
            )?)
        };

        let fsess = FileSession {
            extent: 0,
            lastext: 0,
            extoff: 0,
            lastoff: 0,
            extlen: 0,
            fileoff: 0,
            lastbytes: 0,
            accessed: 0,

            appending: false,
            append_ext: None,

            last: m3::kif::INVALID_SEL,
            epcap: m3::kif::INVALID_SEL,
            sgate: send_gate,

            oflags: flags,
            filename: filename.to_string(),
            ino,

            sel,
            creator: crt,
            meta_session: meta_session_id,

            capscon: CapContainer { caps: vec![] },

            server_session,
        };

        let wrapped_fssess = Rc::new(RefCell::new(fsess));

        crate::hdl().files().add_sess(wrapped_fssess.clone());

        Ok(wrapped_fssess)
    }

    pub fn clone(&mut self, _selector: Selector, _data: &mut CapExchange) -> Result<(), Error> {
        log!(crate::LOG_DEF, "file:clone(path={})", self.filename);

        panic!("Clone not yet implemented")
    }

    pub fn get_mem(&mut self, data: &mut CapExchange) -> Result<(), Error> {
        let pop_offset: u32 = data.in_args().pop()?;
        let mut offset = pop_offset as usize;

        log!(
            crate::LOG_DEF,
            "file::get_mem(path={}, offset={})",
            self.filename,
            offset
        );

        let inode = INodes::get(self.ino)?;

        let mut first_off = offset as usize;
        let mut ext_off = 0;
        let mut tmp_extent = 0;
        INodes::seek(
            inode.clone(),
            &mut first_off,
            SeekMode::SET,
            &mut tmp_extent,
            &mut ext_off,
        )?;
        offset = tmp_extent;
        let sel = m3::pes::VPE::cur().alloc_sel();

        let mut extlen = 0;
        let len = INodes::get_extent_mem(
            inode.clone(),
            offset,
            ext_off,
            &mut extlen,
            Perm::from(self.oflags),
            sel,
            true,
            self.accessed,
        )?;

        data.out_caps(m3::kif::CapRngDesc::new(CapType::OBJECT, sel, 1));
        data.out_args().push(&0);
        data.out_args().push(&len);

        log!(crate::LOG_DEF, "file::get_mem -> {}", len);
        self.capscon.add(sel);
        return Ok(());
    }

    pub fn set_ep(&mut self, ep: Selector) {
        self.epcap = ep;
    }

    pub fn ino(&self) -> InodeNo {
        self.ino
    }

    pub fn caps(&self) -> CapRngDesc {
        CapRngDesc::new(CapType::OBJECT, self.sel, 2)
    }

    fn next_in_out(&mut self, is: &mut GateIStream, out: bool) -> Result<(), Error> {
        log!(
            crate::LOG_DEF,
            "file::next_{}(); file[path={}, fileoff={}, ext={}, extoff={}]",
            if out { "out" } else { "in" },
            self.filename,
            self.fileoff,
            self.extent,
            self.extoff
        );

        if (out && !self.oflags.contains(OpenFlags::W))
            || (!out && !self.oflags.contains(OpenFlags::R))
        {
            return Err(Error::new(Code::NoPerm));
        }

        let inode = INodes::get(self.ino)?;
        // in/out implicitly commits the previous in/out request
        if out && self.appending {
            self.commit_append(inode.clone(), self.lastbytes)?;
        }

        if self.accessed < 31 {
            self.accessed += 1;
        }

        let mut sel = m3::pes::VPE::cur().alloc_sel();
        let mut extlen = 0;

        // Do we need to append to the file?
        let len = if out && (self.fileoff as u64 == inode.inode().size) {
            let files = crate::hdl().files();
            let open_file = files.get_file_mut(self.ino).unwrap();

            if open_file.appending() {
                log!(
                    crate::LOG_DEF,
                    "file::next_in_out : append already in progress!"
                );
                return Err(Error::new(Code::Exists));
            }

            // Continue in last extent if there is space
            if (self.extent > 0)
                && (self.fileoff as u64 == inode.inode().size)
                && ((self.fileoff % crate::hdl().superblock().block_size as usize) != 0)
            {
                let mut off = 0;
                self.fileoff = INodes::seek(
                    inode.clone(),
                    &mut off,
                    SeekMode::END,
                    &mut self.extent,
                    &mut self.extoff,
                )?;
            }
            // Exchange extent in which we store the "to append" extent
            let mut e = LoadedExtent::Unstored {
                extent: Rc::new(RefCell::new(Extent {
                    start: 0,
                    length: 0,
                })),
            };

            let len = INodes::req_append(
                inode.clone(),
                self.extent,
                self.extoff,
                &mut extlen,
                sel,
                Perm::from(self.oflags),
                &mut e,
                self.accessed,
            )?;

            self.appending = true;
            self.append_ext = if *e.length() > 0 { Some(e) } else { None };

            open_file.set_appending(true);
            len
        }
        else {
            // get next mem_cap
            let len = INodes::get_extent_mem(
                inode.clone(),
                self.extent,
                self.extoff,
                &mut extlen,
                Perm::from(self.oflags),
                sel,
                out,
                self.accessed,
            );
            match len {
                // if we didn't find the extent, turn that into EOF
                Err(e) if e.code() == Code::NotFound => 0,
                Err(e) => return Err(e),
                Ok(len) => len,
            }
        };

        // The mem cap covers all blocks from `self.extoff` to `self.extoff + len`. Thus, the offset to start
        // is the offset within the first of these blocks
        let mut capoff = self.extoff % crate::hdl().superblock().block_size as usize;
        if len > 0 {
            syscalls::activate(self.epcap, sel, INVALID_SEL, 0)?;

            // Move forward
            self.lastoff = self.extoff;
            self.lastext = self.extent;
            if (self.extoff + len) >= extlen {
                self.extent += 1;
                self.extoff = 0;
            }
            else {
                self.extoff += len - self.extoff % crate::hdl().superblock().block_size as usize;
            }

            self.fileoff += len - capoff;
        }
        else {
            self.lastoff = 0;
            capoff = 0;
            sel = m3::kif::INVALID_SEL;
        }

        self.extlen = extlen;
        self.lastbytes = len - capoff;

        log!(
            crate::LOG_DEF,
            "file::next_{}() -> ({}, {})",
            if out { "out" } else { "in" },
            self.lastoff,
            self.lastbytes
        );

        if crate::hdl().revoke_first() {
            // revoke last mem cap and remember new one
            if self.last != m3::kif::INVALID_SEL {
                m3::pes::VPE::cur()
                    .revoke(
                        m3::kif::CapRngDesc::new(m3::kif::CapType::OBJECT, self.last, 1),
                        false,
                    )
                    .unwrap();
            }
            self.last = sel;
            reply_vmsg!(is, 0 as u32, capoff, self.lastbytes)
        }
        else {
            reply_vmsg!(is, 0 as u32, capoff, self.lastbytes)?;
            if self.last != m3::kif::INVALID_SEL {
                m3::pes::VPE::cur()
                    .revoke(
                        m3::kif::CapRngDesc::new(m3::kif::CapType::OBJECT, self.last, 1),
                        false,
                    )
                    .unwrap();
            }
            self.last = sel;
            Ok(())
        }
    }

    fn commit_append(&mut self, inode: LoadedInode, submit: usize) -> Result<(), Error> {
        assert!(submit > 0, "commit_append() submit must be > 0");
        log!(
            crate::LOG_DEF,
            "file::commit_append(inode={}, submit={})",
            { inode.inode().inode },
            submit
        );
        if !self.appending {
            return Ok(());
        }

        // adjust file position
        self.fileoff -= self.lastbytes - submit;

        // add new extent?
        if let Some(ref append_ext) = self.append_ext {
            let blocksize = crate::hdl().superblock().block_size as usize;
            let blocks = (submit + blocksize - 1) / blocksize;
            let old_len = *append_ext.length();
            // append extent to file
            *append_ext.length_mut() = blocks as u32;
            let mut new_ext = false;
            INodes::append_extent(inode.clone(), &append_ext, &mut new_ext)?;

            // free superfluous blocks
            if old_len as usize > blocks {
                crate::hdl().blocks().free(
                    *append_ext.start() as usize + blocks,
                    old_len as usize - blocks,
                )?;
            }

            self.extlen = blocks * blocksize;
            // have we appended the new extent to the previous extent?
            if !new_ext {
                self.extent -= 1;
            }

            self.lastoff = 0;
            self.append_ext = None;
        }

        // we are at the end of the extent now, so move forward if not already done
        if self.extoff >= self.extlen {
            self.extent += 1;
            self.extoff = 0;
        }

        // change size
        inode.inode_mut().size += submit as u64;
        INodes::mark_dirty(inode.inode().inode);

        // stop appending
        let files = crate::hdl().files();
        let ofile = files.get_file_mut(self.ino).unwrap();
        assert!(ofile.appending(), "ofile should be in append mode!");
        ofile.set_appending(false);

        self.append_ext = None;
        self.appending = false;

        Ok(())
    }

    #[allow(dead_code)] // TODO currently unused since there seams to be no SYNC Op in rust
    fn sync(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        crate::hdl().flush_buffer()?;
        reply_vmsg!(stream, 0 as u32)
    }
}

impl M3FSSession for FileSession {
    fn creator(&self) -> usize {
        self.creator
    }

    fn next_in(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        self.next_in_out(stream, false)
    }

    fn next_out(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        self.next_in_out(stream, true)
    }

    fn commit(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        let nbytes: usize = stream.pop()?;

        log!(
            crate::LOG_DEF,
            "file::commit(nbytes={}); file[path={}, fileoff={}, ext={}, extoff={}]",
            nbytes,
            self.filename,
            self.fileoff,
            self.extent,
            self.extoff
        );

        if (nbytes == 0) || (nbytes > self.lastbytes) {
            return Err(Error::new(Code::InvArgs));
        }

        let inode = INodes::get(self.ino)?;

        let res = if self.appending {
            self.commit_append(inode.clone(), nbytes)
        }
        else {
            if (self.extent > self.lastext) && ((self.lastoff + nbytes) > self.extlen) {
                self.extent -= 1;
            }

            if nbytes < self.lastbytes {
                self.extoff = self.lastoff + nbytes;
            }
            Ok(())
        };

        self.lastbytes = 0;
        if let Err(e) = res {
            Err(e)
        }
        else {
            reply_vmsg!(stream, 0 as u32)
        }
    }

    fn seek(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        let mut off: usize = stream.pop()?;
        let whence = SeekMode::from(stream.pop::<u32>()?);

        log!(
            crate::LOG_DEF,
            "file::seek(path={}, off={}, whence={})",
            self.filename,
            off,
            whence
        );
        if whence == SeekMode::CUR {
            return Err(Error::new(Code::InvArgs));
        }

        let inode = INodes::get(self.ino)?;

        let pos = INodes::seek(
            inode.clone(),
            &mut off,
            whence,
            &mut self.extent,
            &mut self.extoff,
        )?;
        self.fileoff = pos + off;
        reply_vmsg!(stream, 0, pos, off)
    }

    fn fstat(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        log!(crate::LOG_DEF, "file::fstat(path={})", self.filename);
        let inode = INodes::get(self.ino)?;

        let mut info = FileInfo::default();
        INodes::stat(inode.clone(), &mut info);

        reply_vmsg!(stream, 0, info)
    }

    fn stat(&mut self, stream: &mut GateIStream) -> Result<(), Error> {
        self.fstat(stream)
    }

    fn mkdir(&mut self, _stream: &mut GateIStream) -> Result<(), Error> {
        Err(Error::new(Code::NotSup))
    }

    fn rmdir(&mut self, _stream: &mut GateIStream) -> Result<(), Error> {
        Err(Error::new(Code::NotSup))
    }

    fn link(&mut self, _stream: &mut GateIStream) -> Result<(), Error> {
        Err(Error::new(Code::NotSup))
    }

    fn unlink(&mut self, _stream: &mut GateIStream) -> Result<(), Error> {
        Err(Error::new(Code::NotSup))
    }
}
