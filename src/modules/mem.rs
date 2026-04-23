use std::time::{Duration, Instant};

use lib::*;
use serde::{Deserialize, Serialize};
use sysinfo::System;

macro_rules! mem_object {
    ($name:ident) => {
        #[derive(Debug, Serialize, Deserialize)]
        pub struct $name {
            pub total: Bytes,
            pub used: Bytes,
            pub percent: Percent,
        }
    };
}

mem_object!(RAM);
mem_object!(Swap);

#[derive(Debug, Serialize, Deserialize)]
pub struct Mem {
    pub ram: RAM,
    pub swp: Swap,
}

pub struct MemModule {
    name: &'static str,
    interval: Duration,
    last: Instant,
    sys: &'static mut System,
}

impl MemModule {
    pub fn new(name: &'static str, interval: Duration, sys: &'static mut System) -> Self {
        Self {
            name,
            interval,
            last: Instant::now(),
            sys,
        }
    }
}

impl super::Module for MemModule {
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
        self.sys.refresh_memory();

        let ram_total = self.sys.total_memory();
        let ram_used = self.sys.used_memory();
        let ram_percent = if ram_total == 0 {
            0.0
        } else {
            (ram_used as f32 / ram_total as f32) * 100.0
        };

        let ram = RAM {
            total: Bytes::from(ram_total),
            used: Bytes::from(ram_used),
            percent: Percent::from(ram_percent),
        };

        let swap_total = self.sys.total_swap();
        let swap_used = self.sys.used_swap();
        let swap_percent = if swap_total == 0 {
            0.0
        } else {
            (swap_used as f32 / swap_total as f32) * 100.0
        };

        let swp = Swap {
            total: Bytes::from(swap_total),
            used: Bytes::from(swap_used),
            percent: Percent::from(swap_percent),
        };

        Ok(serde_json::to_value(Mem { ram, swp }).map_err(|err| PulseError::Json(err))?)
    }
}
