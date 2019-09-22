#!/bin/bash

usage() {
    echo "Usage: $1 <script> [--debug=<prog>]" 1>&2
    exit 1
}

if [ "$1" = "-h" ] || [ "$1" = "--help" ] || [ "$1" = "-?" ]; then
    usage $0
fi

build=build/$M3_TARGET-$M3_ISA-$M3_BUILD
bindir=$build/bin

if [ $# -lt 1 ]; then
    usage $0
fi
script=$1
shift

debug=""
for p in $@; do
    case $p in
        --debug=*)
            debug=${p#--debug=}
            ;;
    esac
done

if [ "$M3_FS" = "" ]; then
    M3_FS="default.img"
fi
export M3_FS

if [ "$M3_HDD" = "" ]; then
    M3_HDD="disk.img"
fi
export M3_HDD

error() {
    echo $1 1>&2
    exit 1
}

generate_lines() {
    # workaround for bash: it executes the while-loop in a subprocess
    $1 | (
        while read line || [ -n "$line" ]; do
            echo $line
        done
    )
}

generate_kargs() {
    c=0
    generate_lines $1 | ( while read line; do
            i=0
            for a in $line; do
                if [ $c -eq 1 ]; then
                    if [ $i -eq 0 ]; then
                        echo -n $a
                    else
                        echo -n ",$a"
                    fi
                elif [ $c -gt 1 ]; then
                    if [ $i -eq 0 ]; then
                        echo -n ",--,$a"
                    else
                        echo -n ",$a"
                    fi
                fi
                i=$((i + 1))
            done
            c=$((c + 1))
        done
    )
}

remove_kernel_args() {
    for word in $1; do
        case "$word" in
            daemon)
                ;;
            requires=*)
                ;;
            core=*)
                ;;

            *)
                echo -n "$word "
                ;;
        esac
    done
}

build_params_host() {
    c=0
    generate_lines $1 | while read line; do
        if [ $c -eq 0 ]; then
            echo -n "$bindir/$line "
        else
            echo -n "$bindir/$line -- "
        fi
        c=$((c + 1))
    done
}

build_params_gem5() {
    kargs=`generate_kargs $1 | tr ',' ' '`

    if [ "$M3_GEM5_DBG" = "" ]; then
        M3_GEM5_DBG="Dtu"
    fi
    if [ "$M3_GEM5_CPU" = "" ]; then
        if [ "$debug" != "" ]; then
            M3_GEM5_CPU="TimingSimpleCPU"
        else
            M3_GEM5_CPU="DerivO3CPU"
        fi
    fi

    M3_CORES=${M3_CORES:-16}

    c=0
    cmd=`generate_lines $1 | ( while read line; do
            if [ $c -eq 0 ]; then
                echo -n $bindir/$line -- $kargs,
            else
                echo -n $bindir/pemux,
            fi
            c=$((c + 1))
        done

        while [ $c -lt $M3_CORES ]; do
            echo -n $bindir/pemux,
            c=$((c + 1))
        done
    )`

    if [[ $cmd == *disk* ]]; then
        ./src/tools/disk.py create $build/$M3_HDD $build/$M3_FS
    fi

    M3_GEM5_CPUFREQ=${M3_GEM5_CPUFREQ:-1GHz}
    M3_GEM5_MEMFREQ=${M3_GEM5_MEMFREQ:-333MHz}
    M3_GEM5_OUT=${M3_GEM5_OUT:-run}
    M3_GEM5_CFG=${M3_GEM5_CFG:-config/default.py}
    export M3_GEM5_PES=$M3_CORES
    export M3_GEM5_FS=$build/$M3_FS
    export M3_GEM5_IDE_DRIVE=$build/$M3_HDD

    params=`mktemp`
    trap "rm -f $params" EXIT ERR INT TERM

    echo -n "--outdir=$M3_GEM5_OUT --debug-file=gem5.log --debug-flags=$M3_GEM5_DBG" >> $params
    if [ "$M3_GEM5_PAUSE" != "" ]; then
        echo -n " --listener-mode=on" >> $params
    fi
    if [ "$M3_GEM5_DBGSTART" != "" ]; then
        echo -n " --debug-start=$M3_GEM5_DBGSTART" >> $params
    fi
    echo -n " $M3_GEM5_CFG --cpu-type $M3_GEM5_CPU --isa $M3_ISA --cmd \"$cmd\"" >> $params
    echo -n " --cpu-clock=$M3_GEM5_CPUFREQ --sys-clock=$M3_GEM5_MEMFREQ" >> $params
    if [ "$M3_GEM5_PAUSE" != "" ]; then
        echo -n " --pausepe=$M3_GEM5_PAUSE" >> $params
    fi
    if [ "$M3_GEM5_CC" != "" ]; then
        echo -n " --coherent" >> $params
    fi

    if [ "$M3_ISA" = "x86_64" ]; then
        gem5build="X86"
    else
        gem5build="ARM"
    fi

    export M5_PATH=$build
    if [ "$DBG_GEM5" != "" ]; then
        tmp=`mktemp`
        trap "rm -f $tmp" EXIT ERR INT TERM
        echo "b main" >> $tmp
        echo -n "run " >> $tmp
        cat $params >> $tmp
        echo >> $tmp
        gdb --tui hw/gem5/build/$gem5build/gem5.debug --command=$tmp
    else
        xargs -a $params hw/gem5/build/$gem5build/gem5.opt
    fi
}

if [ "$M3_TARGET" = "host" ]; then
    params=`build_params_host $script`

    if [[ $params == *disk* ]]; then
        ./src/tools/disk.py create $build/$M3_HDD $build/$M3_FS
    fi

    if [ "$M3_VALGRIND" != "" ]; then
        valgrind $M3_VALGRIND $params
    else
        setarch `uname -m` -R $params
    fi
elif [ "$M3_TARGET" = "gem5" ]; then
    build_params_gem5 $script
else
    echo "Unknown target '$M3_TARGET'"
fi

if [ -f $build/$M3_FS.out ]; then
    $build/src/tools/m3fsck/m3fsck $build/$M3_FS.out && echo "FS image '$build/$M3_FS.out' is valid"
fi
