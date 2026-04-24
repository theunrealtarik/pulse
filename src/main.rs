mod modules;

use std::rc::Rc;
use std::str::FromStr;
use std::time::Duration;
use std::{cell::RefCell, collections::HashMap};

use clap::Parser;
use strum::IntoEnumIterator;
use sysinfo::{Disks, System};

use modules::{CpuModule, DiskModule, MemModule, ModuleKind, NetworkModule, Scheduler};

use crate::modules::GpuModule;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "List avaialable modules")]
    modules: bool,
    #[arg(long, num_args = 1.., value_delimiter = ' ', value_parser = Args::parse_refresh, help = "Set the refresh rate for each module (module:duration)")]
    refresh: Vec<(ModuleKind, Duration)>,
}

impl Args {
    fn parse_duration(input: &str) -> Result<Duration, String> {
        let input = input.trim().to_lowercase();

        let (num, unit) = input.chars().partition::<String, _>(|c| c.is_ascii_digit());
        let value = num.parse::<u64>().map_err(|err| err.to_string())?;

        let dur = match unit.as_str() {
            "ms" => Duration::from_millis(value),
            "s" => Duration::from_secs(value),
            "m" => Duration::from_mins(value),
            "h" => Duration::from_hours(value),
            "" => return Err("Missing time unit (ms, s, m, h)".into()),
            _ => return Err(format!("Unknown unit {}{}", num, unit)),
        };

        Ok(dur)
    }

    fn parse_refresh(input: &str) -> Result<(ModuleKind, Duration), String> {
        let (module_name, duration) = input
            .trim()
            .split_once(":")
            .ok_or_else(|| "Expected module:duration".to_string())?;

        let module = ModuleKind::from_str(module_name).map_err(|err| err.to_string())?;
        let duration = Self::parse_duration(duration)?;
        Ok((module, duration))
    }
}

fn main() {
    let args = Args::parse();

    let sys = Rc::new(RefCell::new(System::new()));
    let dsk = Rc::new(RefCell::new(Disks::new_with_refreshed_list()));

    let mut scheduler = Scheduler::default();

    if args.modules {
        for m in ModuleKind::iter() {
            println!("{}", m);
        }
        std::process::exit(0);
    }

    let mut intervals: HashMap<ModuleKind, Duration> = ModuleKind::iter()
        .map(|k| (k, Duration::from_secs(1)))
        .collect::<HashMap<_, _>>();

    for (module_kind, duration) in args.refresh {
        intervals.insert(module_kind, duration);
    }

    scheduler.push(Box::new(NetworkModule::new(
        ModuleKind::Net.to_string(),
        intervals.remove(&ModuleKind::Net),
    )));

    scheduler.push(Box::new(CpuModule::new(
        ModuleKind::Cpu.to_string(),
        intervals.remove(&ModuleKind::Cpu),
        Rc::clone(&sys),
    )));

    scheduler.push(Box::new(GpuModule::new(
        ModuleKind::Gpu.to_string(),
        intervals.remove(&ModuleKind::Gpu),
    )));

    scheduler.push(Box::new(MemModule::new(
        ModuleKind::Mem.to_string(),
        intervals.remove(&ModuleKind::Mem),
        Rc::clone(&sys),
    )));

    scheduler.push(Box::new(DiskModule::new(
        ModuleKind::Disk.to_string(),
        intervals.remove(&ModuleKind::Disk),
        Rc::clone(&dsk),
    )));

    scheduler.run().unwrap();
}
