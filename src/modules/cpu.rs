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
    name: String,
    interval: Duration,
    last: Option<Instant>,
    sys: SharedSystem,
}

impl CpuModule {
    pub fn new(interval: Option<Duration>, sys: SharedSystem) -> Self {
        Self {
            name: super::ModuleKind::Cpu.to_string(),
            interval: interval.unwrap_or(Duration::from_secs(1)),
            last: None,
            sys,
        }
    }
}

impl super::Module for CpuModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn interval(&self) -> std::time::Duration {
        self.interval
    }

    fn get_last(&self) -> Option<std::time::Instant> {
        self.last
    }

    fn set_last(&mut self, instant: Instant) {
        self.last = Some(instant);
    }

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError> {
        let mut sys = self.sys.borrow_mut();

        sys.refresh_cpu_all();

        let cores = sys
            .cpus()
            .iter()
            .map(|c| Percent::from(c.cpu_usage()))
            .collect::<Vec<Percent>>();

        let cpu_info_raw = fs::read_to_string(PathBuf::from(PROC_CPUINFO))?;

        let brand = parse_from_line!(cpu_info_raw, 4)?;
        let arch = System::cpu_arch();

        let usage = Percent::from(sys.global_cpu_usage());
        let freq = Frequency::from(
            parse_from_line!(cpu_info_raw, 7)?
                .parse::<f32>()
                .map_err(PulseError::from)?,
        );

        let logical = cores.len() as u8;
        let mut physical = 0;
        for line in cpu_info_raw.lines() {
            if line.strip_prefix("core id\t\t:").is_some() {
                let core = line
                    .split(":")
                    .nth(1)
                    .unwrap_or_default()
                    .trim()
                    .parse::<u8>()
                    .map_err(PulseError::from)?;

                if physical <= core {
                    physical = core + 1
                }
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
                    .ok_or_else(|| PulseError::Invalid("entry file name".to_string()))?
                    .to_string_lossy();

                return Ok(file_name.starts_with("temp") | file_name.starts_with("_input"));
            })
            .unwrap_or_default()
        })
        .ok_or_else(|| PulseError::Missing("temp entry".to_string()))?;

        let temp_str = fs::read_to_string(&temp_monitor)?;
        let temp_val = temp_str.trim().parse::<f32>().map_err(PulseError::from)?;
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
