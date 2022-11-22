use crate::dcc::{Direction, FunctionNum, TimeScale, Timestamp, VelocityValue};
use once_cell::sync::{Lazy};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone)]
pub enum JmriUpdate {
    Function {
        num: FunctionNum,
        is_on: bool,
    },
    Velocity(VelocityValue),
    Direction(Direction),
    Time {
        timestamp: Timestamp,
        scale: TimeScale,
    },
}

pub struct Regexes {
    pub function: Regex,
    pub velocity: Regex,
    pub direction: Regex,
    pub clock: Regex,
}

impl Default for Regexes {
    fn default() -> Self {
        Regexes {
            function: Regex::new(r"F(?P<on>[01])(?P<num>\d\d?)").unwrap(),
            velocity: Regex::new(r"V(?P<v>-?\d{1,3})").unwrap(),
            direction: Regex::new(r"(?P<d>R[01])").unwrap(),
            clock: Regex::new(r"PFT(?P<time>\d+)<;>(?P<scale>\d+(?:\.\d+)?)").unwrap(),
        }
    }
}

pub static REGEXES: Lazy<Regexes> = Lazy::new(Regexes::default);

pub fn jmri_message(msg: &str) -> Option<JmriUpdate> {
    if let Some(captures) = REGEXES.function.captures(msg) {
        let is_on = captures.name("on").unwrap().as_str() == "1";
        let num = FunctionNum::from_str(captures.name("num").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Function { is_on, num });
    }

    if let Some(captures) = REGEXES.velocity.captures(msg) {
        let vel = VelocityValue::from_str(captures.name("v").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Velocity(vel));
    }

    if let Some(captures) = REGEXES.direction.captures(msg) {
        let dir = Direction::from_str(captures.name("d").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Direction(dir));
    }

    None
}
