use crate::dcc::Direction;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

// Regexes for parsing messages from JMRI WiThrottle
// static RE_VERSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"VN(\d+(\.\d)?)").unwrap());
static RE_FUNCTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"F(?P<on>[01])(?P<num>\d\d?)").unwrap());
pub static RE_VELOCITY: Lazy<Regex> = Lazy::new(|| Regex::new(r"V(?P<v>-?\d{1,3})").unwrap());
static RE_DIRECTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?P<d>R[01])").unwrap());
static RE_CLOCK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"PFT(?P<time>\d+)<;>(?P<scale>\d+(?:\.\d+)?)").unwrap());

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    pub fn new(message: String) -> Self {
        ParseError { message }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl std::error::Error for ParseError {}

pub enum Update {
    Function { num: u8, is_on: bool },
    Velocity(i16),
    Direction(Direction),
    Time { timestamp: u64, scale: f32 },
}

pub fn parse(msg: &str) -> Option<Update> {
    if let Some(captures) = RE_FUNCTION.captures(msg) {
        let is_on = captures.name("on").unwrap().as_str() == "1";
        let num = u8::from_str(captures.name("num").unwrap().as_str()).unwrap();
        return Some(Update::Function { is_on, num });
    }

    if let Some(captures) = RE_VELOCITY.captures(msg) {
        let vel = i16::from_str(captures.name("v").unwrap().as_str()).unwrap();
        return Some(Update::Velocity(vel));
    }

    if let Some(captures) = RE_DIRECTION.captures(msg) {
        let dir = Direction::from_str(captures.name("d").unwrap().as_str()).unwrap();
        return Some(Update::Direction(dir));
    }

    if let Some(captures) = RE_CLOCK.captures(msg) {
        let timestamp = captures.name("time").unwrap().as_str();
        let timestamp = u64::from_str(timestamp).unwrap();

        let scale = captures.name("scale").unwrap().as_str();
        let scale = f32::from_str(scale).unwrap();

        return Some(Update::Time { timestamp, scale });
    }

    None
}
