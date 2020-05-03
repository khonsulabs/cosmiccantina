use serde_derive::{Deserialize, Serialize};

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
