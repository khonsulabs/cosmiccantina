use crate::config::UserConfig;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use kludgine::prelude::*;
use shared::{current_timestamp, ServerRequest, ServerResponse, UserProfile};
use std::time::Duration;
use tokio::sync::mpsc::{
    error::TryRecvError as TokioTryRecvError, Receiver as TokioReceiver, Sender as TokioSender,
};
use yarws::{Client, Msg};

lazy_static! {
    static ref NETWORK: KludgineHandle<Network> = { KludgineHandle::new(Network::new()) };
}

#[derive(Clone, Debug)]
pub enum LoginState {
    LoggedOut,
    Connected,
    Authenticated { profile: UserProfile },
    Error { message: Option<String> },
}

pub struct Network {
    login_state: LoginState,
    sender: Sender<ServerRequest>,
    receiver: Receiver<ServerRequest>,
    roundtrip: f64,
    world_timestamp: f64,
    profiles: Option<Vec<UserProfile>>,
}

impl Network {
    fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            login_state: LoginState::LoggedOut,
            sender,
            receiver,
            roundtrip: 0.0,
            world_timestamp: 0.0,
            profiles: None,
        }
    }

    pub async fn spawn() {
        Runtime::spawn(network_loop());
    }

    async fn set_login_state(state: LoginState) {
        let mut network = NETWORK.write().await;
        network.login_state = state;
    }

    pub async fn login_state() -> LoginState {
        let network = NETWORK.read().await;
        network.login_state.clone()
    }

    pub async fn request(request: ServerRequest) {
        println!("Sending request: {:?}", request);
        let network = NETWORK.read().await;
        network.sender.send(request).unwrap_or_default();
    }

    async fn receiver() -> Receiver<ServerRequest> {
        let network = NETWORK.read().await;
        network.receiver.clone()
    }

    async fn world_updated(timestamp: f64, profiles: Vec<UserProfile>) {
        let mut network = NETWORK.write().await;
        network.world_timestamp = timestamp;
        network.profiles = Some(profiles);
    }

    async fn ping_updated(new_roundtrip: f64) {
        let mut network = NETWORK.write().await;
        network.roundtrip = new_roundtrip;
    }

    pub async fn ping() -> f64 {
        let network = NETWORK.read().await;
        network.roundtrip
    }

    pub async fn last_world_update() -> Option<(f64, Vec<UserProfile>)> {
        let mut network = NETWORK.write().await;
        let mut profiles = None;
        std::mem::swap(&mut profiles, &mut network.profiles);

        profiles.map(|profiles| (network.world_timestamp, profiles))
    }
}

async fn network_loop() {
    loop {
        let socket = match Client::new(&format!(
            "{}/ws",
            std::env::var("SERVER_URL").unwrap_or("wss://cantina.khonsu.gg".to_owned())
        ))
        .connect()
        .await
        {
            Ok(socket) => socket,
            Err(err) => {
                println!("Error connecting to socket. {}", err);
                tokio::time::delay_for(Duration::from_millis(100)).await;
                Network::set_login_state(LoginState::Error { message: None }).await;
                continue;
            }
        };
        let (mut tx, mut rx) = socket.into_channel().await;
        let receiver = Network::receiver().await;
        let mut network_limiter = FrequencyLimiter::new(Duration::from_millis(1));
        Network::request(ServerRequest::Authenticate {
            installation_id: UserConfig::installation_id().await,
            version: shared::PROTOCOL_VERSION.to_owned(),
        })
        .await;

        loop {
            network_limiter.advance_frame();

            if receive_loop(&mut rx).await || send_loop(&receiver, &mut tx).await {
                break;
            }

            if let Some(sleep_time) = network_limiter.remaining() {
                tokio::time::delay_for(sleep_time).await;
            }
        }
    }
}

async fn receive_loop(rx: &mut TokioReceiver<Msg>) -> bool {
    let mut average_server_timestamp_delta = 0f64;
    loop {
        match rx.try_recv() {
            Ok(msg) => match msg {
                Msg::Binary(bytes) => match bincode::deserialize::<ServerResponse>(&bytes) {
                    Ok(response) => match response {
                        ServerResponse::Error { message } => {
                            Network::set_login_state(LoginState::Error { message }).await;
                        }
                        ServerResponse::AdoptInstallationId { installation_id } => {
                            println!("Received app token {}", installation_id);
                            UserConfig::set_installation_id(installation_id).await;
                            Network::set_login_state(LoginState::Connected).await;
                        }
                        ServerResponse::Authenticated { profile } => {
                            println!("Authenticated as {}", profile.username);
                            Network::set_login_state(LoginState::Authenticated { profile }).await;
                        }
                        ServerResponse::WorldUpdate {
                            timestamp,
                            profiles,
                        } => {
                            Network::world_updated(
                                timestamp - average_server_timestamp_delta,
                                profiles,
                            )
                            .await;
                        }
                        ServerResponse::AuthenticateAtUrl { url } => {
                            webbrowser::open(&url).expect("Error launching URL");
                        }
                        ServerResponse::Ping {
                            timestamp,
                            average_server_timestamp_delta: delta,
                            average_roundtrip,
                        } => {
                            average_server_timestamp_delta = delta;
                            Network::ping_updated(average_roundtrip).await;
                            Network::request(ServerRequest::Pong {
                                original_timestamp: timestamp,
                                timestamp: current_timestamp(),
                            })
                            .await;
                        }
                    },
                    Err(_) => println!("Error deserializing message."),
                },
                _ => {}
            },
            Err(err) => match err {
                TokioTryRecvError::Closed => {
                    println!("Socket Disconnected");
                    return true;
                }
                _ => return false,
            },
        }
    }
}

async fn send_loop(receiver: &Receiver<ServerRequest>, tx: &mut TokioSender<Msg>) -> bool {
    loop {
        match receiver.try_recv() {
            Ok(request) => {
                match tx
                    .send(Msg::Binary(bincode::serialize(&request).unwrap()))
                    .await
                {
                    Err(err) => {
                        println!("Error sending message: {}", err);
                        return true;
                    }
                    _ => {}
                }
            }
            Err(err) => match err {
                TryRecvError::Disconnected => return true,
                TryRecvError::Empty => return false,
            },
        }
    }
}
