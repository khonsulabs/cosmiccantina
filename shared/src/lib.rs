use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerRequest {
    Ping(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerResponse {
    Pong(String),
}
