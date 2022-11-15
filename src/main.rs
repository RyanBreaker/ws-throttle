#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::error::Error;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

mod dcc;
mod jmri;
mod server;

const RETURN: char = '\n';
const UUID: &str = "5ce26240-c61c-417f-94df-4d6571fb1979";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    // Direct stream to JMRI
    let jmri_stream = match TcpStream::connect("localhost:12090").await {
        Ok(stream) => stream,
        Err(e) => panic!("Error connecting to JMRI: {}", e),
    };
    let (jmri_stream_reader, jmri_stream_writer) = jmri_stream.into_split();
    let jmri_stream_reader = BufReader::new(jmri_stream_reader);

    // JMRI Receive channels - handling for messages from JMRI
    let (jmri_receive_tx, _jmri_receive_rx) = mpsc::channel::<String>(32);

    // JMRI Send channels - handling for messages to send to JMRI
    let (jmri_send_tx, jmri_send_rx) = mpsc::channel::<String>(32);

    // JMRI stream handles
    let stream_read_handle = make_stream_read_handle(jmri_receive_tx, jmri_stream_reader);
    let stream_write_handle =
        make_stream_write_handle(jmri_send_tx, jmri_send_rx, jmri_stream_writer);

    // let state_handler_handle = tokio::spawn(async move {});

    // TODO: Better handler handling
    let _ = stream_read_handle.await;
    let _ = stream_write_handle.await;

    Ok(())
}

fn make_stream_read_handle(
    sender: Sender<String>,
    mut reader: BufReader<OwnedReadHalf>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut line = String::new();

        loop {
            let result = reader.read_line(&mut line).await;
            if let Some(e) = result.err() {
                error!("Error reading line from JMRI: {}", e);
                return;
            }

            let lines = line
                .split(RETURN)
                .map(|line| line.trim())
                .filter(|line| !line.is_empty());

            for line in lines {
                let result = sender.send(line.to_string()).await;
                if let Some(e) = result.err() {
                    error!("Error sending to JMRI receive channel: {}", e);
                    return;
                }
            }

            line.clear();
        }
    })
}

fn make_stream_write_handle(
    sender: Sender<String>,
    mut receiver: Receiver<String>,
    mut writer: OwnedWriteHalf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Queue opening messages
        let _ = sender.send(format!("HU{}{}", UUID, RETURN));
        let _ = sender.send(format!("NRusty{}", RETURN));

        while let Some(message) = receiver.recv().await {
            let result = writer
                .write_all(format!("{}{}", message, RETURN).as_bytes())
                .await;
            if let Some(e) = result.err() {
                error!("Error sending to JMRI: {}", e);
                break;
            }
        }
    })
}
