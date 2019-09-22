extern crate log;

mod error;
mod flamegraph;
mod symbols;
mod trace;

use log::{Level, Log, Metadata, Record};
use std::collections::BTreeMap;
use std::env;
use std::process::exit;
use std::str::FromStr;

struct Logger {
    level: Level,
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_string = record.level().to_string();
            let target = if record.target().len() > 0 {
                record.target()
            }
            else {
                record.module_path().unwrap_or_default()
            };

            eprintln!("{:<5} [{}] {}", level_string, target, record.args());
        }
    }

    fn flush(&self) {
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    Trace,
    FlameGraph,
    Snapshot,
}

#[derive(Eq, PartialEq)]
pub enum ISA {
    X86_64,
    ARM,
}

fn usage(prog: &str) -> ! {
    eprintln!(
        "Usage: {} (x86_64|arm) (trace|flamegraph|snapshot <time>) [<binary>...]",
        prog
    );
    exit(1)
}

fn main() -> Result<(), error::Error> {
    let level = Level::from_str(&env::var("RUST_LOG").unwrap_or("error".to_string()))?;
    log::set_boxed_logger(Box::new(Logger { level }))?;
    log::set_max_level(level.to_level_filter());

    let args: Vec<String> = env::args().collect();

    let isa = match args.get(1) {
        Some(isa) if isa == "x86_64" => ISA::X86_64,
        Some(isa) if isa == "arm" => ISA::ARM,
        _ => usage(&args[0]),
    };

    let mode = match args.get(2) {
        Some(mode) if mode == "trace" => Mode::Trace,
        Some(mode) if mode == "flamegraph" => Mode::FlameGraph,
        Some(mode) if mode == "snapshot" => Mode::Snapshot,
        _ => usage(&args[0]),
    };

    let (snapshot_time, bin_start) = if mode == Mode::Snapshot {
        if args.len() < 5 {
            usage(&args[0]);
        }
        let time = args.get(3).expect("Invalid arguments");
        (time.parse::<u64>().expect("Invalid time"), 4)
    }
    else {
        (0, 3)
    };

    let mut syms = BTreeMap::new();
    for f in &args[bin_start..] {
        symbols::parse_symbols(&mut syms, f)?;
    }

    match mode {
        Mode::Trace => trace::generate(&syms),
        Mode::FlameGraph | Mode::Snapshot => flamegraph::generate(mode, snapshot_time, &isa, &syms),
    }
}
