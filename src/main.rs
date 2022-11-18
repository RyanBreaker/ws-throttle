#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast::error::RecvError;

use crate::dcc::{DccTime, Direction, Throttle};
use crate::jmri::{JmriMessage, JmriStream};
use crate::jmri::parse::{parse, Update};
use crate::server::{WSListener, WSMessage};

mod dcc;
mod jmri;
mod server;

pub const RETURN: &str = "\n";
pub const UUID: &str = "5ce26240-c61c-417f-94df-4d6571fb1979";
pub const ADDR: &str = "S67";

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

    // let mut js = match JmriStream::new("localhost:12090").await {
    let mut js = match JmriStream::new("192.168.3.9:12090").await {
        Ok(stream) => stream,
        Err(e) => panic!("Error connecting to JMRI: {}", e),
    };

    let mut ws = WSListener::new();

    let jmri_sender = js.clone_sender();
    let messages = [
        format!("HU{}", UUID),
        "NRusty".to_string(),
        format!("MT+{}<;>{}", ADDR, ADDR),
    ];
    for message in messages {
        let msg = JmriMessage::Send(message);
        jmri_sender.send(msg).unwrap();
    }

    let mut jmri_listener = js.subscribe();
    let listener_throttle = throttles.clone();
    let listener_time = time.clone();
    let jmri_ws_sender = ws.clone_channel();
    let jmri_listen_handle = tokio::spawn(async move {
        loop {
            let msg = match jmri_listener.recv().await {
                Ok(msg) => match msg {
                    JmriMessage::Send(_) => continue,
                    JmriMessage::Receive(msg) => msg,
                },
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

            if let Some(update) = parse(msg.as_str()) {
                match update.clone() {
                    Update::Function { num, is_on } => throttle.set_func(num, is_on),
                    Update::Velocity(value) => throttle.set_vel(value),
                    Update::Direction(dir) => throttle.set_dir(dir),
                    Update::Time { timestamp, scale } => time.update(timestamp, scale),
                };
                let ws_msg = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: serde_json::to_string(&update).unwrap(),
                };
                jmri_ws_sender.send(ws_msg).unwrap();
            }
        }
    });

    let ws_chann_tx = ws.clone_channel();
    let ws_throttles = throttles.clone();
    let ws_jmri_sender = jmri_sender.clone();
    let ws_listen_handle = tokio::spawn(async move {
        let mut ws_chann_rx = ws_chann_tx.subscribe();
        loop {
            let msg = match ws_chann_rx.recv().await {
                Ok(msg) => match msg {
                    WSMessage::Send { .. } => continue,
                    WSMessage::Receive { message, .. } => message,
                },
                Err(e) => match e {
                    RecvError::Closed => break,
                    RecvError::Lagged(_) => continue,
                },
            };

            if msg == "update" {
                let throttle = ws_throttles.lock().unwrap();
                let throttle = throttle.get(ADDR).unwrap();
                let message = serde_json::to_string(throttle).unwrap();
                let send = WSMessage::Send {
                    address: ADDR.to_string(),
                    message,
                };
                ws_chann_tx.send(send).unwrap();
                continue;
            }

            if msg == "test-update" {
                let update_func = Update::Function {
                    is_on: true,
                    num: 12,
                };
                let update_func = serde_json::to_string(&update_func).unwrap();
                let update_func = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: update_func,
                };

                let update_dir = Update::Direction(Direction::Forward);
                let update_dir = serde_json::to_string(&update_dir).unwrap();
                let update_dir = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: update_dir,
                };

                let update_vel = Update::Velocity(20);
                let update_vel = serde_json::to_string(&update_vel).unwrap();
                let update_vel = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: update_vel,
                };

                ws_chann_tx.send(update_func).unwrap();
                ws_chann_tx.send(update_dir).unwrap();
                ws_chann_tx.send(update_vel).unwrap();
                continue;
            }

            if let Ok(msg) = serde_json::from_str::<Update>(msg.as_str()) {
                if let Some(request) = make_jmri_request(ADDR, msg) {
                    ws_jmri_sender.send(request).unwrap();
                }
            }
        }
    });

    jmri_listen_handle.await?;
    ws_listen_handle.await?;

    Ok(())
}

fn make_jmri_request(address: &str, update: Update) -> Option<JmriMessage> {
    let msg = match update {
        Update::Function { num, is_on } => {
            let is_on = if is_on { "1" } else { "0" };
            JmriMessage::Send(format!("MTA{}<;>F{}{}", address, is_on, num))
        }
        Update::Velocity(vel) => {
            let s = format!("MTA{}<;>V{}", address, vel);
            JmriMessage::Send(format!("{}\nMTA{}<;>qV", s, address))
        }
        Update::Direction(dir) => {
            let s = format!("MTA{}<;>{}", address, dir);
            JmriMessage::Send(format!("{}\nMTA{}<;>vR", s, address))
        }
        Update::Time { .. } => return None,
    };

    Some(msg)
}
