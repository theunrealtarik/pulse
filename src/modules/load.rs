use std::time::{Duration, Instant};

use lib::*;
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Serialize, Deserialize)]
pub struct Load {
    one: Percent,
    five: Percent,
    fifteen: Percent,
}

pub struct LoadModule {
    name: String,
    interval: Duration,
    last: Option<Instant>,
}

impl LoadModule {
    pub fn new(interval: Option<Duration>) -> Self {
        Self {
            name: super::ModuleKind::Load.to_string(),
            interval: interval.unwrap_or(Duration::from_mins(1)),
            last: None,
        }
    }
}

impl super::Module for LoadModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn interval(&self) -> std::time::Duration {
        self.interval
    }

    fn get_last(&self) -> Option<std::time::Instant> {
        self.last
    }

    fn set_last(&mut self, instant: std::time::Instant) {
        self.last = Some(instant)
    }

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError> {
        let load_avg = System::load_average();
        serde_json::to_value(Load {
            one: Percent::from(load_avg.one),
            five: Percent::from(load_avg.five),
            fifteen: Percent::from(load_avg.fifteen),
        })
        .map_err(PulseError::from)
    }
}
