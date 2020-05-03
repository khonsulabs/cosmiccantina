use async_std::sync::RwLock;
use crossbeam::channel::{unbounded, Sender};
use futures::{executor::block_on, SinkExt, StreamExt};
use lazy_static::lazy_static;
use migrations::{pg, sqlx};
use serde_derive::{Deserialize, Serialize};
use shared::{Installation, ServerRequest, ServerResponse, UserProfile};
use sqlx::postgres::{PgListener, PgRow};
use sqlx::{FromRow, PgPool, Row};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tera::Tera;
use url::Url;
use uuid::Uuid;
use warp::http::{header, StatusCode};
use warp::{
    filters::ws::{Message, WebSocket},
    Filter,
};

lazy_static! {
    static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        tera.add_raw_template("logged_in", include_str!("logged_in.html"))
            .unwrap();
        tera
    };
    static ref CONNECTED_CLIENTS: ConnectedClients = { ConnectedClients::default() };
}

struct ConnectedClients {
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
    async fn connect(&self, installation_id: Uuid, sender: Sender<ServerResponse>) {
        let mut senders = self.senders.write().await;
        senders.insert(installation_id, sender);
    }

    async fn associate_acccount(&self, installation_id: Uuid, account_id: i64) {
        let mut installations_by_account = self.installations_by_account.write().await;
        let mut account_by_installation = self.account_by_installation.write().await;
        account_by_installation.insert(installation_id, account_id);
        let installations = installations_by_account
            .entry(account_id)
            .or_insert_with(|| HashSet::new());
        installations.insert(installation_id);
    }

    async fn disconnect(&self, installation_id: Uuid) {
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

    async fn send_to_installation_id(&self, installation_id: Uuid, message: ServerResponse) {
        let senders = self.senders.write().await;
        if let Some(sender) = senders.get(&installation_id) {
            sender.send(message).unwrap_or_default();
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Error initializing environment");

    migrations::run_all()
        .await
        .expect("Error running migrations");

    tokio::spawn(pg_notify_loop());

    let websockets = warp::path!("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|websocket| websocket_main(websocket)));
    // let client_authorize = warp::path!("auth" / "client").map(|| oauth_client_authenticate());
    let itchio_callback = warp::path!("auth" / "itchio_callback").map(|| itchio_callback());
    let receive_token = warp::path!("auth" / "receive_token")
        .and(warp::body::form())
        .map(|body: HashMap<String, String>| {
            receive_token(&body["state"], body["access_token"].clone())
        });
    let oauth = itchio_callback.or(receive_token);
    let routes = websockets
        .or(oauth)
        .or(warp::any().map(|| warp::reply::with_status("Not Found", StatusCode::NOT_FOUND)));

    warp::serve(routes).run(([0, 0, 0, 0], 7878)).await;
}

#[derive(Default)]
struct ConnectedClient {
    installation_id: Option<Uuid>,
}

async fn websocket_main(websocket: WebSocket) {
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
            ServerRequest::Authenticate { installation_id } => {
                let installation_id = match installation_id {
                    Some(installation_id) => todo!(
                    "Lookup installation ID and figure out what state the connection should be in"
                ),
                    None => {
                        let installation_id = Uuid::new_v4();
                        let pool = pg();
                        let installation = sqlx::query_as!(
                            Installation,
                            "SELECT * FROM installation_lookup($1)",
                            installation_id
                        )
                        .fetch_one(&pool)
                        .await?;
                        responder
                            .send(ServerResponse::AdoptInstallationId {
                                installation_id: installation.id,
                            })
                            .unwrap_or_default();
                        installation.id
                    }
                };

                self.installation_id = Some(installation_id);
                CONNECTED_CLIENTS
                    .connect(installation_id, responder.clone())
                    .await;

                responder
                    .send(ServerResponse::AuthenticateAtUrl {
                        url: itchio_authorization_url(installation_id),
                    })
                    .unwrap_or_default();

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

fn itchio_callback() -> impl warp::reply::Reply {
    let mut context = tera::Context::new();
    context.insert("post_url", "http://localhost:7878/auth/receive_token");

    warp::reply::with_header(
        TEMPLATES.render("logged_in", &context).unwrap(),
        header::CONTENT_TYPE,
        "text/html; charset=UTF-8",
    )
}

fn receive_token(state: &str, access_token: String) -> impl warp::reply::Reply {
    let installation_id = match Uuid::parse_str(state) {
        Ok(uuid) => uuid,
        Err(_) => {
            println!("Invalid UUID in state");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };
    tokio::spawn(async move {
        login_itchio(installation_id, access_token)
            .await
            .expect("Error logging into itchio")
    });
    StatusCode::OK
}

#[derive(Debug, Serialize, Deserialize)]
struct ItchioProfile {
    pub cover_url: Option<String>,
    pub display_name: Option<String>,
    pub username: String,
    pub id: i64,
    pub developer: bool,
    pub gamer: bool,
    pub press_user: bool,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ItchioProfileResponse {
    pub user: ItchioProfile,
}

async fn login_itchio(installation_id: Uuid, access_token: String) -> Result<(), anyhow::Error> {
    // Call itch.io API to get the user information
    let client = reqwest::Client::new();
    let response: ItchioProfileResponse = client
        .get("https://itch.io/api/1/key/me")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", access_token),
        )
        .send()
        .await?
        .json()
        .await?;

    let pg = pg();
    let mut tx = pg.begin().await?;
    let account = sqlx::query!(
        "SELECT account_lookup($1, $2) as account_id",
        response.user.id,
        response.user.username
    )
    .fetch_one(&mut tx)
    .await
    .expect("Function should always return a value");

    sqlx::query!(
        "SELECT installation_login($1, $2, $3) as rows_changed",
        installation_id,
        account.account_id,
        access_token,
    )
    .fetch_one(&mut tx)
    .await?;
    tx.commit().await?;

    Ok(())
}

fn env(var: &str) -> String {
    std::env::var(var).unwrap()
}

async fn pg_notify_loop() -> Result<(), anyhow::Error> {
    let pool = pg();
    let mut listener = PgListener::from_pool(&pool).await?;
    listener.listen_all(vec!["installation_login"]).await?;
    while let Ok(notification) = listener.recv().await {
        if notification.channel() == "installation_login" {
            // The payload is the installation_id that logged in.
            let installation_id = Uuid::parse_str(notification.payload())?;
            let profile = sqlx::query_as!(
                UserProfile,
                "SELECT id, username FROM installation_profile($1)",
                installation_id,
            )
            .fetch_one(&pool)
            .await?;

            CONNECTED_CLIENTS
                .send_to_installation_id(installation_id, ServerResponse::Authenticated { profile })
                .await;
        }
    }
    panic!("Error on postgres listening");
}
