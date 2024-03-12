use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use axum::http::Method;
use fedimint_core::api::InviteCode;
use router::ws::websocket_handler;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{debug, info};

mod config;
mod error;
mod router;
mod state;
mod utils;

use axum::routing::{get, post};
use axum::Router;
use axum_otel_metrics::HttpMetricsLayerBuilder;
use clap::{Parser, Subcommand, ValueEnum};
use router::handlers::*;
use state::AppState;
// use tower_http::cors::{Any, CorsLayer};
use tower_http::validate_request::ValidateRequestHeaderLayer;

#[derive(Clone, Debug, ValueEnum)]
enum Mode {
    Fedimint,
    Cashu,
    Ws,
    Default,
}

#[derive(Subcommand)]
enum Commands {
    Start,
    Stop,
}

#[derive(Parser)]
#[clap(version = "1.0", author = "Kody Low")]
struct Cli {
    /// Federation invite code
    #[clap(long, env = "FEDERATION_INVITE_CODE", required = false)]
    federation_invite_code: String,

    /// Path to FM database
    #[clap(long, env = "FM_DB_PATH", required = true)]
    fm_db_path: PathBuf,

    /// Password
    #[clap(long, env = "PASSWORD", required = true)]
    password: String,

    /// Domain
    #[clap(long, env = "DOMAIN", required = true)]
    domain: String,

    /// Port
    #[clap(long, env = "PORT", default_value_t = 3001)]
    port: u16,

    /// Mode of operation
    #[clap(long, default_value = "default")]
    mode: Mode,
}

// const PID_FILE: &str = "/tmp/fedimint_http.pid";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    let cli: Cli = Cli::parse();

    let mut state = AppState::new(cli.fm_db_path).await?;
    debug!("AppState initialized");

    match InviteCode::from_str(&cli.federation_invite_code) {
        Ok(invite_code) => {
            let federation_id = state.multimint.register_new(invite_code, true).await?;
            info!("Created client for federation id: {:?}", federation_id);
            debug!("Federation invite code processed successfully");
        }
        Err(e) => {
            info!(
                "No federation invite code provided, skipping client creation: {}",
                e
            );
            debug!(
                "Skipping federation invite code processing due to error: {}",
                e
            );
        }
    }

    debug!("Setting up router based on mode: {:?}", cli.mode);
    let app = match cli.mode {
        Mode::Fedimint => {
            debug!("Configuring Fedimint mode");
            Router::new()
                .nest("/fedimint/v2", fedimint_v2_rest())
                .with_state(state)
                .layer(ValidateRequestHeaderLayer::bearer(&cli.password))
        }
        Mode::Cashu => {
            debug!("Configuring Cashu mode");
            Router::new()
                .nest("/cashu/v1", cashu_v1_rest())
                .with_state(state)
                .layer(ValidateRequestHeaderLayer::bearer(&cli.password))
        }
        Mode::Ws => {
            debug!("Configuring WebSocket mode");
            Router::new()
                .route("/fedimint/v2/ws", get(websocket_handler))
                .with_state(state)
                .layer(ValidateRequestHeaderLayer::bearer(&cli.password))
        }
        Mode::Default => {
            debug!("Configuring default router");
            create_default_router(state, &cli.password).await?
        }
    };

    debug!("Configuring CORS");
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    debug!("Configuring metrics");
    let metrics = HttpMetricsLayerBuilder::new()
        .with_service_name("fedimint-http".to_string())
        .build();

    debug!("Finalizing app configuration");
    let app = app
        .route("/", get(handle_readme))
        .route("/health", get(handle_status))
        .merge(metrics.routes())
        .layer(metrics)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", &cli.domain, &cli.port))
        .await
        .unwrap();
    info!("fedimint-http Listening on {}", &cli.port);
    debug!("Server starting on {}:{}", &cli.domain, &cli.port);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub async fn create_default_router(state: AppState, password: &str) -> Result<Router> {
    debug!("Creating default router with password protection");
    let app = Router::new()
        .route("/fedimint/v2/ws", get(websocket_handler))
        .nest("/fedimint/v2", fedimint_v2_rest())
        .nest("/cashu/v1", cashu_v1_rest())
        .with_state(state)
        .layer(ValidateRequestHeaderLayer::bearer(password));

    Ok(app)
}

