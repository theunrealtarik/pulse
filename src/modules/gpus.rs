use lib::*;

use libdrm_amdgpu_sys::AMDGPU;
use libdrm_amdgpu_sys::AMDGPU::GPU_INFO;
use libdrm_amdgpu_sys::AMDGPU::SENSOR_INFO::SENSOR_TYPE;
use libdrm_amdgpu_sys::LibDrmAmdgpu;

use serde::{Deserialize, Serialize};

use std::fs;
use std::fs::File;

use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

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
pub struct GpuTemprature {
    edge: Temprature,
    junction: Temprature,
    memory: Temprature,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GpuStats {
    vram_total: Bytes,
    vram_used: Bytes,
    vram_usage: Percent,
    temp: GpuTemprature,
    fan_speed: usize,
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
    pub fn new(interval: Option<Duration>) -> Self {
        Self {
            name: super::ModuleKind::Gpu.to_string(),
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
        let gpus_hw_paths = Monitor::find_many_in_dir(&PathBuf::from(CLASS_HWMON), |path| {
            let name = fs::read_to_string(path.join("name"))?;
            let name = name.trim().to_lowercase();
            Ok(name.contains("amdgpu"))
        })?;

        let libdrm_amdgpu = LibDrmAmdgpu::new()
            .map_err(|_| PulseError::Init(String::from("failed to initialize libdrm")))?;
        let pci_devs = AMDGPU::get_all_amdgpu_pci_bus();
        if pci_devs.is_empty() {
            return Err(PulseError::Missing("no amd gpu found".to_string()));
        }

        let drms = pci_devs
            .iter()
            .map(|dev| {
                (
                    dev.get_drm_render_path().map_err(PulseError::Io),
                    dev.get_hwmon_path()
                        .ok_or_else(|| PulseError::NotFound(format!("device hwmon path"))),
                )
            })
            .collect::<Vec<_>>();

        let mut amd_gpus: Vec<GPU> = Vec::new();
        for (drm_path, hwmon_path) in drms {
            let drm_path = drm_path?;
            let hwmon_path = hwmon_path?;

            let gpu_hw = Monitor::from(hwmon_path);

            macro_rules! parse_entry {
                ($entry_name:literal, $parse_type:ty) => {
                    fs::read_to_string(gpu_hw.path().join($entry_name))?
                        .trim()
                        .parse::<$parse_type>()
                        .map_err(PulseError::from)
                };
            }

            let edg_temp = parse_entry!("temp1_input", f32)?;
            let jnc_temp = parse_entry!("temp2_input", f32)?;
            let mem_temp = parse_entry!("temp3_input", f32)?;

            let fan_speed = parse_entry!("fan1_input", usize)?;

            let fs = File::open(drm_path.clone()).map_err(PulseError::from)?;
            let (device, _, _) = libdrm_amdgpu
                .init_device_handle(fs.as_raw_fd())
                .map_err(|err| PulseError::Init(format!("failed to initialize device: {}", err)))?;

            let info = device
                .device_info()
                .map_err(|err| PulseError::NotFound(format!("device info ({})", err)))?;

            let mem_info = device
                .memory_info()
                .map_err(|err| PulseError::NotFound(format!("memory info ({})", err)))?;

            let gpu_stats = GpuStats {
                vram_total: Bytes::from(mem_info.vram.total_heap_size),
                vram_used: Bytes::from(mem_info.vram.heap_usage),
                vram_usage: Percent::from(
                    mem_info.vram.heap_usage as f64 / mem_info.vram.total_heap_size as f64 * 100.0,
                ),
                temp: GpuTemprature {
                    edge: Temprature::from(edg_temp / 1000.0),
                    junction: Temprature::from(jnc_temp / 1000.0),
                    memory: Temprature::from(mem_temp / 1000.0),
                },
                fan_speed,
            };

            let gpu_info = GpuInfo {
                path: drm_path,
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
fn test_gpu_module() {
    use crate::modules::Module;
    let mut gpu_module = GpuModule::new(None);
    let gpu_data = gpu_module.load().unwrap();
    println!("{:#?}", gpu_data);
}
