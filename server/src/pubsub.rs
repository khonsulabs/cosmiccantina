use super::websockets::CONNECTED_CLIENTS;
use migrations::{pg, sqlx};
use shared::{ServerResponse, UserProfile};
use sqlx::postgres::PgListener;
use uuid::Uuid;

pub async fn pg_notify_loop() -> Result<(), anyhow::Error> {
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
                .associate_account(installation_id, profile.id)
                .await;

            CONNECTED_CLIENTS
                .send_to_installation_id(installation_id, ServerResponse::Authenticated { profile })
                .await;
        }
    }
    panic!("Error on postgres listening");
}
