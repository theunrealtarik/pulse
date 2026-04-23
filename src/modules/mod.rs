use std::time::Instant;

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

    fn get_last(&self) -> std::time::Instant;
    fn set_last(&mut self, instant: Instant);

    fn load(&mut self) -> Result<serde_json::Value, lib::PulseError>;

    fn to_json(&mut self) -> Result<String, lib::PulseError> {
        let data = self.load()?;
        serde_json::to_string(&data).map_err(|err| lib::PulseError::Json(err))
    }
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
        for module in self.modules.iter_mut() {
            let json = module.to_json()?;
            println!("{}", json);
        }

        Ok(())
    }
}
