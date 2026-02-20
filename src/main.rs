use clap::Parser;
use linx::{
    AppState, DEFAULT_REDIRECT_CACHE_CAPACITY, build_app, sql,
    value::{DEFAULT_CODE_LEN, MAX_CODE_LEN, MIN_CODE_LEN},
};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, fmt::time};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, env = "LINX_URL", default_value = "http://127.0.0.1:3000")]
    app_url: String,
    #[command(flatten)]
    db: sql::DbArgs,
    #[arg(long, env = "CODE_LEN", default_value_t = DEFAULT_CODE_LEN)]
    code_len: usize,
    #[arg(
        long,
        env = "REDIRECT_CACHE_CAPACITY",
        default_value_t = DEFAULT_REDIRECT_CACHE_CAPACITY
    )]
    redirect_cache_capacity: usize,
    #[arg(long, env = "PORT", default_value_t = 3000)]
    port: u16,
}

fn init_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .with_line_number(true)
        .with_thread_names(true)
        .with_timer(time::uptime())
        .with_span_events(FmtSpan::NEW)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );

    if cfg!(debug_assertions) {
        subscriber.pretty().init();
    } else {
        subscriber.json().init();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let cli = Cli::parse();
    let base_url = cli.app_url.trim_end_matches('/').to_string();

    if cli.code_len < MIN_CODE_LEN || cli.code_len > MAX_CODE_LEN {
        tracing::error!(
            cli.code_len,
            min = MIN_CODE_LEN,
            max = MAX_CODE_LEN,
            "invalid CODE_LEN"
        );
        std::process::exit(2);
    }

    let pool = sql::setup_pool(&cli.db).await?;
    let state = AppState::new(base_url, pool, cli.code_len, cli.redirect_cache_capacity);
    let shutdown_state = state.clone();
    let app = build_app(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", cli.port)).await?;

    tracing::info!("listening on http://{}", listener.local_addr()?);

    let serve_result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutdown signal received");
        })
        .await;

    shutdown_state.flush_pending_stats().await;

    serve_result?;
    Ok(())
}
