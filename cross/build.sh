#!/bin/sh
set -e

BUILD_BINUTILS=true
BUILD_GCC=true
BUILD_CPP=true
BUILD_GDB=true

MAKE_ARGS="-j"$(nproc)

usage() {
    echo "Usage: $1 (x86_64|arm|riscv) [--rebuild]" >&2
    exit
}

if [ $# -ne 1 ] && [ $# -ne 2 ]; then
    usage $0
fi

ARCH="$1"
if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "arm" ] && [ "$ARCH" != "riscv" ]; then
    usage $0
fi

ROOT=`dirname $(readlink -f $0)`
DIST="$(readlink -f $ROOT/..)/build/cross-$ARCH"
BUILD=$ROOT/$ARCH/build
SRC=$ROOT/$ARCH/src
BUILD_CC=gcc

if [ "$2" = "--rebuild" ] || [ ! -d $DIST ] || [ ! -d $SRC ]; then
    REBUILD=1
else
    REBUILD=0
fi

/bin/echo -e "\e[1mDownloading binutils, gcc, newlib, and gdb...\e[0m"

BINVER=2.32
GCCVER=9.1.0
NEWLVER=3.1.0
GDBVER=8.3

BINARCH=binutils-$BINVER.tar.bz2
GCCARCH=gcc-$GCCVER.tar.gz
NEWLARCH=newlib-$NEWLVER.tar.gz
GDBARCH=gdb-$GDBVER.tar.gz

download() {
    if [ ! -f $2 ]; then
        wget -c $1/$2
    fi
}

download http://ftp.gnu.org/gnu/binutils/ $BINARCH
download http://ftp.gnu.org/gnu/gcc/gcc-$GCCVER/ $GCCARCH
download ftp://sources.redhat.com/pub/newlib/ $NEWLARCH
download https://ftp.gnu.org/gnu/gdb/ $GDBARCH

# setup

export PREFIX=$DIST
if [ "$ARCH" = "arm" ]; then
    export TARGET=arm-none-eabi
elif [ "$ARCH" = "riscv" ]; then
    export TARGET=riscv64-unknown-elf
    BUILD_FLAGS="-g -O2 -march=rv64imafdc -mabi=lp64"
else
    export TARGET=$ARCH-elf-m3
fi

mkdir -p $DIST

# cleanup
if [ $REBUILD -eq 1 ]; then
    if $BUILD_BINUTILS; then
        rm -Rf $BUILD/binutils $SRC/binutils
    fi
    if $BUILD_GCC; then
        rm -Rf $BUILD/gcc $SRC/gcc
    fi
    if $BUILD_CPP; then
        rm -Rf $BUILD/newlib $BUILD/gcc/libstdc++-v3 $SRC/newlib
    fi
    if $BUILD_GDB; then
        rm -Rf $BUILD/gdb $SRC/gdb
    fi
    mkdir -p $SRC
fi
mkdir -p $BUILD/gcc $BUILD/binutils $BUILD/newlib $BUILD/gdb

# binutils
if $BUILD_BINUTILS; then
    if [ $REBUILD -eq 1 ] || [ ! -d $SRC/binutils ]; then
        /bin/echo -e "\e[1mUnpacking binutils...\e[0m"
        cat $BINARCH | bunzip2 | tar -C $SRC -xf -
        mv $SRC/binutils-$BINVER $SRC/binutils
        if [ -f $ARCH/binutils.diff ]; then
            cd $ARCH && patch -p0 < binutils.diff
        fi
    fi
    cd $BUILD/binutils
    if [ $REBUILD -eq 1 ] || [ ! -f $BUILD/binutils/Makefile ]; then
        /bin/echo -e "\e[1mConfiguring binutils...\e[0m"
        CC=$BUILD_CC $SRC/binutils/configure --target=$TARGET --prefix=$PREFIX --disable-nls --disable-werror
        if [ $? -ne 0 ]; then
            exit 1
        fi
    fi
    /bin/echo -e "\e[1mBuilding binutils...\e[0m"
    make $MAKE_ARGS all && make install
    if [ $? -ne 0 ]; then
        exit 1
    fi
    cd $ROOT
