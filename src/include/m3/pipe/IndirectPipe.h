/*
 * Copyright (C) 2015, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#pragma once

#include <m3/com/MemGate.h>
#include <m3/session/Pipe.h>
#include <m3/vfs/File.h>

namespace m3 {

class IndirectPipe {
public:
    explicit IndirectPipe(MemGate &mem, size_t memsize);
    ~IndirectPipe();

    /**
     * @return the file descriptor for the reader
     */
    fd_t reader_fd() const {
        return _rdfd;
    }
    /**
     * Closes the read-end
     */
    void close_reader();

    /**
     * @return the file descriptor for the writer
     */
    fd_t writer_fd() const {
        return _wrfd;
    }
    /**
     * Closes the write-end
     */
    void close_writer();

private:
    MemGate &_mem;
    Pipe _pipe;
    fd_t _rdfd;
    fd_t _wrfd;
};

class IndirectPipeFile : public File {
    friend class IndirectPipe;

protected:
    explicit IndirectPipeFile(capsel_t mem, Pipe *pipe)
        : _mem(MemGate::bind(mem)), _pipe(pipe), _destroy(false) {
    }

public:
    explicit IndirectPipeFile(capsel_t mem, capsel_t sess,
        capsel_t metagate, capsel_t rdgate, capsel_t wrgate)
        : _mem(MemGate::bind(mem)), _pipe(new Pipe(sess, metagate, rdgate, wrgate)),
          _destroy(true) {
    }
    ~IndirectPipeFile() {
        if(_destroy)
            delete _pipe;
    }

    virtual Errors::Code stat(FileInfo &) const override {
        // not supported
        return Errors::NOT_SUP;
    }
    virtual ssize_t seek(size_t, int) override {
        // not supported
        return Errors::NOT_SUP;
    }

    virtual size_t serialize_length() override;
    virtual void delegate(VPE &vpe) override;
    virtual void serialize(Marshaller &m) override;
    static File *unserialize(Unmarshaller &um);

protected:
    MemGate _mem;
    Pipe *_pipe;
    bool _destroy;
};

}
