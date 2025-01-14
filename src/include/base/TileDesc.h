/*
 * Copyright (C) 2016-2018 Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#pragma once

#include <base/Common.h>
#include <base/Config.h>

#include <utility>

namespace m3 {

/**
 * The different types of tiles
 */
enum class TileType {
    // Compute tile with internal memory
    COMP_IMEM = 0,
    // Compute tile with cache and external memory
    COMP_EMEM = 1,
    // memory tile
    MEM = 2,
};

/**
 * The different ISAs
 */
enum class TileISA {
    NONE = 0,
    X86 = 1,
    ARM = 2,
    RISCV = 3,
    ACCEL_INDIR = 4,
    ACCEL_COPY = 5,
    ACCEL_ROT13 = 6,
    IDE_DEV = 7,
    NIC_DEV = 8,
    SERIAL_DEV = 9,
};

enum TileAttr {
    BOOM = 0x1,
    ROCKET = 0x2,
    NIC = 0x4,
    KECACC = 0x8,
};

/**
 * Describes a tile
 */
struct TileDesc {
    typedef uint64_t value_t;

    /**
     * Default constructor
     */
    explicit TileDesc() : _value() {
    }
    /**
     * Creates a tile description from the given descriptor word
     */
    explicit TileDesc(value_t value) : _value(value) {
    }
    /**
     * Creates a tile description of given type, ISA and memory size
     */
    explicit TileDesc(TileType type, TileISA isa, size_t memsize = 0, uint attr = 0)
        : _value(static_cast<value_t>(type) | (static_cast<value_t>(isa) << 3) |
                 (static_cast<value_t>(attr) << 7) | memsize) {
    }

    /**
     * @return the raw descriptor word
     */
    value_t value() const {
        return _value;
    }

    /**
     * @return the type of tile
     */
    TileType type() const {
        return static_cast<TileType>(_value & 0x7);
    }
    /**
     * @return the isa of the tile
     */
    TileISA isa() const {
        return static_cast<TileISA>((_value >> 3) & 0xF);
    }
    /**
     * @return the attributes of the tile
     */
    uint attr() const {
        return (_value >> 7) & 0xF;
    }
    /**
     * @return if the tile has a core that is programmable
     */
    bool is_programmable() const {
        return isa() != TileISA::NONE && isa() < TileISA::ACCEL_INDIR;
    }
    /**
     * @return if the tile supports multiple contexts
     */
    bool is_device() const {
        return isa() == TileISA::NIC_DEV || isa() == TileISA::IDE_DEV ||
               isa() == TileISA::SERIAL_DEV;
    }

    /**
     * @return if the tile supports activities
     */
    bool supports_activities() const {
        return type() != TileType::MEM;
    }
    /**
     * @return if the tile supports the context switching protocol
     */
    bool supports_tilemux() const {
        return supports_activities() && !is_device();
    }

    /**
     * @return the memory size (for type() == COMP_IMEM | MEM)
     */
    size_t mem_size() const {
        return _value & ~static_cast<value_t>(0xFFF);
    }
    /**
     * @return true if the tile has internal memory
     */
    bool has_memory() const {
        return type() == TileType::COMP_IMEM || type() == TileType::MEM;
    }
    /**
     * @return true if the tile has a cache, i.e., external memory
     */
    bool has_cache() const {
        return type() == TileType::COMP_EMEM;
    }
    /**
     * @return true if the tile has virtual memory support of some form
     */
    bool has_virtmem() const {
        return has_cache();
    }

    /**
     * @return the starting address and size of the standard receive buffer space
     */
    std::pair<uintptr_t, size_t> rbuf_std_space() const {
        return std::make_pair(rbuf_base(), RBUF_STD_SIZE);
    }

    /**
     * @return the starting address and size of the receive buffer space
     */
    std::pair<uintptr_t, size_t> rbuf_space() const {
        size_t size = has_virtmem() ? RBUF_SIZE : RBUF_SIZE_SPM;
        return std::make_pair(rbuf_base() + RBUF_STD_SIZE, size);
    }

    /**
     * @return the highest address of the stack
     */
    uintptr_t stack_top() const {
        auto space = stack_space();
        return space.first + space.second;
    }

    /**
     * @return the starting address and size of the stack
     */
    std::pair<uintptr_t, size_t> stack_space() const {
        return std::make_pair(rbuf_base() - STACK_SIZE, STACK_SIZE);
    }

private:
    uintptr_t rbuf_base() const {
#if defined(__host__)
        return RBUF_STD_ADDR;
#else
        if(has_virtmem())
            return RBUF_STD_ADDR;
        size_t rbufs = TILEMUX_RBUF_SIZE + RBUF_SIZE_SPM + RBUF_STD_SIZE;
        return MEM_OFFSET + mem_size() - rbufs;
#endif
    }

    value_t _value;
} PACKED;

}
