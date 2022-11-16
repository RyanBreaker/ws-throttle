#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::dcc::{DccTime, Throttle};
use crate::jmri::parse::{parse, Update};
use crate::jmri::JmriStream;
use tokio::sync::broadcast::error::RecvError;

mod dcc;
mod jmri;
mod server;

const RETURN: &str = "\n";
pub const UUID: &str = "5ce26240-c61c-417f-94df-4d6571fb1979";
const ADDR: &str = "S67";

type ThrottlesState = Arc<Mutex<HashMap<String, Throttle>>>;
type TimeState = Arc<Mutex<DccTime>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let throttles: ThrottlesState = Arc::new(Mutex::new(HashMap::from([(
        ADDR.to_string(),
        Throttle::new(ADDR.to_string()),
    )])));
    let time: TimeState = Arc::new(Mutex::new(DccTime::new()));

    let mut js = match JmriStream::new("localhost:12090").await {
        Ok(stream) => stream,
        Err(e) => panic!("Error connecting to JMRI: {}", e),
    };

    let mut jmri_listener = js.subscribe();
    let listener_throttle = throttles.clone();
    let listener_time = time.clone();
    let listen_handle = tokio::spawn(async move {
        loop {
            let msg = match jmri_listener.recv().await {
                Ok(msg) => msg,
                Err(e) => match e {
                    RecvError::Closed => {
                        error!("Listener channel closed: {}", e);
                        break;
                    }
                    RecvError::Lagged(_) => continue,
                },
            };

            debug!("Message: {}", msg);

            let mut throttle = listener_throttle.lock().unwrap();
            let throttle = throttle.get_mut(ADDR).unwrap();
            let mut time = listener_time.lock().unwrap();
            match parse(msg.as_str()) {
                None => continue,
                Some(update) => match update {
                    Update::Function { num, is_on } => throttle.set_func(num, is_on),
                    Update::Velocity(value) => throttle.set_vel(value),
                    Update::Direction(dir) => throttle.set_dir(dir),
                    Update::Time { timestamp, scale } => time.update(timestamp, scale),
                },
            }
        }
    });

    listen_handle.await?;

    Ok(())
}
