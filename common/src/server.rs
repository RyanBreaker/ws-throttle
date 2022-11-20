use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use warp::ws::{Message, WebSocket, Ws};
use warp::{Error, Filter};

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

type Channel = Sender<WSMessage>;

impl WSListener {
    pub fn new() -> Self {
        let (channel, _) = broadcast::channel::<WSMessage>(10);
        let listener_handle = make_ws_handle(channel.clone());

        WSListener {
            listener_handle,
            channel,
        }
    }

    pub fn clone_channel(&mut self) -> Channel {
        self.channel.clone()
    }

    pub fn subscribe(&mut self) -> Receiver<WSMessage> {
        self.channel.subscribe()
    }
}

impl Default for WSListener {
    fn default() -> Self {
        Self::new()
    }
}

fn make_ws_handle(channel: Sender<WSMessage>) -> JoinHandle<()> {
    let channel = warp::any().map(move || channel.clone());
    let health_route = warp::path("health").map(|| "OK");
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(channel)
        .map(|ws: Ws, channel: Channel| ws.on_upgrade(move |ws| handle_ws_connection(ws, channel)));

    let routes = health_route.or(ws_route);

    tokio::spawn(async move {
        warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
    })
}

fn make_ws_send_handle(
    mut receiver: Receiver<WSMessage>,
    mut tx: SplitSink<WebSocket, Message>,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        loop {
            let msg = match receiver.recv().await {
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

            match tx.send(Message::text(msg)).await {
                Ok(_) => {}
                Err(e) => return Err(e),
            };
        }

        Ok(())
    })
}

fn make_ws_receive_handle(sender: Sender<WSMessage>, mut rx: SplitStream<WebSocket>) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(msg) = rx.next().await {
            let message = match msg {
                Ok(msg) => {
                    if let Ok(s) = msg.to_str() {
                        s.to_string()
                    } else {
                        continue;
                    }
                }
                Err(_e) => continue,
            };

            let message = WSMessage::Receive {
                address: ADDR.to_string(),
                message,
            };

            let _ = sender.send(message);
        }
    })
}

async fn handle_ws_connection(ws: WebSocket, channel: Channel) {
    let (ws_tx, ws_rx) = ws.split();

    let send_handle = make_ws_send_handle(channel.subscribe(), ws_tx);
    let receive_handle = make_ws_receive_handle(channel.clone(), ws_rx);

    // TODO: Fall-through on failures or breaks
    let _ = send_handle.await;
    let _ = receive_handle.await;
}
