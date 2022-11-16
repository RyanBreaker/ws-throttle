pub mod parse;

use crate::RETURN;
use std::io;
use std::io::ErrorKind;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

struct JmriStreamChannels {
    listen: broadcast::Sender<String>,
    send: mpsc::Sender<String>,
}

pub struct JmriStream {
    channels: JmriStreamChannels,
    listen_handle: JoinHandle<io::Result<()>>,
    send_handle: JoinHandle<io::Result<()>>,
}

impl JmriStream {
    pub async fn new(address: &str) -> io::Result<JmriStream> {
        let stream = TcpStream::connect(address).await?;
        let (stream_reader, mut stream_writer) = stream.into_split();
        let mut stream_reader = BufReader::new(stream_reader);

        let (listen_tx, _listen_rx) = broadcast::channel::<String>(32);
        let listen_tx_handle = listen_tx.clone();
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
                    if let Err(e) = listen_tx_handle.send(line.to_string()) {
                        error!("Error sending to JmriStream channel: {}", e);
                        return Err(io::Error::new(ErrorKind::Interrupted, e));
                    }
                }
                line.clear();
            }
        });

        let (send_tx, mut send_rx) = mpsc::channel::<String>(32);
        let _ = send_tx.send(format!("HUskjdfkjdsf{}", RETURN)).await;
        let _ = send_tx.send(format!("NRusty{}", RETURN)).await;

        let send_handle: JoinHandle<io::Result<()>> = tokio::spawn(async move {
            // TODO: Maybe rewrite as other kind of loop for error handling
            while let Some(msg) = send_rx.recv().await {
                stream_writer
                    .write_all([msg.as_str(), RETURN].concat().as_bytes())
                    .await?;
            }

            Ok(())
        });

        let channels = JmriStreamChannels {
            listen: listen_tx,
            send: send_tx,
        };

        Ok(JmriStream {
            channels,
            listen_handle,
            send_handle,
        })
    }

    pub fn clone_sender(&mut self) -> mpsc::Sender<String> {
        self.channels.send.clone()
    }

    pub fn subscribe(&mut self) -> broadcast::Receiver<String> {
        self.channels.listen.subscribe()
    }
}
