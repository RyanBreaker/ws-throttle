#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use common::dcc::{DccTime, Direction, Throttle};
use common::jmri::{JmriMessage, JmriStream};
use common::parse;
use common::parse::JmriUpdate;
use common::server::{WSListener, WSMessage};
use tokio::sync::broadcast::error::RecvError;

mod config;

// TODO: Base this on an address request rather than using this hard-code test value
pub const ADDR: &str = "S67";

type ThrottlesState = Arc<Mutex<HashMap<String, Throttle>>>;
type TimeState = Arc<Mutex<DccTime>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let config = Config::get()?;

    let throttles: ThrottlesState = Arc::new(Mutex::new(HashMap::from([(
        ADDR.to_string(),
        Throttle::new(ADDR.to_string()),
    )])));
    let time: TimeState = Arc::new(Mutex::new(DccTime::default()));

    let mut jmri_stream = match JmriStream::new(config.jmri_host).await {
        Ok(stream) => stream,
        Err(e) => panic!("Error connecting to JMRI: {}", e),
    };

    let mut ws_listener = WSListener::new(config.server_host);

    let jmri_sender = jmri_stream.clone_sender();
    let messages = [
        format!("HU{}", config.uuid),
        "NRusty".to_string(),
        format!("MT+{}<;>{}", ADDR, ADDR),
    ];
    for message in messages {
        let msg = JmriMessage::Send(message);
        jmri_sender.send(msg).unwrap();
    }

    // TODO: Better way around creating a bunch of vars?
    let mut jmri_listener = jmri_stream.subscribe();
    let listener_throttle = throttles.clone();
    let listener_time = time.clone();
    let jmri_ws_sender = ws_listener.clone_channel();
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

            if let Some(update) = parse::jmri_message(msg.as_str()) {
                match update.clone() {
                    JmriUpdate::Function { num, is_on } => throttle.set_func(num, is_on),
                    JmriUpdate::Velocity(value) => throttle.set_vel(value),
                    JmriUpdate::Direction(dir) => throttle.set_dir(dir),
                    JmriUpdate::Time { timestamp, scale } => time.update(timestamp, scale),
                };
                let ws_msg = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: serde_json::to_string(&update).unwrap(),
                };
                jmri_ws_sender.send(ws_msg).unwrap();
            }
        }
    });

    // TODO: Better way around creating a bunch of vars?
    let ws_chann_tx = ws_listener.clone_channel();
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

            // If client requests, send entire current Throttle struct
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

            // For testing Serde serialization on the Update messages
            if msg == "test-update" {
                let update_func = JmriUpdate::Function {
                    is_on: true,
                    num: 12,
                };
                let update_func = serde_json::to_string(&update_func).unwrap();
                let update_func = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: update_func,
                };

                let update_dir = JmriUpdate::Direction(Direction::Forward);
                let update_dir = serde_json::to_string(&update_dir).unwrap();
                let update_dir = WSMessage::Send {
                    address: ADDR.to_string(),
                    message: update_dir,
                };

                let update_vel = JmriUpdate::Velocity(20);
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

            if let Ok(msg) = serde_json::from_str::<JmriUpdate>(msg.as_str()) {
                if let Some(request) = make_jmri_request(ADDR, msg) {
                    ws_jmri_sender.send(request).unwrap();
                }
            }
        }
    });

    // TODO: figure out how to fall-through if either exit, handling errors
    jmri_listen_handle.await?;
    ws_listen_handle.await?;

    Ok(())
}

fn make_jmri_request(address: &str, update: JmriUpdate) -> Option<JmriMessage> {
    let msg = match update {
        JmriUpdate::Function { num, is_on } => {
            let is_on = if is_on { "1" } else { "0" };
            JmriMessage::Send(format!("MTA{}<;>F{}{}", address, is_on, num))
        }
        JmriUpdate::Velocity(vel) => {
            let s = format!("MTA{}<;>V{}", address, vel);
            JmriMessage::Send(format!("{}\nMTA{}<;>qV", s, address))
        }
        JmriUpdate::Direction(dir) => {
            let s = format!("MTA{}<;>{}", address, dir);
            JmriMessage::Send(format!("{}\nMTA{}<;>vR", s, address))
        }
        _ => return None,
    };

    Some(msg)
}