fn fedimint_v2_rest() -> Router<AppState> {
    debug!("Configuring Fedimint V2 REST API");
    let mint_router = Router::new()
        .route("/reissue", post(fedimint::mint::reissue::handle_rest))
        .route("/spend", post(fedimint::mint::spend::handle_rest))
        .route("/validate", post(fedimint::mint::validate::handle_rest))
        .route("/split", post(fedimint::mint::split::handle_rest))
        .route("/combine", post(fedimint::mint::combine::handle_rest));

    let ln_router = Router::new()
        .route("/invoice", post(fedimint::ln::invoice::handle_rest))
        .route(
            "/await-invoice",
            post(fedimint::ln::await_invoice::handle_rest),
        )
        .route("/pay", post(fedimint::ln::pay::handle_rest))
        .route("/await-pay", post(fedimint::ln::await_pay::handle_rest))
        .route(
            "/list-gateways",
            post(fedimint::ln::list_gateways::handle_rest),
        )
        .route(
            "/switch-gateway",
            post(fedimint::ln::switch_gateway::handle_rest),
        );

    let wallet_router = Router::new()
        .route(
            "/deposit-address",
            post(fedimint::wallet::deposit_address::handle_rest),
        )
        .route(
            "/await-deposit",
            post(fedimint::wallet::await_deposit::handle_rest),
        )
        .route("/withdraw", post(fedimint::wallet::withdraw::handle_rest));

    let admin_router = Router::new()
        .route("/backup", post(fedimint::admin::backup::handle_rest))
        .route(
            "/discover-version",
            get(fedimint::admin::discover_version::handle_rest),
        )
        .route(
            "/federation-ids",
            get(fedimint::admin::federation_ids::handle_rest),
        )
        .route("/info", get(fedimint::admin::info::handle_rest))
        .route("/join", post(fedimint::admin::join::handle_rest))
        .route("/restore", post(fedimint::admin::restore::handle_rest))
        .route(
            "/list-operations",
            post(fedimint::admin::list_operations::handle_rest),
        )
        .route("/module", post(fedimint::admin::module::handle_rest))
        .route("/config", get(fedimint::admin::config::handle_rest));

    Router::new()
        .nest("/admin", admin_router)
        .nest("/mint", mint_router)
        .nest("/ln", ln_router)
        .nest("/wallet", wallet_router)
}

fn cashu_v1_rest() -> Router<AppState> {
    debug!("Configuring Cashu V1 REST API");
    Router::new()
        .route("/keys", get(cashu::keys::handle_keys))
        .route("/keys/:keyset_id", get(cashu::keys::handle_keys_keyset_id))
        .route("/keysets", get(cashu::keysets::handle_keysets))
        .route("/swap", post(cashu::swap::handle_swap))
        .route(
            "/mint/quote/:method",
            get(cashu::mint::quote::handle_method),
        )
        .route(
            "/mint/quote/:method/:quote_id",
            get(cashu::mint::quote::handle_method_quote_id),
        )
        .route("/mint/:method", post(cashu::mint::method::handle_method))
        .route(
            "/melt/quote/:method",
            get(cashu::melt::quote::handle_method),
        )
        .route(
            "/melt/quote/:method/:quote_id",
            get(cashu::melt::quote::handle_method_quote_id),
        )
        .route("/melt/:method", post(cashu::melt::method::handle_method))
        .route("/info", get(cashu::info::handle_info))
        .route("/check", post(cashu::check::handle_check))
}
