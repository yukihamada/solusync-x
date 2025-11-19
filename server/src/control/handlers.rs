use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    protocol::{MediaAction, MediaParams, MessageHeader},
    AppState,
};

/// Play request
#[derive(Debug, Deserialize)]
pub struct PlayRequest {
    pub track_id: String,
    pub start_at: Option<f64>,
    pub volume: Option<f32>,
}

/// API response
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Handle play command
pub async fn play(
    State(state): State<AppState>,
    Json(req): Json<PlayRequest>,
) -> impl IntoResponse {
    let start_at = match req.start_at {
        Some(t) => t,
        None => state.clock_manager.now().await + 0.1,
    };
    
    let control = crate::protocol::MediaControlMessage {
        header: MessageHeader::new(Uuid::new_v4(), 0),
        action: MediaAction::Play,
        track_id: req.track_id.clone(),
        start_at,
        params: MediaParams {
            volume: req.volume,
            loop_count: None,
            fade_in_ms: None,
            fade_out_ms: None,
            seek_position: None,
        },
    };
    
    match state
        .media_server
        .get_control_sender()
        .send(control)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "track_id": req.track_id,
                "start_at": start_at,
            }))),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ),
    }
}

/// Handle pause command
pub async fn pause(
    State(state): State<AppState>,
    Json(track_id): Json<String>,
) -> impl IntoResponse {
    let control = crate::protocol::MediaControlMessage {
        header: MessageHeader::new(Uuid::new_v4(), 0),
        action: MediaAction::Pause,
        track_id: track_id.clone(),
        start_at: state.clock_manager.now().await,
        params: MediaParams {
            volume: None,
            loop_count: None,
            fade_in_ms: None,
            fade_out_ms: None,
            seek_position: None,
        },
    };
    
    match state
        .media_server
        .get_control_sender()
        .send(control)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(track_id)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(e.to_string())),
        ),
    }
}

/// Sync request
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub client_time: f64,
}

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub client_time: f64,
    pub server_time: f64,
    pub offset: f64,
}

/// Handle time sync
pub async fn sync(
    State(state): State<AppState>,
    Json(req): Json<SyncRequest>,
) -> impl IntoResponse {
    let server_time = state.clock_manager.now().await;
    let offset = server_time - req.client_time;
    
    (
        StatusCode::OK,
        Json(ApiResponse::success(SyncResponse {
            client_time: req.client_time,
            server_time,
            offset,
        })),
    )
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub server_id: String,
    pub server_time: f64,
    pub uptime_seconds: u64,
    pub connected_clients: u32,
    pub active_streams: u32,
}

/// Get server status
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    // TODO: Get real stats
    let status = StatusResponse {
        server_id: Uuid::new_v4().to_string(),
        server_time: state.clock_manager.now().await,
        uptime_seconds: 0,
        connected_clients: 0,
        active_streams: 0,
    };
    
    (StatusCode::OK, Json(ApiResponse::success(status)))
}

/// Get connected clients
pub async fn connected_clients(State(state): State<AppState>) -> impl IntoResponse {
    let clients = state.control_server.get_connected_clients().await;
    (StatusCode::OK, Json(ApiResponse::success(clients)))
}