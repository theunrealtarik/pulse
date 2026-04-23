use std::{
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
pub use mem::MemModule;
pub use network::NetworkModule;

pub trait Module {
    fn name(&self) -> &'static str;
    fn interval(&self) -> std::time::Duration;

    fn get_last(&self) -> Option<std::time::Instant>;
    fn set_last(&mut self, instant: Instant);

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError>;
}

#[derive(Default)]
pub struct Scheduler {
    modules: Vec<Box<dyn Module>>,
}

impl Scheduler {
    pub fn push(&mut self, m: Box<dyn Module>) {
        self.modules.push(m);
    }

    pub fn run(&mut self) -> Result<(), PulseError> {
        loop {
            let now = Instant::now();
            let mut output = serde_json::Map::new();

            for module in self.modules.iter_mut() {
                let last = module.get_last();
                if last.is_none() || now.duration_since(last.unwrap_or(now)) >= module.interval() {
                    let json = module.load()?;
                    output.insert(module.name().to_string(), json);
                    module.set_last(now);
                }
            }

            if !output.is_empty() {
                println!("{}", serde_json::Value::Object(output));
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}
