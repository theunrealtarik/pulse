use lib::*;

use libdrm_amdgpu_sys::AMDGPU::GPU_INFO;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

// AMD
use libdrm_amdgpu_sys::AMDGPU;
use libdrm_amdgpu_sys::LibDrmAmdgpu;

use crate::modules::Module;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum GpuKind {
    AMD,
    Nvidia,
    Intel,
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuInfo {
    path: PathBuf,
    vendor: String,
    model: String,
    family: String,
    device_id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuStats {
    vram_total: Bytes,
    vram_used: Bytes,
    vram_usage: Percent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GPU {
    kind: GpuKind,
    info: GpuInfo,
    stats: GpuStats,
}

pub struct GpuModule {
    name: String,
    interval: Duration,
    last: Option<Instant>,
}

impl GpuModule {
    pub fn new(name: String, interval: Option<Duration>) -> Self {
        Self {
            name,
            interval: interval.unwrap_or(Duration::from_secs(1)),
            last: None,
        }
    }
}

impl super::Module for GpuModule {
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
        self.last = Some(instant)
    }

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError> {
        let libdrm_amdgpu = LibDrmAmdgpu::new()
            .map_err(|_| PulseError::Init(String::from("failed to initialize libdrm")))?;
        let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();
        if pci_devs.is_empty() {
            return Err(PulseError::Missing("no amd gpu found".to_string()));
        }

        let drm_paths = pci_devs
            .iter()
            .map(|dev| dev.get_drm_render_path().map_err(PulseError::Io))
            .collect::<Vec<_>>();

        let mut amd_gpus: Vec<GPU> = Vec::new();
        for path in drm_paths {
            let path = path?;
            let fs = File::open(path.clone()).map_err(PulseError::from)?;
            let (device, _, _) = libdrm_amdgpu
                .init_device_handle(fs.as_raw_fd())
                .map_err(|err| PulseError::Init(format!("failed to initialize device: {}", err)))?;

            let info = device
                .device_info()
                .map_err(|err| PulseError::NotFound(format!("device info ({})", err)))?;

            let gpu_stats = device
                .memory_info()
                .and_then(|s| {
                    let vram_total = s.vram.total_heap_size;
                    let vram_used = s.vram.heap_usage;

                    return Ok(GpuStats {
                        vram_total: Bytes::from(vram_total),
                        vram_used: Bytes::from(vram_used),
                        vram_usage: Percent::from(vram_used as f32 / vram_total as f32 * 100.0),
                    });
                })
                .map_err(|err| PulseError::NotFound(format!("memory info ({})", err)))?;

            let gpu_info = GpuInfo {
                path,
                vendor: String::from("AMD"),
                model: info.find_device_name_or_default(),
                family: info.get_family_name().to_string(),
                device_id: info.device_id(),
            };

            amd_gpus.push(GPU {
                kind: GpuKind::AMD,
                info: gpu_info,
                stats: gpu_stats,
            });
        }

        Ok(serde_json::to_value(amd_gpus).map_err(PulseError::from)?)
    }
}

#[test]
fn get_gpu_info() {
    let mut gpu_module = GpuModule::new(String::from("GPU"), None);
    let gpu_data = gpu_module.load().unwrap();
    println!("{:#?}", gpu_data);
}
