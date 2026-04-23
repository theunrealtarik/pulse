use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize, ser::SerializeStruct};
use strum::IntoEnumIterator;

pub const CLASS_NET: &str = "/sys/class/net";
pub const CLASS_HWMON: &str = "/sys/class/hwmon";
pub const PROC_NET: &str = "/proc/net";
pub const PROC_CPUINFO: &str = "/proc/cpuinfo";

// Helpers

#[macro_export]
macro_rules! parse_from_line {
    ($text:expr, $line_idx:expr) => {
        $text
            .lines()
            .nth($line_idx)
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .ok_or(lib::PulseError::Parse("iw output"))
    };
}

#[derive(Debug, Clone)]
pub struct Monitor(PathBuf);

impl Monitor {
    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn new<F>(filter: F) -> Option<Self>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        let hwmons = fs::read_dir(PathBuf::from(CLASS_HWMON)).ok()?;

        for hwmon in hwmons {
            let entry_path = hwmon.ok()?.path();

            if let Ok(r) = filter(&entry_path)
                && r
            {
                return Some(Monitor(entry_path));
            }

            continue;
        }

        None
    }

    pub fn entry<F>(&self, filter: F) -> Option<PathBuf>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        let monitor_dir = fs::read_dir(&self.0).ok()?;

        for entry in monitor_dir {
            let entry = entry.ok()?;
            let entry_path = entry.path();

            if let Ok(r) = filter(&entry_path)
                && r
            {
                return Some(entry_path);
            }

            continue;
        }

        None
    }
}

// Error

#[derive(Debug)]
pub enum PulseError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Parse(&'static str),
    Missing(&'static str),
    Invalid(&'static str),
    NotFound(String),
}

impl From<std::io::Error> for PulseError {
    fn from(e: std::io::Error) -> Self {
        PulseError::Io(e)
    }
}

impl From<serde_json::Error> for PulseError {
    fn from(e: serde_json::Error) -> Self {
        PulseError::Json(e)
    }
}

// Data

#[derive(Debug, Clone, Copy, strum::Display, strum::EnumIter)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
    PB,
}

#[derive(Debug)]
pub struct Formatted {
    pub value: f64,
    pub unit: Unit,
}

impl std::fmt::Display for Formatted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}{}", self.value, self.unit)
    }
}

/// Bytes
#[derive(
    Debug, Default, Clone, Copy, Deserialize, derive_more::Add, derive_more::Mul, derive_more::From,
)]
pub struct Bytes(u64);

impl Bytes {
    pub fn format(&self) -> Formatted {
        let units = Unit::iter().collect::<Vec<_>>();

        let step = 1024f64;
        let mut value = self.0 as f64;

        for unit in units {
            if value < step {
                return Formatted { value, unit };
            }

            value /= step;
        }

        Formatted {
            value,
            unit: Unit::PB,
        }
    }
}

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Bytes", 2)?;

        state.serialize_field("raw", &self.0)?;
        state.serialize_field("formatted", self)?;

        state.end()
    }
}

/// Percent
#[derive(
    Debug, Clone, Copy, Deserialize, derive_more::Add, derive_more::Mul, derive_more::From,
)]
pub struct Percent(f32);

impl Percent {
    pub fn new(v: f32) -> Self {
        Self(v.clamp(0.0, 100.0))
    }
}

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}%", self.0)
    }
}

impl Serialize for Percent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Percent", 2)?;

        state.serialize_field("raw", &self.0)?;
        state.serialize_field("formatted", &format!("{}", self))?;

        state.end()
    }
}

/// Temprature
#[derive(
    Debug, Clone, Copy, Deserialize, derive_more::Add, derive_more::Mul, derive_more::From,
)]
pub struct Temprature(f32);

impl std::fmt::Display for Temprature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

impl Serialize for Temprature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Temprature", 2)?;

        state.serialize_field("raw", &self.0)?;
        state.serialize_field("formatted", &format!("{}", self))?;

        state.end()
    }
}

/// Frequency
#[derive(
    Debug, Clone, Copy, Deserialize, derive_more::Add, derive_more::Mul, derive_more::From,
)]
pub struct Frequency(f32);

impl std::fmt::Display for Frequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1} Mhz", self.0)
    }
}

impl Serialize for Frequency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Frequency", 2)?;

        state.serialize_field("raw", &self.0)?;
        state.serialize_field("formatted", &format!("{}", self))?;

        state.end()
    }
}
