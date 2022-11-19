use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

const ADDR: &str = "S67";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WSMessage {
    Send { address: String, message: String },
    Receive { address: String, message: String },
}

#[allow(dead_code)]
pub struct WSListener {
    listener_handle: JoinHandle<()>,
    channel: Channel,
}

type Channel = broadcast::Sender<WSMessage>;

impl WSListener {
    pub fn new() -> Self {
        let (channel, _) = broadcast::channel::<WSMessage>(10);

        let health_route = warp::path("health").map(|| "OK");

        let channel_ws = channel.clone();
        let channel_ws = warp::any().map(move || channel_ws.clone());
        let ws_route =
            warp::path("ws")
                .and(warp::ws())
                .and(channel_ws)
                .map(|ws: Ws, channel: Channel| {
                    ws.on_upgrade(move |ws| handle_connection(ws, channel))
                });

        let routes = health_route.or(ws_route);

        let listener_handle = tokio::spawn(async move {
            warp::serve(routes).run(([0, 0, 0, 0], 8081)).await;
        });

        WSListener {
            listener_handle,
            channel,
        }
    }

    pub fn clone_channel(&mut self) -> Channel {
        self.channel.clone()
    }

    pub fn subscribe(&mut self) -> broadcast::Receiver<WSMessage> {
        self.channel.subscribe()
    }
}

impl Default for WSListener {
    fn default() -> Self {
        Self::new()
    }
}

async fn handle_connection(ws: WebSocket, channel: Channel) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    let mut channel_rx = channel.subscribe();
    let send_handle = tokio::spawn(async move {
        loop {
            let msg = match channel_rx.recv().await {
                Ok(msg) => match msg {
                    WSMessage::Send { address, message } => {
                        if !address.eq(ADDR) {
                            continue;
                        }
                        message
                    }
                    WSMessage::Receive { .. } => continue,
                },
                Err(e) => match e {
                    RecvError::Closed => break,
                    RecvError::Lagged(_) => continue,
                },
            };

            let _ = ws_tx.send(Message::text(msg)).await;
        }
    });

    let channel_tx = channel.clone();
    let receive_handle = tokio::spawn(async move {
        while let Some(msg) = ws_rx.next().await {
            let message = match msg {
                Ok(msg) => {
                    if let Ok(s) = msg.to_str() {
                        s.to_string()
                    } else {
                        continue;
                    }
                }
                Err(_e) => {
                    // TODO create error?
                    break;
                }
            };

            let message = WSMessage::Receive {
                address: ADDR.to_string(),
                message,
            };
            let _ = channel_tx.send(message);
        }
    });

    let _ = send_handle.await;
    let _ = receive_handle.await;
}
