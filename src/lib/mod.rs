use std::fs;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use serde::{Deserialize, Serialize, ser::SerializeStruct};
use strum::IntoEnumIterator;
use sysinfo::{Disks, System};

pub const CLASS_NET: &str = "/sys/class/net";
pub const CLASS_HWMON: &str = "/sys/class/hwmon";
pub const PROC_NET: &str = "/proc/net";
pub const PROC_CPUINFO: &str = "/proc/cpuinfo";

// Helpers

pub type SharedSystem = Rc<RefCell<System>>;
pub type SharedDisks = Rc<RefCell<Disks>>;

#[macro_export]
macro_rules! parse_from_line {
    ($text:expr, $line_idx:expr) => {
        $text
            .lines()
            .nth($line_idx)
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .ok_or(lib::PulseError::Parse("iw output".to_string()))
    };
}

#[derive(Debug, Clone, derive_more::From)]
pub struct Monitor(PathBuf);

impl Monitor {
    pub fn path(&self) -> &PathBuf {
        &self.0
    }

    pub fn find_many_in_dir<F>(dir: &PathBuf, filter: F) -> Result<Vec<PathBuf>, PulseError>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        Ok(fs::read_dir(&dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| match filter(&p) {
                Ok(true) => true,
                _ => false,
            })
            .collect::<Vec<_>>())
    }

    pub fn find_in_dir<F>(dir: &PathBuf, filter: F) -> Result<PathBuf, PulseError>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        match Self::find_many_in_dir(dir, filter)?.first() {
            Some(p) => Ok(p.clone()),
            None => Err(PulseError::NotFound(format!(
                "failed to find target in {}",
                dir.to_string_lossy()
            ))),
        }
    }

    pub fn new<F>(filter: F) -> Result<Self, PulseError>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        let hwmon_path = PathBuf::from(CLASS_HWMON);
        Self::find_in_dir(&hwmon_path, filter).map(Self)
    }

    /// Returns [`Monitor`] of path [`CLASS_HWMON`]`/hwmonX` based on the value of [`CLASS_HWMON`]`/hwmonX/name`
    pub fn from_name<F>(filter: F) -> Result<Self, PulseError>
    where
        F: Fn(String) -> bool,
    {
        Monitor::new(|path| {
            let name = fs::read_to_string(path.join("name"))?;
            let name = name.to_lowercase();
            return Ok(filter(name));
        })
    }

    pub fn entry<F>(&self, filter: F) -> Result<PathBuf, PulseError>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        Self::find_in_dir(&self.0, filter)
    }

    pub fn read<F>(&self, filter: F) -> Result<String, PulseError>
    where
        F: Fn(&PathBuf) -> Result<bool, PulseError>,
    {
        self.entry(filter)
            .map(fs::read_to_string)?
            .map(|s| s.trim().to_string())
            .map_err(PulseError::from)
    }
}

// Error

#[derive(Debug)]
pub enum PulseError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Init(String),
    Parse(String),
    Missing(String),
    Invalid(String),
    NotFound(String),
    ParseInt(std::num::ParseIntError),
    ParseFloat(std::num::ParseFloatError),
}

impl std::fmt::Display for PulseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Json(err) => write!(f, "JSON error: {}", err),
            Self::Init(err) => write!(f, "Initialization: {}", err),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Missing(msg) => write!(f, "Missing: {}", msg),
            Self::Invalid(msg) => write!(f, "Invalid: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::ParseInt(err) => write!(f, "Parse int error: {}", err),
            Self::ParseFloat(err) => write!(f, "Parse float error: {}", err),
        }
    }
}

impl std::error::Error for PulseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PulseError::Io(err) => Some(err),
            PulseError::Json(err) => Some(err),
            PulseError::ParseInt(err) => Some(err),
            PulseError::ParseFloat(err) => Some(err),
            _ => None,
        }
    }
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

impl From<std::num::ParseIntError> for PulseError {
    fn from(e: std::num::ParseIntError) -> Self {
        PulseError::ParseInt(e)
    }
}

impl From<std::num::ParseFloatError> for PulseError {
    fn from(e: std::num::ParseFloatError) -> Self {
        PulseError::ParseFloat(e)
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
        state.serialize_field("formatted", &format!("{}", &self.format()))?;

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

impl From<f64> for Percent {
    fn from(value: f64) -> Self {
        Self(value as f32)
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