fi

if $BUILD_GCC || $BUILD_CPP; then
    # put the include-files of newlib in the system-include-dir to pretend that we have a full libc
    # this is necessary for libgcc and libsupc++. we'll provide our own version of the few required
    # libc-functions later
    rm -Rf $DIST/$TARGET/include $DIST/$TARGET/sys-include
    mkdir -p tmp
    cat $ROOT/$NEWLARCH | gunzip | tar -C tmp -xf - newlib-$NEWLVER/newlib/libc/include
    mv tmp/newlib-$NEWLVER/newlib/libc/include $DIST/$TARGET
    rm -Rf tmp
fi

# gcc
export PATH=$PREFIX/bin:$PATH
if $BUILD_GCC; then
    if [ $REBUILD -eq 1 ] || [ ! -d $SRC/gcc ]; then
        /bin/echo -e "\e[1mUnpacking gcc...\e[0m"
        cat $GCCARCH | gunzip | tar -C $SRC -xf -
        mv $SRC/gcc-$GCCVER $SRC/gcc
        if [ -f $ARCH/gcc.diff ]; then
            cd $ARCH && patch -p0 < gcc.diff
        fi
    fi
    cd $BUILD/gcc
    if [ $REBUILD -eq 1 ] || [ ! -f $BUILD/gcc/Makefile ]; then
        /bin/echo -e "\e[1mConfiguring gcc...\e[0m"
        CC=$BUILD_CC CFLAGS_FOR_TARGET=$BUILD_FLAGS \
            $SRC/gcc/configure --target=$TARGET --prefix=$PREFIX --disable-nls \
              --enable-languages=c,c++ --disable-linker-build-id
        if [ $? -ne 0 ]; then
            exit 1
        fi
    fi
    /bin/echo -e "\e[1mBuilding gcc...\e[0m"
    make $MAKE_ARGS all-gcc && make install-gcc
    if [ $? -ne 0 ]; then
        exit 1
    fi
    ln -sf $DIST/bin/$TARGET-gcc $DIST/bin/$TARGET-cc

    # libgcc (only i586 supports dynamic linking)
    if [ "$ARCH" = "x86_64" ]; then
        # for libgcc, we need crt*S.o and a libc. crt1S.o and crtnS.o need to be working. the others
        # don't need to do something useful, but at least they have to be present.
        TMPCRT0=`mktemp`
        TMPCRT1=`mktemp`
        TMPCRTN=`mktemp`

        REG_SP="%rsp"
        REG_BP="%rbp"

        # we need the function prologs and epilogs. otherwise the INIT entry in the dynamic section
        # won't be created (and the init- and fini-sections don't work).
        echo ".section .init" >> $TMPCRT1
        echo ".global _init" >> $TMPCRT1
        echo "_init:" >> $TMPCRT1
        echo "  push    $REG_BP" >> $TMPCRT1
        echo "  mov     $REG_SP,$REG_BP" >> $TMPCRT1
        echo ".section .fini" >> $TMPCRT1
        echo ".global _fini" >> $TMPCRT1
        echo "_fini:" >> $TMPCRT1
        echo "  push    $REG_BP" >> $TMPCRT1
        echo "  mov     $REG_SP,$REG_BP" >> $TMPCRT1

        echo ".section .init" >> $TMPCRTN
        echo "  leave" >> $TMPCRTN
        echo "  ret" >> $TMPCRTN
        echo ".section .fini" >> $TMPCRTN
        echo "  leave" >> $TMPCRTN
        echo "  ret" >> $TMPCRTN

        # assemble them
        $TARGET-as -o $DIST/$TARGET/lib/crt0S.o $TMPCRT0 || exit 1
        $TARGET-as -o $DIST/$TARGET/lib/crt1S.o $TMPCRT1 || exit 1
        $TARGET-as -o $DIST/$TARGET/lib/crtnS.o $TMPCRTN || exit 1
        # build empty libc
        $TARGET-gcc -nodefaultlibs -nostartfiles -shared -Wl,-shared -Wl,-soname,libc.so \
          -o $DIST/$TARGET/lib/libc.so $DIST/$TARGET/lib/crt0S.o || exit 1
        # cleanup
        rm -f $TMPCRT0 $TMPCRT1 $TMPCRTN
    fi

    # now build libgcc
    /bin/echo -e "\e[1mBuilding libgcc...\e[0m"
    make $MAKE_ARGS all-target-libgcc && make install-target-libgcc
    if [ $? -ne 0 ]; then
        exit 1
    fi
    cd $ROOT

    # copy crt* to basic gcc-stuff
    cp -f $BUILD/gcc/$TARGET/libgcc/crt*.o $DIST/lib/gcc/$TARGET/$GCCVER
