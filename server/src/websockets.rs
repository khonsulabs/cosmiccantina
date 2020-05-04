use super::env;
use async_std::sync::RwLock;
use crossbeam::channel::{unbounded, Sender};
use futures::{executor::block_on, SinkExt, StreamExt};
use lazy_static::lazy_static;
use migrations::{pg, sqlx};
use shared::{Installation, ServerRequest, ServerResponse, UserProfile};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use url::Url;
use uuid::Uuid;
use warp::filters::ws::{Message, WebSocket};

lazy_static! {
    pub static ref CONNECTED_CLIENTS: ConnectedClients = { ConnectedClients::default() };
}

pub struct ConnectedClients {
    senders: Arc<RwLock<HashMap<Uuid, Sender<ServerResponse>>>>,
    installations_by_account: Arc<RwLock<HashMap<i64, HashSet<Uuid>>>>,
    account_by_installation: Arc<RwLock<HashMap<Uuid, i64>>>,
}

impl Default for ConnectedClients {
    fn default() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
            installations_by_account: Arc::new(RwLock::new(HashMap::new())),
            account_by_installation: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ConnectedClients {
    pub async fn connect(&self, installation_id: Uuid, sender: Sender<ServerResponse>) {
        let mut senders = self.senders.write().await;
        senders.insert(installation_id, sender);
    }

    pub async fn associate_account(&self, installation_id: Uuid, account_id: i64) {
        let mut installations_by_account = self.installations_by_account.write().await;
        let mut account_by_installation = self.account_by_installation.write().await;
        account_by_installation.insert(installation_id, account_id);
        let installations = installations_by_account
            .entry(account_id)
            .or_insert_with(|| HashSet::new());
        installations.insert(installation_id);
    }

    pub async fn disconnect(&self, installation_id: Uuid) {
        let mut installations_by_account = self.installations_by_account.write().await;
        let account_by_installation = self.account_by_installation.read().await;
        let mut senders = self.senders.write().await;

        senders.remove(&installation_id);
        if let Some(account_id) = account_by_installation.get(&installation_id) {
            let remove_account =
                if let Some(installations) = installations_by_account.get_mut(account_id) {
                    installations.remove(&installation_id);
                    installations.len() == 0
                } else {
                    false
                };
            if remove_account {
                installations_by_account.remove(&account_id);
            }
        }
    }

    pub async fn send_to_installation_id(&self, installation_id: Uuid, message: ServerResponse) {
        let senders = self.senders.write().await;
        if let Some(sender) = senders.get(&installation_id) {
            sender.send(message).unwrap_or_default();
        }
    }
}

#[derive(Default)]
pub struct ConnectedClient {
    installation_id: Option<Uuid>,
}

pub async fn main(websocket: WebSocket) {
    let (mut tx, mut rx) = websocket.split();
    let (sender, transmission_receiver) = unbounded();

    tokio::spawn(async move {
        while let Ok(response) = transmission_receiver.recv() {
            tx.send(Message::binary(bincode::serialize(&response).unwrap()))
                .await
                .unwrap_or_default()
        }
    });

    let mut client = ConnectedClient::default();
    while let Some(result) = rx.next().await {
        match result {
            Ok(message) => match bincode::deserialize::<ServerRequest>(message.as_bytes()) {
                Ok(request) => {
                    if let Err(err) = client
                        .handle_websocket_request(request, sender.clone())
                        .await
                    {
                        sender
                            .send(ServerResponse::Error {
                                message: Some(err.to_string()),
                            })
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

impl ConnectedClient {
    async fn handle_websocket_request(
        &mut self,
        request: ServerRequest,
        responder: Sender<ServerResponse>,
    ) -> Result<(), anyhow::Error> {
        match request {
            ServerRequest::Authenticate {
                installation_id,
                version,
            } => {
                self.installation_id = Some(match installation_id {
                    Some(installation_id) => installation_id,
                    None => {
                        let installation_id = Uuid::new_v4();
                        responder
                            .send(ServerResponse::AdoptInstallationId {
                                installation_id: installation_id,
                            })
                            .unwrap_or_default();
                        installation_id
                    }
                });

                let pool = pg();
                let installation = sqlx::query_as!(
                    Installation,
                    "SELECT * FROM installation_lookup($1)",
                    self.installation_id
                )
                .fetch_one(&pool)
                .await?;

                CONNECTED_CLIENTS
                    .connect(installation.id, responder.clone())
                    .await;

                if let Some(account_id) = installation.account_id {
                    let profile = sqlx::query_as!(
                        UserProfile,
                        "SELECT id, username FROM installation_profile($1)",
                        installation.id,
                    )
                    .fetch_one(&pool)
                    .await?;

                    CONNECTED_CLIENTS
                        .associate_account(installation.id, account_id)
                        .await;
                    responder
                        .send(ServerResponse::Authenticated { profile })
                        .unwrap_or_default();
                }
                Ok(())
            }
            ServerRequest::AuthenticationUrl => {
                if let Some(installation_id) = self.installation_id {
                    responder
                        .send(ServerResponse::AuthenticateAtUrl {
                            url: itchio_authorization_url(installation_id),
                        })
                        .unwrap_or_default();
                }
                Ok(())
            }
        }
    }
}

impl Drop for ConnectedClient {
    fn drop(&mut self) {
        if let Some(installation_id) = self.installation_id {
            block_on(CONNECTED_CLIENTS.disconnect(installation_id));
        }
    }
}

static REDIRECT_URI: &'static str = "http://localhost:7878/auth/itchio_callback";

fn itchio_authorization_url(installation_id: Uuid) -> String {
    Url::parse_with_params(
        "https://itch.io/user/oauth",
        &[
            ("client_id", env("OAUTH_CLIENT_ID")),
            ("scope", "profile:me".to_owned()),
            ("response_type", "token".to_owned()),
            ("redirect_uri", REDIRECT_URI.to_owned()),
            ("state", installation_id.to_string()),
        ],
    )
    .unwrap()
    .to_string()
}
