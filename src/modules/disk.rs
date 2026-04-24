use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashMap, time::Instant};

use lib::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    mount_point: PathBuf,
    total: Bytes,
    free: Bytes,
    used: Bytes,
    usage: Percent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Disk {
    device: String,
    partitions: HashMap<PathBuf, Partition>,
}

pub struct DiskModule {
    name: String,
    interval: Duration,
    last: Option<Instant>,
    disks: SharedDisks,
}

impl DiskModule {
    pub fn new(interval: Option<Duration>, disks: SharedDisks) -> Self {
        Self {
            name: super::ModuleKind::Disk.to_string(),
            interval: interval.unwrap_or(Duration::from_secs(1)),
            last: None,
            disks,
        }
    }
}

impl super::Module for DiskModule {
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
        let disks = self.disks.borrow();
        let mut disks_data: HashMap<String, Disk> = HashMap::new();

        for disk in disks.iter() {
            let total = disk.total_space();
            let free = disk.available_space();
            let used = total.saturating_sub(free);
            let usage = if total == 0 {
                0.0
            } else {
                (used as f64 / total as f64) * 100.0
            };

            let device_name = disk.name().to_string_lossy().to_string();
            let mount_point = PathBuf::from(disk.mount_point());

            let partition = Partition {
                mount_point: mount_point.clone(),
                total: Bytes::from(total),
                free: Bytes::from(free),
                used: Bytes::from(used),
                usage: Percent::new(usage as f32),
            };

            disks_data
                .entry(device_name.clone())
                .and_modify(|disk| {
                    disk.partitions
                        .insert(mount_point.clone(), partition.clone());
                })
                .or_insert(Disk {
                    device: device_name,
                    partitions: HashMap::from([(mount_point, partition)]),
                });
        }

        Ok(serde_json::to_value(disks_data).map_err(|err| PulseError::Json(err))?)
    }
}
