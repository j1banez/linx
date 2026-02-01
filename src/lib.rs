use crate::error::AppError;
use crate::response::AppResponse;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use rand::Rng;
use serde::Deserialize;
use sqlx::SqlitePool;
use tracing::instrument;

mod error;
mod response;

pub const DEFAULT_CODE_LEN: usize = 6;
pub const MIN_CODE_LEN: usize = 4;
pub const MAX_CODE_LEN: usize = 32;
const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Clone)]
pub struct AppState {
    base_url: String,
    db: SqlitePool,
    code_len: usize,
}

impl AppState {
    pub fn new(base_url: String, db: SqlitePool, code_len: usize) -> Self {
        Self {
            base_url,
            db,
            code_len,
        }
    }
}

#[derive(Deserialize, Debug)]
struct ShortenRequest {
    url: String,
    code: Option<String>,
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/{code}", get(redirect))
        .route("/{code}/stats", get(stats))
        .with_state(state)
}

async fn health() -> AppResponse {
    AppResponse::Health
}

#[instrument(skip(state))]
async fn shorten(
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Result<AppResponse, AppError> {
    // Handle empty values
    let code_input = payload
        .code
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let code = match code_input {
        Some(code) => {
            if code.len() > MAX_CODE_LEN {
                return Err(AppError::BadRequest(format!(
                    "code too long (max {MAX_CODE_LEN})"
                )));
            }
            if !is_base62(&code) {
                return Err(AppError::BadRequest("invalid code (base62 only)".into()));
            }

            insert_link(&state.db, &code, &payload.url).await?;

            code
        }
        None => generate_and_insert(&state.db, &payload.url, state.code_len).await?,
    };

    Ok(AppResponse::Shorten(
        format!("{}/{code}", state.base_url),
        code,
    ))
}

#[instrument(skip(state))]
async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<AppResponse, AppError> {
    let url = sqlx::query_scalar::<_, String>("SELECT url FROM link WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.db)
        .await?;

    match url {
        Some(to) => {
            let db = state.db.clone();
            let code = code.clone();

            tokio::spawn(async move {
                if let Err(err) = bump_link_stats(&db, &code).await {
                    tracing::error!(%code, error = ?err, "failed to bump link stats");
                }
            });

            Ok(AppResponse::Redirect(to))
        }
        None => Err(AppError::NotFound(None)),
    }
}

#[instrument(skip(state))]
async fn stats(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<AppResponse, AppError> {
    let row = sqlx::query_as::<_, (String, i64, i64, Option<i64>)>(
        "SELECT url, clicks, created_at, last_accessed_at
         FROM link
         WHERE code = ?",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some((url, clicks, created_at, last_accessed_at)) => Ok(AppResponse::new_stats(
            code,
            url,
            clicks,
            created_at,
            last_accessed_at,
        )),
        None => Err(AppError::NotFound(Some("code".into()))),
    }
}

async fn insert_link(db: &SqlitePool, code: &str, url: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind(code)
        .bind(url)
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

async fn bump_link_stats(db: &SqlitePool, code: &str) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE link
         SET clicks = clicks + 1,
             last_accessed_at = unixepoch()
         WHERE code = ?",
    )
    .bind(code)
    .execute(db)
    .await?;

    Ok(())
}

async fn generate_and_insert(
    db: &SqlitePool,
    url: &str,
    code_len: usize,
) -> Result<String, AppError> {
    // Retry multiple times in case of collisions
    for _ in 0..5 {
        let code = generate_code(code_len);
        match insert_link(db, &code, url).await {
            Ok(()) => return Ok(code),
            Err(AppError::Internal) => return Err(AppError::Internal),
            Err(_) => continue,
        }
    }

    Err(AppError::Conflict(
        "unable to generate unique code".to_string(),
    ))
}

fn generate_code(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(length);

    for _ in 0..length {
        let index = rng.gen_range(0..BASE62.len());
        code.push(BASE62[index] as char);
    }

    code
}

fn is_base62(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| BASE62.contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_base62_accepts_alphanumeric() {
        assert!(is_base62("abcXYZ012"));
        assert!(is_base62("0"));
        assert!(is_base62("Z"));
        assert!(is_base62("z"));
    }

    #[test]
    fn is_base62_rejects_non_base62_chars() {
        assert!(!is_base62(""));
        assert!(!is_base62("hello-world"));
        assert!(!is_base62("hello_world"));
        assert!(!is_base62("hello world"));
        assert!(!is_base62("é"));
        assert!(!is_base62("/"));
        assert!(!is_base62("%2F"));
        assert!(!is_base62("?"));
        assert!(!is_base62("!"));
    }

    #[test]
    fn generate_code_has_correct_length_and_charset() {
        for len in [1usize, 2, 6, 12, 32] {
            let code = generate_code(len);
            assert_eq!(code.len(), len);
            assert!(code.bytes().all(|b| BASE62.contains(&b)));
        }
    }
}
