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

mod error;
mod response;

pub const DEFAULT_CODE_LEN: usize = 6;
const MAX_CODE_LEN: usize = 32;
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

#[derive(Deserialize)]
struct ShortenRequest {
    url: String,
    code: Option<String>,
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/{code}", get(redirect))
        .with_state(state)
}

async fn health() -> AppResponse {
    AppResponse::Health
}

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

async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<AppResponse, AppError> {
    let url = sqlx::query_scalar::<_, String>("SELECT url FROM link WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| AppError::Internal)?;

    match url {
        Some(to) => Ok(AppResponse::Redirect(to)),
        None => Err(AppError::NotFound(None)),
    }
}

async fn insert_link(db: &SqlitePool, code: &str, url: &str) -> Result<(), AppError> {
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind(code)
        .bind(url)
        .execute(db)
        .await
        .map(|_| ())
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() {
                    AppError::Conflict("code already exists".to_string());
                }
                AppError::Internal
            }
            _ => AppError::Internal,
        })
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
    s.bytes().all(|b| BASE62.contains(&b))
}
