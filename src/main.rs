use std::time::Duration;

use sysinfo::System;

use crate::modules::{CpuModule, Scheduler};

mod modules;

fn main() {
    let mut sys = System::new();
    let mut scheduler = Scheduler::default();
}
