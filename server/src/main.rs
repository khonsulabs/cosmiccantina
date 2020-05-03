use crossbeam::channel::{unbounded, Sender};
use futures::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use migrations::{pg, sqlx};
use shared::{Installation, ServerRequest, ServerResponse};
use std::collections::HashMap;
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
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Error initializing environment");

    migrations::run_all()
        .await
        .expect("Error running migrations");

    let websockets = warp::path!("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|websocket| websocket_main(websocket)));
    // let client_authorize = warp::path!("auth" / "client").map(|| oauth_client_authenticate());
    let itchio_callback = warp::path!("auth" / "itchio_callback").map(|| itchio_callback());
    let receive_token = warp::path!("auth" / "receive_token")
        .and(warp::body::form())
        .map(|body: HashMap<String, String>| receive_token(&body["state"], &body["access_token"]));
    let oauth = itchio_callback.or(receive_token);
    let routes = websockets
        .or(oauth)
        .or(warp::any().map(|| warp::reply::with_status("Not Found", StatusCode::NOT_FOUND)));

    warp::serve(routes).run(([0, 0, 0, 0], 7878)).await;
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

    while let Some(result) = rx.next().await {
        match result {
            Ok(message) => match bincode::deserialize::<ServerRequest>(message.as_bytes()) {
                Ok(request) => {
                    if let Err(err) = handle_websocket_request(request, sender.clone()).await {
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

async fn handle_websocket_request(
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

            responder
                .send(ServerResponse::AuthenticateAtUrl {
                    url: itchio_authorization_url(installation_id),
                })
                .unwrap_or_default();

            Ok(())
        }
    }
}

static REDIRECT_URI: &'static str = "http://localhost:7878/auth/itchio_callback";

// fn oauth_client_authenticate() -> impl warp::reply::Reply {
//     warp::reply::with_header(
//         StatusCode::TEMPORARY_REDIRECT,
//         header::LOCATION,
//         &itchio_authorization_url(installation_id),
//     )
// }

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

fn receive_token(state: &str, access_token: &str) -> impl warp::reply::Reply {
    println!("{}, {}", state, access_token);
    StatusCode::INTERNAL_SERVER_ERROR
}

fn env(var: &str) -> String {
    std::env::var(var).unwrap()
}
