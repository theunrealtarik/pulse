use std::{
    collections::HashMap,
    thread,
    time::{Duration, Instant},
};

mod cpu;
mod disk;
mod gpus;
mod mem;
mod network;

use lib::PulseError;

pub use cpu::CpuModule;
pub use disk::DiskModule;
pub use gpus::GpuModule;
pub use mem::MemModule;
pub use network::NetworkModule;

#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    PartialEq,
    Eq,
    strum::Display,
    strum::EnumString,
    strum::EnumIter,
    clap::ValueEnum,
)]
#[strum(serialize_all = "lowercase")]
pub enum ModuleKind {
    Cpu,
    Gpu,
    Mem,
    Disk,
    Net,
}

pub trait Module {
    fn name(&self) -> &str;
    fn interval(&self) -> std::time::Duration;

    fn get_last(&self) -> Option<std::time::Instant>;
    fn set_last(&mut self, instant: Instant);

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError>;
}

#[derive(Default)]
pub struct Scheduler {
    modules: Vec<Box<dyn Module>>,
    object: HashMap<String, serde_json::Value>,
}

impl Scheduler {
    pub fn push(&mut self, m: Box<dyn Module>) {
        self.modules.push(m);
    }

    pub fn run(&mut self) {
        loop {
            let now = Instant::now();
            for module in self.modules.iter_mut() {
                let last = module.get_last();
                if last.is_none() || now.duration_since(last.unwrap_or(now)) >= module.interval() {
                    let json = match module.load() {
                        Ok(data) => data,
                        Err(_) => {
                            continue;
                        }
                    };

                    self.object.insert(module.name().to_string(), json);
                    module.set_last(now);
                }
            }

            if let Ok(output) = serde_json::to_value(&self.object)
                && !self.object.is_empty()
            {
                println!("{}", output);
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}
