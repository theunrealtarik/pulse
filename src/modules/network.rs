use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use std::{collections::HashMap, time::Duration};

use lib::*;
use serde::{Deserialize, Serialize};

type IfaceName = String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Signal {
    value: i32,
    quality: i32,
}

impl From<i32> for Signal {
    fn from(value: i32) -> Self {
        Self {
            value,
            quality: (2 * (value + 100)).clamp(0, 100),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum Connection {
    #[serde(rename = "wired")]
    Wired,
    #[serde(rename = "wireless")]
    Wireless {
        ssid: String,
        freq: f32,
        signal: Signal,
    },
    #[serde(rename = "unknown")]
    Unknown,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, strum::EnumIter)]
pub enum Flag {
    Up = 1 << 0,
    Broadcast = 1 << 1,
    Debug = 1 << 2,
    Loopback = 1 << 3,
    PointToPoint = 1 << 4,
    NoTrailers = 1 << 5,
    Running = 1 << 6,
    NoArp = 1 << 7,
    Promisc = 1 << 8,
    AllMulti = 1 << 9,
    Master = 1 << 10,
    Slave = 1 << 11,
    Multicast = 1 << 12,
    PortSel = 1 << 13,
    AutoMedia = 1 << 14,
    Dynamic = 1 << 15,
}

impl Flag {
    pub fn from_bits(bits: u32) -> Vec<Self> {
        use strum::IntoEnumIterator;

        Self::iter()
            .filter(|flag| bits & (*flag as u32) != 0)
            .collect()
    }

    pub fn contains(bits: u32, flag: Self) -> bool {
        bits & flag as u32 != 0
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Stats {
    rx: Bytes,
    tx: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    name: IfaceName,
    connection: Connection,
    ip: Option<String>,
    path: PathBuf,
    flags: Vec<Flag>,
    stats: Stats,
}

impl Interface {
    pub fn read_statistics(&self) -> Stats {
        let rx_path = self.path.join("statistics/rx_bytes");
        let tx_path = self.path.join("statistics/tx_bytes");

        let rx = fs::read_to_string(rx_path)
            .unwrap()
            .parse::<u64>()
            .unwrap_or_default();

        let tx = fs::read_to_string(tx_path)
            .unwrap()
            .parse::<u64>()
            .unwrap_or_default();

        Stats {
            rx: Bytes::from(rx),
            tx: Bytes::from(tx),
        }
    }
}

type IfacesMap = HashMap<IfaceName, Interface>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    default: Interface,
    ifaces: IfacesMap,
}

impl Network {
    pub fn get_active_iface() -> Result<IfaceName, PulseError> {
        let cmd = Command::new("ip")
            .args(["route", "get", "1.1.1.1"])
            .output()?;
        let output = String::from_utf8_lossy(&cmd.stdout);
        let mut parts = output.trim().split_whitespace();

        while let Some(token) = parts.next() {
            if token == "dev" {
                if let Some(iface) = parts.next() {
                    return Ok(iface.to_string());
                }
            }
        }

        Err(PulseError::Parse(
            "failed to parse default network iface".to_string(),
        ))
    }
}

pub struct NetworkModule {
    name: String,
    interval: Duration,
    last: Option<Instant>,
}

impl NetworkModule {
    pub fn new(interval: Option<Duration>) -> Self {
        Self {
            name: super::ModuleKind::Net.to_string(),
            interval: interval.unwrap_or(Duration::from_secs(1)),
            last: None,
        }
    }
}

impl super::Module for NetworkModule {
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

    fn load(&mut self) -> Result<serde_json::Value, PulseError> {
        let net_path = PathBuf::from(CLASS_NET);
        let mut ifaces: HashMap<String, Interface> = HashMap::new();

        let addrs = get_if_addrs::get_if_addrs()
            .map_err(lib::PulseError::Io)?
            .into_iter()
            .map(|i| (i.name, i.addr))
            .collect::<HashMap<_, _>>();

        for entry in fs::read_dir(net_path)? {
            let entry = entry?;
            let path = entry.path();

            let name = entry
                .file_name()
                .to_str()
                .ok_or(PulseError::Invalid("invalid interface name".to_string()))?
                .to_string();

            let connection = if path.join("wireless").exists() {
                let cmd = Command::new("iw").args(["dev", &name, "link"]).output()?;
                let output = String::from_utf8_lossy(&cmd.stdout);

                if output.contains("Not connected.") {
                    continue;
                }

                let ssid = parse_from_line!(output, 1)?;
                let freq = parse_from_line!(output, 2)?
                    .parse::<f32>()
                    .map_err(PulseError::from)?;
                let signal_value = parse_from_line!(output, 5)?
                    .split_whitespace()
                    .next()
                    .ok_or(PulseError::Parse("signal split".to_string()))?
                    .parse::<i32>()
                    .map_err(PulseError::from)?;

                Connection::Wireless {
                    ssid,
                    freq,
                    signal: Signal::from(signal_value),
                }
            } else {
                Connection::Wired
            };

            let flags_raw = fs::read_to_string(path.join("flags"))?;
            let flags_str = flags_raw.trim();

            let flags_val = if let Some(hex) = flags_str.strip_prefix("0x") {
                u32::from_str_radix(hex, 16)
            } else {
                flags_str.parse::<u32>()
            }
            .map_err(PulseError::from)?;

            let flags = Flag::from_bits(flags_val);

            let ip = addrs.get(&name).map(|ip| match ip {
                get_if_addrs::IfAddr::V4(addr) => addr.ip.to_string(),
                get_if_addrs::IfAddr::V6(addr) => addr.ip.to_string(),
            });

            ifaces.insert(
                name.clone(),
                Interface {
                    name,
                    connection,
                    ip,
                    path,
                    flags,
                    stats: Stats::default(),
                },
            );
        }

        let default_name = Network::get_active_iface()?;
        let default = ifaces
            .remove(&default_name)
            .ok_or_else(|| PulseError::NotFound(format!("{}", default_name)))?;

        let network = Network { default, ifaces };
        Ok(serde_json::to_value(network).map_err(|err| PulseError::Json(err))?)
    }
}

#[test]
fn test_network_module() {}
