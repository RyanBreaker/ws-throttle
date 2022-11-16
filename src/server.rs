use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use warp::ws::{WebSocket, Ws};
use warp::Filter;

pub struct WSListener {
    listener_handle: JoinHandle<()>,
}

impl WSListener {
    pub fn new() -> Self {
        // reeeeee
        let (send_tx, send_rx) = mpsc::channel::<String>(10);
        let health_route = warp::path("health").map(|| "OK");
        let ws_route = warp::path("ws").and(warp::ws()).and(source_bundle).map(
            |ws: Ws, bundle: (broadcast::Sender<String>, broadcast::Sender<String>)| {
                ws.on_upgrade(move |socket| handle_ws(socket, (bundle.0, bundle.1.subscribe())))
            },
        );
        let routes = health_route
            .or(ws_route)
            .with(warp::cors().allow_any_origin());

        let listener_handle = tokio::spawn(async move {
            warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
        });

        WSListener {
            listener_handle,
            channels: (send_tx, listen_rx),
        }
    }
}

async fn handle_ws(ws: WebSocket, bundle: Bundle) {}
