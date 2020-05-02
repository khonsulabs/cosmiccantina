use futures::{SinkExt, StreamExt};
use shared::{ServerRequest, ServerResponse};
use warp::{
    filters::ws::{Message, WebSocket},
    Filter,
};

#[tokio::main]
async fn main() {
    // Match any request and return hello world!

    let websockets = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|websocket| websocket_main(websocket)));
    let routes = websockets.or(warp::any().map(|| "Hello, World!"));

    warp::serve(routes).run(([0, 0, 0, 0], 7878)).await;
}

async fn websocket_main(websocket: WebSocket) {
    let (mut tx, mut rx) = websocket.split();

    while let Some(result) = rx.next().await {
        match result {
            Ok(message) => match bincode::deserialize::<ServerRequest>(message.as_bytes()) {
                Ok(request) => {
                    let response = handle_websocket_request(request).await;
                    if let Some(response) = response {
                        tx.send(Message::binary(bincode::serialize(&response).unwrap()))
                            .await
                            .unwrap_or_default();
                    }
                }
                Err(err) => println!("Bincode error: {}", err),
            },
            Err(err) => {
                println!("Error on websocket: {}", err);
                return;
            }
        }
    }
}

async fn handle_websocket_request(request: ServerRequest) -> Option<ServerResponse> {
    match request {
        ServerRequest::Ping(version) => Some(ServerResponse::Pong(version)),
    }
}
