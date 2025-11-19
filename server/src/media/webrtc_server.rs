use anyhow::Result;
use std::sync::Arc;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::MediaEngine,
        APIBuilder,
    },
    ice_transport::{ice_credential_type::RTCIceCredentialType, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType},
};

/// WebRTC server for media streaming
pub struct WebRtcServer {
    api: webrtc::api::API,
    config: RTCConfiguration,
}

impl WebRtcServer {
    pub fn new() -> Self {
        // Create media engine with audio/video codecs
        let mut media_engine = MediaEngine::default();
        
        // Register audio codecs
        media_engine
            .register_codec(
                RTCRtpCodecParameters {
                    capability: RTCRtpCodecCapability {
                        mime_type: "audio/opus".to_string(),
                        clock_rate: 48000,
                        channels: 2,
                        sdp_fmtp_line: "".to_string(),
                        rtcp_feedback: vec![],
                    },
                    payload_type: 111,
                    ..Default::default()
                },
                RTPCodecType::Audio,
            )
            .expect("Failed to register Opus codec");
        
        // Register video codecs
        media_engine
            .register_codec(
                RTCRtpCodecParameters {
                    capability: RTCRtpCodecCapability {
                        mime_type: "video/H264".to_string(),
                        clock_rate: 90000,
                        channels: 0,
                        sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42001f".to_string(),
                        rtcp_feedback: vec![],
                    },
                    payload_type: 102,
                    ..Default::default()
                },
                RTPCodecType::Video,
            )
            .expect("Failed to register H264 codec");
        
        // Create interceptor registry
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .expect("Failed to register interceptors");
        
        // Create API
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();
        
        // ICE configuration
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: String::new(),
                credential: String::new(),
                credential_type: RTCIceCredentialType::Unspecified,
            }],
            ..Default::default()
        };
        
        Self { api, config }
    }
    
    /// Create a new peer connection
    pub async fn create_peer_connection(&self) -> Result<Arc<RTCPeerConnection>> {
        let peer_connection = Arc::new(
            self.api
                .new_peer_connection(self.config.clone())
                .await?,
        );
        
        // Set up event handlers
        let pc = peer_connection.clone();
        peer_connection.on_peer_connection_state_change(Box::new(
            move |state: RTCPeerConnectionState| {
                tracing::info!("Peer connection state changed: {:?}", state);
                Box::pin(async {})
            },
        ));
        
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            if let Some(c) = candidate {
                tracing::debug!("New ICE candidate: {}", c.to_string());
            }
            Box::pin(async {})
        }));
        
        Ok(peer_connection)
    }
    
    /// Create SDP offer
    pub async fn create_offer(
        peer_connection: &Arc<RTCPeerConnection>,
    ) -> Result<webrtc::peer_connection::sdp::session_description::RTCSessionDescription> {
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;
        Ok(offer)
    }
    
    /// Handle SDP answer
    pub async fn handle_answer(
        peer_connection: &Arc<RTCPeerConnection>,
        answer: webrtc::peer_connection::sdp::session_description::RTCSessionDescription,
    ) -> Result<()> {
        peer_connection.set_remote_description(answer).await?;
        Ok(())
    }
    
    /// Add ICE candidate
    pub async fn add_ice_candidate(
        peer_connection: &Arc<RTCPeerConnection>,
        candidate: webrtc::ice_transport::ice_candidate::RTCIceCandidateInit,
    ) -> Result<()> {
        peer_connection.add_ice_candidate(candidate).await?;
        Ok(())
    }
}