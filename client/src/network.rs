use kludgine::prelude::*;
use shared::{ServerRequest, ServerResponse, UserProfile};
use std::time::Duration;
use uuid::Uuid;
use yarws::{Client, Msg};

lazy_static! {
    static ref NETWORK: KludgineHandle<Network> = { KludgineHandle::new(Network::new()) };
}

#[derive(Clone, Debug)]
pub enum LoginState {
    LoggedOut,
    Connected { installation_id: Uuid },
    Authenticated { profile: UserProfile },
}

pub struct Network {
    login_state: LoginState,
}

impl Network {
    fn new() -> Self {
        Self {
            login_state: LoginState::LoggedOut,
        }
    }

    pub async fn spawn() {
        Runtime::spawn(network_loop())
    }

    async fn set_login_state(state: LoginState) {
        let mut network = NETWORK.write().await;
        network.login_state = state;
    }

    pub async fn login_state() -> LoginState {
        let network = NETWORK.read().await;
        network.login_state.clone()
    }
}

async fn network_loop() {
    loop {
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
                installation_id: None,
            })
            .unwrap(),
        ))
        .await
        .unwrap_or_default();
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::Binary(bytes) => match bincode::deserialize::<ServerResponse>(&bytes) {
                    Ok(response) => match response {
                        ServerResponse::Error { message } => {
                            println!("Authentication error {:?}", message);
                            Network::set_login_state(LoginState::LoggedOut).await;
                        }
                        ServerResponse::AdoptInstallationId { installation_id } => {
                            println!("Received app token {}", installation_id);
                            Network::set_login_state(LoginState::Connected { installation_id })
                                .await;
                        }
                        ServerResponse::Authenticated { profile } => {
                            println!("Authenticated as {}", profile.username);
                            Network::set_login_state(LoginState::Authenticated { profile }).await;
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
