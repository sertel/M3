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

#include <m3/com/GateStream.h>
#include <m3/session/M3FS.h>
#include <m3/vfs/GenericFile.h>
#include <m3/vfs/VFS.h>

namespace m3 {

Reference<File> M3FS::open(const char *path, int perms) {
    if(!(perms & FILE_NEWSESS)) {
        size_t ep_idx = get_ep();

        GateIStream reply = send_receive_vmsg(_gate, OPEN_PRIV, path, perms, _eps[ep_idx].id);
        reply.pull_result();
        ssize_t file_id;
        reply >> file_id;

        _eps[ep_idx].file = file_id;
        return Reference<File>(new GenericFile(perms, sel(), id(), static_cast<size_t>(file_id),
                                               _eps[ep_idx].ep->id(), &_gate));
    }
    else {
        KIF::ExchangeArgs args;
        ExchangeOStream os(args);
        os << OPEN << perms << String(path);
        args.bytes = os.total();
        KIF::CapRngDesc crd = obtain(2, &args);

        return Reference<File>(new GenericFile(perms, crd.start()));
    }
}

void M3FS::close(size_t file_id) {
    for(auto &ep: _eps) {
        if(ep.file == static_cast<ssize_t>(file_id)) {
            ep.file = -1;;
            break;
        }
    }
}

size_t M3FS::get_ep() {
    for(size_t i = 0; i < _eps.size(); ++i) {
        if(_eps[i].file == -1)
            return i;
    }

    auto ep = Activity::self().epmng().acquire();
    size_t id = delegate_ep(ep->sel());

    _eps.push_back(CachedEP(id, ep));
    return _eps.size() - 1;
}

Errors::Code M3FS::try_stat(const char *path, FileInfo &info) noexcept {
    GateIStream reply = send_receive_vmsg(_gate, STAT, path);
    Errors::Code res;
    reply >> res;
    if(res != Errors::NONE)
        return res;
    reply >> info;
    return Errors::NONE;
}

Errors::Code M3FS::try_mkdir(const char *path, mode_t mode) {
    GateIStream reply = send_receive_vmsg(_gate, MKDIR, path, mode);
    Errors::Code res;
    reply >> res;
    return res;
}

Errors::Code M3FS::try_rmdir(const char *path) {
    GateIStream reply = send_receive_vmsg(_gate, RMDIR, path);
    Errors::Code res;
    reply >> res;
    return res;
}

Errors::Code M3FS::try_link(const char *oldpath, const char *newpath) {
    GateIStream reply = send_receive_vmsg(_gate, LINK, oldpath, newpath);
    Errors::Code res;
    reply >> res;
    return res;
}

Errors::Code M3FS::try_unlink(const char *path) {
    GateIStream reply = send_receive_vmsg(_gate, UNLINK, path);
    Errors::Code res;
    reply >> res;
    return res;
}

Errors::Code M3FS::try_rename(const char *oldpath, const char *newpath) {
    GateIStream reply = send_receive_vmsg(_gate, RENAME, oldpath, newpath);
    Errors::Code res;
    reply >> res;
    return res;
}

size_t M3FS::delegate_ep(capsel_t sel) {
    KIF::ExchangeArgs args;
    ExchangeOStream os(args);
    os << FileSystem::DEL_EP;
    args.bytes = os.total();

    ClientSession::delegate(KIF::CapRngDesc(KIF::CapRngDesc::OBJ, sel, 1), &args);

    ExchangeIStream is(args);
    size_t id;
    is >> id;
    return id;
}

void M3FS::delegate(Activity &act) {
    act.delegate_obj(sel());
    // TODO what if it fails?
    get_sgate(act);
}

void M3FS::serialize(Marshaller &m) {
    m << sel() << id();
}

FileSystem *M3FS::unserialize(Unmarshaller &um) {
    capsel_t sel;
    size_t id;
    um >> sel >> id;
    return new M3FS(id, sel);
}

}
