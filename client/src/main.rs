use shared::{ServerRequest, ServerResponse};
use yarws::{Client, Msg};

#[tokio::main]
async fn main() {
    let socket = Client::new("ws://localhost:7878/ws")
        .connect()
        .await
        .unwrap();
    let (mut tx, mut rx) = socket.into_channel().await;
    tx.send(Msg::Binary(
        bincode::serialize(&ServerRequest::Ping("Hello".to_owned())).unwrap(),
    ))
    .await
    .unwrap_or_default();
    while let Some(msg) = rx.recv().await {
        match msg {
            Msg::Binary(bytes) => match bincode::deserialize::<ServerResponse>(&bytes) {
                Ok(response) => {
                    println!("Got response {:?}", response);
                }
                Err(_) => println!("Error deserializing message."),
            },
            _ => {}
        }
    }
}
