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

use crate::error::Error;

use regex::Regex;

use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct Instruction {
    pub addr: usize,
    pub opcode: u32,
    pub binary: String,
    pub symbol: String,
    pub disasm: String,
}

impl fmt::Display for Instruction {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt,
            "Instr {{ addr: {:#x}, opcode: {:#x}, symbol: \x1B[1m{}\x1B[0m {}, disasm: {} }}",
            self.addr, self.opcode, self.binary, self.symbol, self.disasm
        )
    }
}

pub fn parse_instrs<P>(
    cross_prefix: &str,
    instrs: &mut HashMap<usize, Instruction>,
    file: P,
) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let mut cmd = Command::new(format!("{}objdump", cross_prefix))
        .arg("-SC")
        .arg(file.as_ref().as_os_str())
        .stdout(Stdio::piped())
        .spawn()?;

    let symbol_start_re = Regex::new(r"^[0-9a-f]+\s+<(.*)>:").unwrap();
    let instr_re = Regex::new(r"^\s+([0-9a-f]+):\s+([0-9a-f]+)\s+(.*)").unwrap();

    let binary = file
        .as_ref()
        .file_name()
        .ok_or(Error::InvalPath)?
        .to_str()
        .ok_or(Error::InvalPath)?;
    let stdout = cmd.stdout.as_mut().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut symbol = None;

    let mut line = String::new();
    while reader.read_line(&mut line)? != 0 {
        // 0000000010000000 <_start>:
        if let Some(m) = symbol_start_re.captures(&line) {
            let sym_name = m.get(1).unwrap().as_str().trim();
            symbol = Some(sym_name.to_string());
        }
        //     10000010:   00a28663                beq     t0,a0,1000001c <_start+0x1c>
        else if let Some(m) = instr_re.captures(&line) {
            let addr = usize::from_str_radix(m.get(1).unwrap().as_str(), 16)?;
            let opcode = u32::from_str_radix(m.get(2).unwrap().as_str(), 16)?;
            let disasm = m.get(3).unwrap().as_str().trim().to_string();

            instrs.insert(addr, Instruction {
                addr,
                opcode,
                binary: binary.to_string(),
                symbol: symbol.clone().ok_or(Error::ObjdumpMalformed)?,
                disasm,
            });
        }

        line.clear();
    }

    match cmd.wait() {
        Ok(status) if !status.success() => Err(Error::ObjdumpError(status.code().unwrap())),
        Ok(_) => Ok(()),
        Err(e) => Err(Error::from(e)),
    }
}