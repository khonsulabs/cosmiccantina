use crate::websockets::CONNECTED_ACCOUNTS;
use migrations::{
    pg,
    sqlx::{self, prelude::*},
};
use shared::{current_timestamp, WALK_SPEED};
use std::time::Duration;

pub async fn run() -> Result<(), anyhow::Error> {
    let mut last_update = None;
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;

        let now = current_timestamp();
        for connected_account_handle in CONNECTED_ACCOUNTS.all().await {
            let mut connected_account = connected_account_handle.write().await;
            println!("Walking {:.3}", connected_account.profile.horizontal_input);
            let result = sqlx::query!(
                "SELECT * FROM account_update_walk($1, $2, $3, $4)",
                connected_account.profile.id,
                now,
                last_update.unwrap_or(now),
                WALK_SPEED
            )
            .fetch_one(&pg())
            .await?;

            connected_account.profile.x_offset = result.x_offset.unwrap();
            connected_account.profile.last_update_timestamp = Some(now);
        }

        let pool = pg();
        let mut connection = pool.acquire().await?;
        connection
            .execute(&*format!("NOTIFY world_update, '{:.15}'", now))
            .await?;

        last_update = Some(now);
    }
}
// POSTGRES leader election:
//   table: workers
//     id, last_seen, is_leader
//   am_i_leader(id) -> bool (does the leader switch if last_seen of the current leader is too old)
//   call it before saving and before executing
