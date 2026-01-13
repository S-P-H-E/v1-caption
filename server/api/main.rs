use axum::{
    Json, Router,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tower::ServiceBuilder;
use validator::ValidationError;
use vercel_runtime::Error;
use vercel_runtime::axum::VercelLayer;
use yt_transcript_rs::{YouTubeTranscriptApi, proxies::GenericProxyConfig};

async fn favicon() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        include_bytes!("../public/favicon.ico").as_slice(),
    )
}

fn validate_video_id(video_id: &str) -> Result<(), ValidationError> {
    if video_id.len() != 11 {
        return Err(ValidationError::new(
            "video_id must be exactly 11 characters",
        ));
    }

    let invalid_chars: Vec<char> = video_id
        .chars()
        .filter(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
        .collect();

    if !invalid_chars.is_empty() {
        let mut error = ValidationError::new("invalid_characters");
        error.message =
            Some(format!("video_id contains invalid characters: {:?}", invalid_chars).into());
        return Err(error);
    }

    Ok(())
}

fn format_views(views: &str) -> String {
    let num: u64 = match views.parse() {
        Ok(n) => n,
        Err(_) => return views.to_string(),
    };

    let (value, suffix) = match num {
        n if n >= 1_000_000_000 => (n as f64 / 1_000_000_000.0, "B"),
        n if n >= 1_000_000 => (n as f64 / 1_000_000.0, "M"),
        n if n >= 1_000 => (n as f64 / 1_000.0, "K"),
        n => return n.to_string(),
    };

    // Format with 1 decimal, then strip ".0" if present
    let formatted = format!("{:.1}", value);
    let clean = formatted.trim_end_matches(".0");
    format!("{}{}", clean, suffix)
}

fn seconds_to_timestamp(seconds: f64) -> String {
    let total = seconds as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

#[allow(clippy::collapsible_if)]
fn extract_id_from_url(url: &str) -> Result<String, ValidationError> {
    let url = url.trim();

    // Full URL: https://www.youtube.com/watch?v=VIDEO_ID
    if url.contains("youtube.com/watch") {
        if let Some(id) = url.split("v=").nth(1).and_then(|s| s.split('&').next()) {
            if validate_video_id(id).is_ok() {
                return Ok(id.to_string());
            }
        }
        return Err(ValidationError::new(
            "invalid YouTube URL: could not extract valid video ID",
        ));
    }

    // Short URL: https://youtu.be/VIDEO_ID
    if url.contains("youtu.be/") {
        if let Some(id) = url
            .split("youtu.be/")
            .nth(1)
            .and_then(|s| s.split('?').next())
        {
            if validate_video_id(id).is_ok() {
                return Ok(id.to_string());
            }
        }
        return Err(ValidationError::new(
            "invalid YouTube URL: could not extract valid video ID",
        ));
    }

    Err(ValidationError::new(
        "invalid YouTube URL: must be youtube.com/watch or youtu.be URL",
    ))
}

#[derive(Deserialize)]
struct YTRequest {
    video_id: Option<String>,
    video_url: Option<String>,
}

#[derive(Serialize)]
struct YTResponse {
    id: String,
    title: String,
    author: String,
    views: String,
    transcript: Vec<TranscriptSnippet>,
}

#[derive(Serialize)]
struct TranscriptSnippet {
    start: String,
    duration: f64,
    text: String,
}

async fn hello() -> impl IntoResponse {
    Json(json!({ "message": "Welcome to v1-caption!" }))
}

async fn yt(Json(payload): Json<YTRequest>) -> Result<Json<YTResponse>, (StatusCode, String)> {
    // Validate: exactly one of video_id or video_url must be provided
    let video_id = match (&payload.video_id, &payload.video_url) {
        (Some(_), Some(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Cannot provide both video_id and video_url. Use one or the other.".to_string(),
            ));
        }
        (None, None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Must provide either video_id or video_url.".to_string(),
            ));
        }
        (Some(id), None) => {
            // Validate the video_id
            validate_video_id(id).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    e.message
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string()),
                )
            })?;
            id.clone()
        }
        (None, Some(url)) => {
            // Validate the URL and extract video_id
            extract_id_from_url(url).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    e.message
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string()),
                )
            })?
        }
    };

    // Create API instance with optional proxy
    let proxy_url = std::env::var("PROXY_URL").ok();
    let has_proxy = proxy_url.is_some();
    println!("Proxy configured: {}", has_proxy);

    let proxy_config = proxy_url.map(|url| {
        let preview: String = url.chars().take(40).collect();
        println!(
            "Using proxy: {}{}",
            preview,
            if url.len() > 20 { "..." } else { "" }
        );
        Box::new(GenericProxyConfig::new(Some(url.clone()), Some(url)).unwrap())
            as Box<dyn yt_transcript_rs::proxies::ProxyConfig + Send + Sync>
    });

    let api = YouTubeTranscriptApi::new(None, proxy_config, None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("API init error: {}", e),
        )
    })?;

    // Fetch transcript
    let transcript = api
        .fetch_transcript(&video_id, &["en"], false)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Transcript error (proxy={}): {}", has_proxy, e),
            )
        })?;

    // Fetch video details
    let details = api
        .fetch_video_details(&video_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let snippets: Vec<TranscriptSnippet> = transcript
        .parts()
        .iter()
        .map(|snippet| TranscriptSnippet {
            start: seconds_to_timestamp(snippet.start),
            duration: snippet.duration,
            text: snippet.text.replace(">> ", ""),
        })
        .collect();

    let formatted_views = format_views(&details.view_count);

    Ok(Json(YTResponse {
        id: video_id,
        title: details.title,
        author: details.author,
        views: formatted_views,
        transcript: snippets,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    let router = Router::new()
        .route("/", get(hello))
        .route("/transcript", post(yt))
        .route("/favicon.ico", get(favicon));

    let app = ServiceBuilder::new()
        .layer(VercelLayer::new())
        .service(router);
    vercel_runtime::run(app).await
}
