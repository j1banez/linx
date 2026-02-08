use crate::AppState;
use crate::error::AppError;
use crate::sql;
use crate::validate;
use askama::Template;
use axum::{
    Router,
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::get,
};
use serde::Deserialize;
use tracing::instrument;

const HOME_PAGE_SIZE: i64 = 50;

#[derive(Debug, Deserialize)]
struct ShortenForm {
    url: String,
    code: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct HomeQuery {
    page: Option<String>,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate<'a> {
    msg: Option<&'a str>,
    links: Vec<LinkItem>,
    page: i64,
    total_pages: i64,
    has_prev: bool,
    has_next: bool,
    prev_page: i64,
    next_page: i64,
}

struct LinkItem {
    code: String,
    url: String,
    stats_url: String,
    clicks: i64,
}

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    code: String,
    url: String,
    short_url: String,
    api_stats_url: String,
    clicks: i64,
    created_at: i64,
    last_accessed_at: Option<String>,
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate<'a> {
    message: &'a str,
}

pub fn ui_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(home_page).post(home_submit))
        .route("/{code}/stats", get(stats_page))
}

#[instrument(skip(state))]
async fn home_page(
    State(state): State<AppState>,
    Query(query): Query<HomeQuery>,
) -> Result<Html<String>, AppError> {
    let total = sql::count_links(&state.db).await.map_err(|err| {
        tracing::error!(error = ?err, "home_page count_links failed");
        AppError::Internal
    })?;

    let total_pages = if total == 0 {
        1
    } else {
        // Ceil division so partial pages still count.
        (total + HOME_PAGE_SIZE - 1) / HOME_PAGE_SIZE
    };

    let mut page = query
        .page
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(1);

    if page < 1 {
        page = 1;
    }

    if page > total_pages {
        page = total_pages;
    }

    let offset = (page - 1) * HOME_PAGE_SIZE;

    let rows = sql::list_links(&state.db, HOME_PAGE_SIZE, offset)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "home_page list_links failed");
            AppError::Internal
        })?;

    let links = rows
        .into_iter()
        .map(|(code, url, clicks)| LinkItem {
            stats_url: format!("/{}/stats", code),
            code,
            url,
            clicks,
        })
        .collect();

    let tpl = HomeTemplate {
        msg: None,
        links,
        page,
        total_pages,
        has_prev: page > 1,
        has_next: page < total_pages,
        prev_page: if page > 1 { page - 1 } else { 1 },
        next_page: if page < total_pages {
            page + 1
        } else {
            total_pages
        },
    };

    let html = tpl.render().map_err(|_| AppError::Internal)?;

    Ok(Html(html))
}

#[instrument(skip(state))]
async fn home_submit(
    State(state): State<AppState>,
    Form(form): Form<ShortenForm>,
) -> Result<impl IntoResponse, AppError> {
    let url = validate::validate_and_normalize_url(&form.url)?;

    let code = match form.code {
        Some(code) if !code.trim().is_empty() => {
            let code = validate::validate_and_normalize_code(&code)?;
            sql::insert_link(&state.db, &code, &url).await?;
            code
        }
        _ => sql::generate_and_insert(&state.db, &url, state.code_len).await?,
    };

    Ok(Redirect::to(&format!("/{}/stats", code)))
}

#[instrument(skip(state))]
async fn stats_page(State(state): State<AppState>, Path(code): Path<String>) -> impl IntoResponse {
    let row = match sql::fetch_link_stats(&state.db, &code).await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!(%code, error = ?err, "stats_page query failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("internal error".to_string()),
            )
                .into_response();
        }
    };

    let Some((url, clicks, created_at, last_accessed_at)) = row else {
        let tpl = NotFoundTemplate {
            message: "Code not found.",
        };
        let html = tpl.render().unwrap_or_else(|_| "Not found".to_string());

        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    };

    let tpl = StatsTemplate {
        url,
        short_url: format!("{}/{}", state.base_url, code),
        api_stats_url: format!("/api/{}/stats", code),
        code,
        clicks,
        created_at,
        last_accessed_at: last_accessed_at.map(|v| v.to_string()),
    };

    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = ?err, "stats_page render failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("internal error".to_string()),
            )
                .into_response()
        }
    }
}
