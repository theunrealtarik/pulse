use std::path::PathBuf;
use std::time::Duration;
use std::{fs, time::Instant};

use lib::*;
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Serialize, Deserialize)]
pub struct CPU {
    brand: String,
    arch: String,
    usage: Percent,
    freq: Frequency,
    cores: Vec<Percent>,
    logical: u8,
    physical: u8,
    temp: Temprature,
}

pub struct CpuModule {
    name: &'static str,
    interval: Duration,
    last: Instant,
    sys: &'static mut System,
}

impl CpuModule {
    pub fn new(name: &'static str, interval: Duration, sys: &'static mut System) -> Self {
        Self {
            name,
            interval,
            last: Instant::now(),
            sys,
        }
    }
}

impl super::Module for CpuModule {
    fn name(&self) -> &'static str {
        self.name
    }

    fn interval(&self) -> std::time::Duration {
        self.interval
    }

    fn get_last(&self) -> std::time::Instant {
        self.last
    }

    fn set_last(&mut self, instant: Instant) {
        self.last = instant;
    }

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError> {
        self.sys.refresh_cpu_all();

        let cores = self
            .sys
            .cpus()
            .iter()
            .map(|c| Percent::from(c.cpu_usage()))
            .collect::<Vec<Percent>>();

        let cpu_info_raw = fs::read_to_string(PathBuf::from(PROC_CPUINFO))?;

        let brand = parse_from_line!(cpu_info_raw, 4)?;
        let arch = System::cpu_arch();

        let usage = Percent::from(self.sys.global_cpu_usage());
        let freq = Frequency::from(
            parse_from_line!(cpu_info_raw, 7)?
                .parse::<f32>()
                .map_err(|_| PulseError::Parse("cpu freq"))?,
        );

        let logical = cores.len() as u8;
        let mut physical = 0;
        for line in cpu_info_raw.lines() {
            if line.strip_prefix("core id\t:").is_some() {
                physical += 1;
            }
        }

        let temp_monitor = Monitor::new(|path| {
            let name = fs::read_to_string(path.join("name"))?;
            return Ok(
                name.to_lowercase().contains("k10temp") || name.to_lowercase().contains("cpu")
            );
        })
        .map(|m| {
            m.entry(|path| {
                let file_name = path
                    .file_name()
                    .ok_or_else(|| PulseError::Invalid("entry file name"))?
                    .to_string_lossy();

                return Ok(file_name.starts_with("temp") | file_name.starts_with("_input"));
            })
            .unwrap_or_default()
        })
        .ok_or_else(|| PulseError::Missing("temp entry"))?;

        let temp_str = fs::read_to_string(&temp_monitor)?;
        let temp_val = temp_str
            .trim()
            .parse::<f32>()
            .map_err(|_| PulseError::Parse("temp value"))?;
        let temp = Temprature::from(temp_val / 1000.0);

        Ok(serde_json::to_value(CPU {
            brand,
            arch,
            usage,
            freq,
            cores,
            logical,
            physical,
            temp,
        })
        .map_err(|err| PulseError::Json(err))?)
    }
}