fi

# libsupc++
if $BUILD_CPP; then
    # libstdc++
    mkdir -p $BUILD/gcc/libstdc++-v3
    cd $BUILD/gcc/libstdc++-v3
    if [ $REBUILD -eq 1 ] || [ ! -f Makefile ]; then
        /bin/echo -e "\e[1mConfiguring libstdc++...\e[0m"
        # pretend that we're using newlib
        CPP=$TARGET-cpp CFLAGS=$BUILD_FLAGS CXXFLAGS=$BUILD_FLAGS \
            $SRC/gcc/libstdc++-v3/configure --host=$TARGET --prefix=$PREFIX \
            --disable-hosted-libstdcxx --disable-nls --with-newlib
        if [ $? -ne 0 ]; then
            exit 1
        fi
    fi

    /bin/echo -e "\e[1mBuilding libsupc++...\e[0m"
    cd include
    make $MAKE_ARGS && make install-headers
    if [ $? -ne 0 ]; then
        exit 1
    fi

    /bin/echo -e "\e[1mBuilding libstdc++...\e[0m"
    cd ../libsupc++
    make $MAKE_ARGS && make install
    if [ $? -ne 0 ]; then
        exit 1
    fi
    cd $ROOT
fi

# gdb
if $BUILD_GDB; then
    if [ $REBUILD -eq 1 ] || [ ! -d $SRC/gdb ]; then
        /bin/echo -e "\e[1mUnpacking gdb...\e[0m"
        cat $GDBARCH | gunzip | tar -C $SRC -xf -
        mv $SRC/gdb-$GDBVER $SRC/gdb
        if [ -f $ARCH/gdb.diff ]; then
            cd $ARCH && patch -p0 < gdb.diff
        fi
    fi

    cd $BUILD/gdb

    if [ $REBUILD -eq 1 ] || [ ! -f Makefile ]; then
        /bin/echo -e "\e[1mConfiguring gdb...\e[0m"
        $SRC/gdb/configure --target=$TARGET --prefix=$PREFIX --with-python=yes \
          --disable-nls --disable-werror --disable-gas --disable-binutils \
          --disable-ld --disable-gprof \
          --enable-tui
        if [ $? -ne 0 ]; then
            exit 1
        fi
    fi

    /bin/echo -e "\e[1mBuilding gdb...\e[0m"
    make $MAKE_ARGS && make install
    if [ $? -ne 0 ]; then
        exit 1
    fi
fi

# create basic symlinks
rm -Rf $DIST/$TARGET/include
ln -sf $ROOT/../src/include $DIST/$TARGET/include

if [ "$ARCH" = "riscv" ]; then
    cp $DIST/lib/rv64imafdc/lp64d/libsupc++.* $DIST/lib
fi
