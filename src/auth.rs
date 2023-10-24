use super::{db_ops, db_ops::DbModel, models, pw, session};
use anyhow::{bail, Result};
use sqlx::{postgres::PgPool, query_as};

pub async fn authenticate(
    db: &PgPool,
    username_or_email: &str,
    password: &str,
) -> Result<session::Session> {
    let user = models::User::get(
        db,
        &db_ops::GetUserQuery {
            identifier: username_or_email,
        },
    )
    .await?;
    let truth = query_as!(
        pw::HashedPw,
        "SELECT salt, digest FROM users WHERE id = $1",
        user.id
    )
    .fetch_one(db)
    .await?;

    if pw::check(password, &truth).is_ok() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        Ok(session::Session {
            user,
            created_at: now,
        })
    } else {
        bail!("wrong password")
    }
}
