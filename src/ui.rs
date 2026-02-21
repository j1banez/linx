use crate::AppState;
use crate::error::AppError;
use crate::sql;
use crate::value::{Code, CodeError, ValidUrl};
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

#[derive(Debug, Clone, Copy)]
struct Pagination {
    page: i64,
    total_pages: i64,
}

impl Pagination {
    fn new(total: i64, page_size: i64, requested_page: Option<i64>) -> Self {
        let total_pages = if total == 0 {
            1
        } else {
            // Ceil division so partial pages still count.
            (total + page_size - 1) / page_size
        };

        let mut page = requested_page.unwrap_or(1);

        if page < 1 {
            page = 1;
        }

        if page > total_pages {
            page = total_pages;
        }

        Self { page, total_pages }
    }

    fn has_prev(self) -> bool {
        self.page > 1
    }

    fn has_next(self) -> bool {
        self.page < self.total_pages
    }

    fn prev_page(self) -> i64 {
        if self.page > 1 { self.page - 1 } else { 1 }
    }

    fn next_page(self) -> i64 {
        if self.page < self.total_pages {
            self.page + 1
        } else {
            self.total_pages
        }
    }

    fn offset(self, page_size: i64) -> i64 {
        (self.page - 1) * page_size
    }
}

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

    let requested_page = query
        .page
        .as_deref()
        .and_then(|value| value.parse::<i64>().ok());

    let pagination = Pagination::new(total, HOME_PAGE_SIZE, requested_page);

    let offset = pagination.offset(HOME_PAGE_SIZE);

    let rows = sql::list_links(&state.db, HOME_PAGE_SIZE, offset)
        .await
        .map_err(|err| {
            tracing::error!(error = ?err, "home_page list_links failed");
            AppError::Internal
        })?;

    let links = rows
        .into_iter()
        .map(|(code, url, clicks)| LinkItem {
            stats_url: format!("/{code}/stats"),
            code,
            url,
            clicks,
        })
        .collect();

    let tpl = HomeTemplate {
        msg: None,
        links,
        page: pagination.page,
        total_pages: pagination.total_pages,
        has_prev: pagination.has_prev(),
        has_next: pagination.has_next(),
        prev_page: pagination.prev_page(),
        next_page: pagination.next_page(),
    };

    let html = tpl.render().map_err(|_| AppError::Internal)?;

    Ok(Html(html))
}

#[instrument(skip(state))]
async fn home_submit(
    State(state): State<AppState>,
    Form(form): Form<ShortenForm>,
) -> Result<impl IntoResponse, AppError> {
    let url = ValidUrl::try_from(form.url)?;

    let code = match form.code {
        Some(raw) => match Code::try_from(raw) {
            Ok(code) => {
                sql::insert_link(&state.db, &code, &url).await?;
                code
            }
            Err(CodeError::Empty) => {
                sql::generate_and_insert(&state.db, &url, state.code_len).await?
            }
            Err(err) => return Err(err.into()),
        },
        None => sql::generate_and_insert(&state.db, &url, state.code_len).await?,
    };

    Ok(Redirect::to(&format!("/{code}/stats")))
}

#[instrument(skip(state))]
async fn stats_page(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let Ok(code) = Code::try_from(code) else {
        let tpl = NotFoundTemplate {
            message: "Code not found.",
        };
        let html = tpl.render().unwrap_or_else(|_| "Not found".to_string());
        return Ok((StatusCode::NOT_FOUND, Html(html)).into_response());
    };

    let row = match sql::fetch_link_stats(&state.db, &code).await {
        Ok(row) => row,
        Err(err) => {
            tracing::error!(%code, error = ?err, "stats_page query failed");
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("internal error".to_string()),
            )
                .into_response());
        }
    };

    let Some((url, clicks, created_at, last_accessed_at)) = row else {
        let tpl = NotFoundTemplate {
            message: "Code not found.",
        };
        let html = tpl.render().unwrap_or_else(|_| "Not found".to_string());

        return Ok((StatusCode::NOT_FOUND, Html(html)).into_response());
    };

    let code = code.to_string();

    let tpl = StatsTemplate {
        url,
        short_url: format!("{}/{}", state.base_url, code),
        api_stats_url: format!("/api/{code}/stats"),
        code,
        clicks,
        created_at,
        last_accessed_at: last_accessed_at.map(|v| v.to_string()),
    };

    match tpl.render() {
        Ok(html) => Ok(Html(html).into_response()),
        Err(err) => {
            tracing::error!(error = ?err, "stats_page render failed");
            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("internal error".to_string()),
            )
                .into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pagination;

    #[test]
    fn pagination_total_zero_defaults_to_page_one() {
        let p = Pagination::new(0, 50, None);
        assert_eq!(p.page, 1);
        assert_eq!(p.total_pages, 1);
        assert!(!p.has_prev());
        assert!(!p.has_next());
        assert_eq!(p.prev_page(), 1);
        assert_eq!(p.next_page(), 1);
        assert_eq!(p.offset(50), 0);
    }

    #[test]
    fn pagination_clamps_page_below_one() {
        let p = Pagination::new(120, 50, Some(0));
        assert_eq!(p.page, 1);
        assert_eq!(p.total_pages, 3);
        assert!(!p.has_prev());
        assert!(p.has_next());
        assert_eq!(p.prev_page(), 1);
        assert_eq!(p.next_page(), 2);
    }

    #[test]
    fn pagination_keeps_requested_page_in_range() {
        let p = Pagination::new(120, 50, Some(2));
        assert_eq!(p.page, 2);
        assert_eq!(p.total_pages, 3);
        assert!(p.has_prev());
        assert!(p.has_next());
        assert_eq!(p.prev_page(), 1);
        assert_eq!(p.next_page(), 3);
        assert_eq!(p.offset(50), 50);
    }

    #[test]
    fn pagination_clamps_page_above_max() {
        let p = Pagination::new(120, 50, Some(99));
        assert_eq!(p.page, 3);
        assert_eq!(p.total_pages, 3);
        assert!(p.has_prev());
        assert!(!p.has_next());
        assert_eq!(p.prev_page(), 2);
        assert_eq!(p.next_page(), 3);
        assert_eq!(p.offset(50), 100);
    }
}
