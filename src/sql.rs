use crate::error::AppError;
use crate::value::{BASE62, Code, CodeError, ValidUrl};
use clap::Args;
use rand::Rng;
use rand::rngs::OsRng;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

#[derive(Args, Debug, Clone)]
pub struct DbArgs {
    #[arg(long, env = "DATABASE_URL", default_value = "sqlite://./linx.db")]
    pub database_url: String,
    #[arg(long, env = "DATABASE_MAX_CONNECTIONS", default_value_t = 10)]
    pub max_connections: u32,
}

pub async fn setup_pool(db: &DbArgs) -> Result<SqlitePool, std::io::Error> {
    let options = SqliteConnectOptions::from_str(&db.database_url)
        .map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid DATABASE_URL format: {} ({err})", db.database_url),
            )
        })?
        .create_if_missing(true)
        .pragma("journal_mode", "WAL")
        .pragma("synchronous", "NORMAL")
        .pragma("busy_timeout", "2000")
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(db.max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|err| {
            std::io::Error::other(format!(
                "Failed to open SQLite database `{}`: {err}",
                db.database_url
            ))
        })?;

    sqlx::migrate!().run(&pool).await.map_err(|err| {
        std::io::Error::other(format!("Failed to run database migrations: {err}"))
    })?;

    Ok(pool)
}

pub async fn insert_link(db: &SqlitePool, code: &Code, url: &ValidUrl) -> Result<(), AppError> {
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind(code.as_str())
        .bind(url.as_str())
        .execute(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
                AppError::Conflict("code already exists".to_string())
            }
            _ => AppError::Internal,
        })?;

    Ok(())
}

pub async fn fetch_link_url(db: &SqlitePool, code: &Code) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT url FROM link WHERE code = ?")
        .bind(code.as_str())
        .fetch_optional(db)
        .await
}

pub async fn fetch_link_stats(
    db: &SqlitePool,
    code: &Code,
) -> Result<Option<(String, i64, i64, Option<i64>)>, sqlx::Error> {
    sqlx::query_as::<_, (String, i64, i64, Option<i64>)>(
        "SELECT url, clicks, created_at, last_accessed_at
         FROM link
         WHERE code = ?",
    )
    .bind(code.as_str())
    .fetch_optional(db)
    .await
}

pub async fn list_links(
    db: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<(String, String, i64)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String, i64)>(
        "SELECT code, url, clicks
         FROM link
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count_links(db: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM link")
        .fetch_one(db)
        .await
}

pub async fn bump_link_stats_by(db: &SqlitePool, code: &Code, count: i64) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE link
         SET clicks = clicks + ?,
             last_accessed_at = unixepoch()
         WHERE code = ?",
    )
    .bind(count)
    .bind(code.as_str())
    .execute(db)
    .await?;

    Ok(())
}

pub async fn generate_and_insert(
    db: &SqlitePool,
    url: &ValidUrl,
    code_len: usize,
) -> Result<Code, AppError> {
    // Retry multiple times in case of collisions
    for _ in 0..5 {
        let code = generate_code(code_len)?;
        match insert_link(db, &code, url).await {
            Ok(()) => return Ok(code),
            Err(AppError::Internal) => return Err(AppError::Internal),
            Err(_) => {}
        }
    }

    Err(AppError::Conflict(
        "unable to generate unique code".to_string(),
    ))
}

pub fn generate_code(length: usize) -> Result<Code, CodeError> {
    let mut rng = OsRng;
    let mut code = String::with_capacity(length);

    for _ in 0..length {
        let index = rng.gen_range(0..BASE62.len());

        if let Some(&b62) = BASE62.get(index) {
            code.push(b62 as char);
        }
    }

    Code::try_from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_code_has_correct_length() {
        for len in [1usize, 2, 6, 12, 32] {
            let code = generate_code(len).expect("generated code should be valid");
            assert_eq!(code.as_str().len(), len);
        }
    }
}
