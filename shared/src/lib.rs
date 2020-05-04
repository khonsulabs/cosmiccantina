use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROTOCOL_VERSION: &'static str = "0.0.1";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerRequest {
    Authenticate {
        version: String,
        installation_id: Option<Uuid>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerResponse {
    AdoptInstallationId { installation_id: Uuid },
    AuthenticateAtUrl { url: String },
    Authenticated { profile: UserProfile },
    Error { message: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: Uuid,
    pub account_id: Option<i64>,
}
