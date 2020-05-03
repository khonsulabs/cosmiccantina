use shared::{ServerRequest, ServerResponse, UserProfile};
use std::time::Duration;
use yarws::{Client, Msg};

enum LoginState {
    LoggedOut,
    Connected { token: String },
    Authenticated { profile: UserProfile },
}

#[tokio::main]
async fn main() {
    loop {
        let mut login_state = LoginState::LoggedOut;
        let socket = match Client::new("ws://localhost:7878/ws").connect().await {
            Ok(socket) => socket,
            Err(err) => {
                println!("Error connecting to socket. {}", err);
                tokio::time::delay_for(Duration::from_millis(100)).await;
                continue;
            }
        };
        let (mut tx, mut rx) = socket.into_channel().await;
        tx.send(Msg::Binary(
            bincode::serialize(&ServerRequest::Authenticate {
                previous_token: None,
            })
            .unwrap(),
        ))
        .await
        .unwrap_or_default();
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Binary(bytes) => match bincode::deserialize::<ServerResponse>(&bytes) {
                    Ok(response) => match response {
                        ServerResponse::AuthenticationError { message } => {
                            println!("Authentication error {:?}", message);
                        }
                        ServerResponse::AdoptToken { token } => {
                            println!("Received app token {}", token);
                            login_state = LoginState::Connected { token };
                        }
                        ServerResponse::Authenticated { profile } => {
                            println!("Authenticated as {}", profile.username);
                            login_state = LoginState::Authenticated { profile };
                        }
                        ServerResponse::AuthenticateAtUrl { url } => {
                            webbrowser::open(&url).expect("Error launching URL");
                        }
                    },
                    Err(_) => println!("Error deserializing message."),
                },
                _ => {}
            }
        }
    }
}
