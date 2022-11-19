use fmt::Display;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

use crate::parse::RE_VELOCITY;
use serde::{Deserialize, Serialize};

pub type VelocityValue = i16;
pub type FunctionNum = u8;
pub type Timestamp = u64;
pub type TimeScale = f32;

#[derive(Serialize, Deserialize)]
pub struct DccTime {
    pub timestamp: Timestamp,
    pub scale: TimeScale,
}

impl DccTime {
    pub fn new(timestamp: Timestamp, scale: TimeScale) -> Self {
        DccTime {
            timestamp,
            scale,
        }
    }

    pub fn update(&mut self, timestamp: u64, scale: f32) {
        self.timestamp = timestamp;
        self.scale = scale;
    }
}


impl Default for DccTime {
    fn default() -> Self {
        Self::new(0, 1.0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Velocity {
    value: i16,
}

impl Velocity {
    pub fn new(value: i16) -> Self {
        Velocity { value }
    }

    pub fn set(&mut self, new: i16) {
        self.value = new.clamp(-1, 129);
    }
}

#[derive(Debug)]
pub struct VelocityParseError {
    value: String,
}

impl Display for VelocityParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.value.as_str())
    }
}

impl Error for VelocityParseError {}

impl FromStr for Velocity {
    type Err = VelocityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = match RE_VELOCITY.captures(s) {
            None => {
                return Err(VelocityParseError {
                    value: format!("Unable to parse '{}' as Velocity", s),
                })
            }
            Some(c) => c,
        };

        let c = captures.name("v").unwrap().as_str();

        Ok(Velocity {
            value: i16::from_str(c).unwrap(),
        })
    }
}

impl Display for Velocity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.value.to_string().as_str())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Direction {
    Reverse,
    Forward,
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Reverse => "R0",
            Self::Forward => "R1",
        };
        f.write_str(s)
    }
}

impl FromStr for Direction {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "R0" => Direction::Reverse,
            _ => Direction::Forward,
        })
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
pub struct Throttle {
    address: String,
    velocity: Velocity,
    direction: Direction,
    functions: HashSet<u8>,
}

#[allow(dead_code)]
impl Throttle {
    pub fn new(address: String) -> Self {
        Throttle {
            address,
            velocity: Velocity::new(0),
            direction: Direction::Forward,
            functions: HashSet::with_capacity(29),
        }
    }

    pub fn is_emergency(&self) -> bool {
        self.velocity.value < 0
    }

    pub fn get_vel(&self) -> i16 {
        self.velocity.value
    }

    pub fn set_vel(&mut self, vel: i16) {
        self.velocity.set(vel);
    }

    pub fn get_dir(&self) -> &Direction {
        &self.direction
    }

    pub fn set_dir(&mut self, dir: Direction) {
        self.direction = dir;
    }

    pub fn get_func(&self, num: &u8) -> bool {
        self.functions.contains(num)
    }

    pub fn set_func(&mut self, num: u8, is_on: bool) {
        if is_on {
            self.functions.insert(num);
        } else {
            self.functions.remove(&num);
        }
    }
}
