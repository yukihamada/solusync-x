use anyhow::Result;
use axum::{
    extract::{ws::WebSocketUpgrade, State, ConnectInfo},
    response::Response,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::info;
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

    // Serve static files from public directory
    let serve_dir = ServeDir::new("public");

    // Build HTTP/WebSocket server
    let app = Router::new()
        .nest_service("/", serve_dir.clone())
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .route("/api/play", post(control::handlers::play))
        .route("/api/pause", post(control::handlers::pause))
        .route("/api/sync", post(control::handlers::sync))
        .route("/api/status", get(control::handlers::status))
        .route("/api/clients", get(control::handlers::connected_clients))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Display startup information
    display_startup_info(addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}


async fn health_check() -> &'static str {
    "OK"
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state, addr))
}

async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    state: AppState,
    addr: SocketAddr,
) -> () {
    if let Err(e) = state.control_server.handle_connection(socket, Some(addr)).await {
        tracing::error!("WebSocket error: {}", e);
    }
}

fn display_startup_info(addr: SocketAddr) {
    use qrcode::QrCode;
    use qrcode::render::unicode;
    
    let local_ip = local_ip_address::local_ip().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)));
    let local_url = format!("http://{}:8080", local_ip);
    let localhost_url = "http://localhost:8080";
    
    println!("\n{}", "=".repeat(60));
    println!("🌕 SOLUSync-X Server v0.1.0");
    println!("{}", "=".repeat(60));
    println!("\n📡 Server Status: \x1b[32mRunning\x1b[0m\n");
    
    println!("🌐 Access URLs:");
    println!("   Local:    \x1b[36m{}\x1b[0m", localhost_url);
    println!("   Network:  \x1b[36m{}\x1b[0m", local_url);
    println!();
    
    // Generate QR code
    if let Ok(code) = QrCode::new(&local_url) {
        let image = code.render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();
        
        println!("📱 Scan QR code with your phone:");
        println!();
        for line in image.lines() {
            println!("   {}", line);
        }
    }
    
    println!("\n🔌 WebSocket: ws://{}:8080/ws", local_ip);
    println!("❤️  Health:   http://{}:8080/health", local_ip);
    
    println!("\n📊 Features:");
    println!("   • Ultra-low latency sync (±0.5ms)");
    println!("   • PTP-inspired clock synchronization");
    println!("   • Dynamic future buffer (30-250ms)");
    println!("   • WebRTC media streaming");
    println!("   • Self-healing cluster support");
    
    println!("\n🚀 Ready for connections!");
    println!("{}", "=".repeat(60));
    println!();
    
    info!("Server listening on {}", addr);
    info!("Local URL: {}", localhost_url);
    info!("Network URL: {}", local_url);
}