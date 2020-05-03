use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerRequest {
    Authenticate { previous_token: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerResponse {
    AdoptToken { token: String },
    AuthenticateAtUrl { url: String },
    Authenticated { profile: UserProfile },
    AuthenticationError { message: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserProfile {
    pub id: u64,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: Uuid,
    pub account_id: Option<i64>,
}
