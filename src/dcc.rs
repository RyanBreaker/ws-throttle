use crate::jmri::parse;
use crate::jmri::parse::ParseError;
use fmt::Display;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct DccTime {
    pub timestamp: u64,
    pub scale: f32,
}

impl DccTime {
    pub fn new() -> Self {
        DccTime {
            timestamp: 0,
            scale: 0.0,
        }
    }
    
    pub fn update(&mut self, timestamp: u64, scale: f32) {
        self.timestamp = timestamp;
        self.scale = scale;
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

impl FromStr for Velocity {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = match parse::RE_VELOCITY.captures(s) {
            None => return Err(ParseError::new("Unable to ".to_string())),
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

#[derive(Serialize, Deserialize)]
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
