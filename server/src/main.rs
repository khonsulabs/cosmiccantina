use futures::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use shared::{ServerRequest, ServerResponse};
use std::collections::HashMap;
use tera::Tera;
use url::Url;
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

    let websockets = warp::path!("ws")
        .and(warp::path::end())
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| ws.on_upgrade(|websocket| websocket_main(websocket)));
    let client_authorize = warp::path!("auth" / "client").map(|| oauth_client_authenticate());
    let itchio_callback = warp::path!("auth" / "itchio_callback").map(|| itchio_callback());
    let receive_token = warp::path!("auth" / "receive_token")
        .and(warp::body::form())
        .map(|body: HashMap<String, String>| receive_token(&body["state"], &body["access_token"]));
    let oauth = client_authorize.or(itchio_callback).or(receive_token);
    let routes = websockets
        .or(oauth)
        .or(warp::any().map(|| warp::reply::with_status("Not Found", StatusCode::NOT_FOUND)));

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
        ServerRequest::Authenticate { previous_token } => {
            match previous_token {
                Some(token) => todo!("Lookup in database"),
                None => todo!("Generate new token and send it"),
            }
            // TODO send auth url
        }
    }
}

static REDIRECT_URI: &'static str = "http://localhost:7878/auth/itchio_callback";

fn oauth_client_authenticate() -> impl warp::reply::Reply {
    let authorize_url = Url::parse_with_params(
        "https://itch.io/user/oauth",
        &[
            ("client_id", env("OAUTH_CLIENT_ID")),
            ("scope", "profile:me".to_owned()),
            ("response_type", "token".to_owned()),
            ("redirect_uri", REDIRECT_URI.to_owned()),
            ("state", "asdf".to_owned()),
        ],
    )
    .unwrap();
    warp::reply::with_header(
        StatusCode::TEMPORARY_REDIRECT,
        header::LOCATION,
        authorize_url.as_str(),
    )
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
