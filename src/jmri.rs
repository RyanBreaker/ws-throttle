use std::fmt::{Display, Formatter};
use std::io;
use std::io::ErrorKind;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::RETURN;

pub mod parse;

#[derive(Clone, Debug)]
pub enum JmriMessage {
    Send(String),
    Receive(String),
}

impl Display for JmriMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JmriMessage::Send(msg) => f.write_str(["Send: ", msg].concat().as_str()),
            JmriMessage::Receive(msg) => f.write_str(["Receive: ", msg].concat().as_str()),
        }
    }
}

#[allow(dead_code)]
pub struct JmriStream {
    listen_handle: JoinHandle<io::Result<()>>,
    send_handle: JoinHandle<io::Result<()>>,
    channel: broadcast::Sender<JmriMessage>,
}

impl JmriStream {
    pub async fn new(address: &str) -> io::Result<JmriStream> {
        let stream = TcpStream::connect(address).await?;
        let (stream_reader, mut stream_writer) = stream.into_split();
        let mut stream_reader = BufReader::new(stream_reader);

        let (channel, _) = broadcast::channel::<JmriMessage>(32);

        let listen_handle_tx = channel.clone();
        let listen_handle: JoinHandle<io::Result<()>> = tokio::spawn(async move {
            let mut line = String::new();
            loop {
                if let Err(e) = stream_reader.read_line(&mut line).await {
                    error!("Error reading line from JMRI: {}", e);
                    return Err(e);
                }

                let lines = line
                    .split(RETURN)
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty());
                for line in lines {
                    let message = JmriMessage::Receive(line.to_string());
                    if let Err(e) = listen_handle_tx.send(message) {
                        error!("Error sending to JmriStream channel: {}", e);
                        return Err(io::Error::new(ErrorKind::Interrupted, e));
                    }
                }
                line.clear();
            }
        });

        let mut send_rx = channel.subscribe();
        let send_handle: JoinHandle<io::Result<()>> = tokio::spawn(async move {
            loop {
                let msg = match send_rx.recv().await {
                    Ok(msg) => match msg {
                        JmriMessage::Send(msg) => msg,
                        JmriMessage::Receive(_) => continue,
                    },
                    Err(e) => match e {
                        RecvError::Closed => {
                            error!("Error receiving send message: {}", e);
                            break;
                        }
                        RecvError::Lagged(_) => continue,
                    },
                };

                stream_writer
                    .write_all([msg.as_str(), RETURN].concat().as_bytes())
                    .await?;
            }

            Ok(())
        });

        Ok(JmriStream {
            listen_handle,
            send_handle,
            channel,
        })
    }

    pub fn clone_sender(&mut self) -> broadcast::Sender<JmriMessage> {
        self.channel.clone()
    }

    pub fn subscribe(&mut self) -> broadcast::Receiver<JmriMessage> {
        self.channel.subscribe()
    }
}
