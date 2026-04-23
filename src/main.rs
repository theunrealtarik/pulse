use std::{cell::RefCell, rc::Rc, time::Duration};

use sysinfo::{Disks, System};

use crate::modules::{CpuModule, DiskModule, MemModule, NetworkModule, Scheduler};

mod modules;

fn main() {
    let sys = Rc::new(RefCell::new(System::new()));
    let dsk = Rc::new(RefCell::new(Disks::new_with_refreshed_list()));

    let mut scheduler = Scheduler::default();

    scheduler.push(Box::new(NetworkModule::new("net", Duration::from_secs(1))));
    scheduler.push(Box::new(CpuModule::new(
        "cpu",
        Duration::from_secs(15),
        Rc::clone(&sys),
    )));
    scheduler.push(Box::new(MemModule::new(
        "mem",
        Duration::from_secs(15),
        Rc::clone(&sys),
    )));
    scheduler.push(Box::new(DiskModule::new(
        "disk",
        Duration::from_secs(30),
        Rc::clone(&dsk),
    )));

    scheduler.run().unwrap();
}
