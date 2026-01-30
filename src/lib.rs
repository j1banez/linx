use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    http::header::LOCATION,
    response::IntoResponse,
    routing::{get, post},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub const DEFAULT_CODE_LEN: usize = 6;
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

#[derive(Serialize)]
struct ShortenResponse {
    short_url: String,
    code: String,
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/shorten", post(shorten))
        .route("/{code}", get(redirect))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn shorten(
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Result<Json<ShortenResponse>, StatusCode> {
    // Handle empty values
    let code_input = payload.code.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let code = match code_input {
        Some(code) => {
            insert_link(&state.db, &code, &payload.url)
                .await
                .map_err(|err| map_insert_error(&err))?;
            code
        }
        None => generate_and_insert(&state.db, &payload.url, state.code_len).await?,
    };

    Ok(Json(ShortenResponse {
        short_url: format!("{}/{code}", state.base_url),
        code,
    }))
}

async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    let url = sqlx::query_scalar::<_, String>("SELECT url FROM link WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match url {
        Some(value) => Ok((StatusCode::MOVED_PERMANENTLY, [(LOCATION, value)]).into_response()),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn insert_link(db: &SqlitePool, code: &str, url: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO link (code, url) VALUES (?, ?)")
        .bind(code)
        .bind(url)
        .execute(db)
        .await
        .map(|_| ())
}

async fn generate_and_insert(
    db: &SqlitePool,
    url: &str,
    code_len: usize,
) -> Result<String, StatusCode> {
    // Retry multiple times in cae of collisions
    for _ in 0..5 {
        let code = generate_code(code_len);
        match insert_link(db, &code, url).await {
            Ok(()) => return Ok(code),
            Err(err) => {
                if !is_unique_violation(&err) {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    }

    Err(StatusCode::CONFLICT)
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

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.is_unique_violation(),
        _ => false,
    }
}

fn map_insert_error(err: &sqlx::Error) -> StatusCode {
    if is_unique_violation(err) {
        StatusCode::CONFLICT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
