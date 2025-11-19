use anyhow::Result;
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod clock;
mod control;
mod media;
mod protocol;

use crate::{
    clock::ClockManager,
    control::ControlServer,
    media::MediaServer,
};

#[derive(Clone)]
pub struct AppState {
    clock_manager: Arc<ClockManager>,
    media_server: Arc<MediaServer>,
    control_server: Arc<ControlServer>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "solusync_x_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting SOLUSync-X Server v0.1.0");

    // Initialize components
    let clock_manager = Arc::new(ClockManager::new());
    let media_server = Arc::new(MediaServer::new());
    let control_server = Arc::new(ControlServer::new(
        clock_manager.clone(),
        media_server.clone(),
    ));

    let app_state = AppState {
        clock_manager: clock_manager.clone(),
        media_server: media_server.clone(),
        control_server: control_server.clone(),
    };

    // Start background tasks
    tokio::spawn(clock_manager.run());
    tokio::spawn(media_server.run());

    // Build HTTP/WebSocket server
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .route("/api/play", post(control::handlers::play))
        .route("/api/pause", post(control::handlers::pause))
        .route("/api/sync", post(control::handlers::sync))
        .route("/api/status", get(control::handlers::status))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("SOLUSync-X Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root() -> &'static str {
    "SOLUSync-X Server v0.1.0"
}

async fn health_check() -> &'static str {
    "OK"
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
) -> () {
    if let Err(e) = state.control_server.handle_connection(socket).await {
        tracing::error!("WebSocket error: {}", e);
    }
}