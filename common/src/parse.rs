use crate::dcc::{Direction, FunctionNum, Timestamp, TimeScale, VelocityValue};
use once_cell::sync::Lazy;
use regex::Regex;
use std::str::FromStr;
use serde::{Deserialize, Serialize};


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

pub static RE_FUNCTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"F(?P<on>[01])(?P<num>\d\d?)").unwrap());
pub static RE_VELOCITY: Lazy<Regex> = Lazy::new(|| Regex::new(r"V(?P<v>-?\d{1,3})").unwrap());
pub static RE_DIRECTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?P<d>R[01])").unwrap());
pub static RE_CLOCK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"PFT(?P<time>\d+)<;>(?P<scale>\d+(?:\.\d+)?)").unwrap());

pub fn jmri_message(msg: &str) -> Option<JmriUpdate> {
    if let Some(captures) = RE_FUNCTION.captures(msg) {
        let is_on = captures.name("on").unwrap().as_str() == "1";
        let num = FunctionNum::from_str(captures.name("num").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Function { is_on, num });
    }

    if let Some(captures) = RE_VELOCITY.captures(msg) {
        let vel = VelocityValue::from_str(captures.name("v").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Velocity(vel));
    }

    if let Some(captures) = RE_DIRECTION.captures(msg) {
        let dir = Direction::from_str(captures.name("d").unwrap().as_str()).unwrap();
        return Some(JmriUpdate::Direction(dir));
    }

    None
}
