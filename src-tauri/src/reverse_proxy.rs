use reqwest::Client;
use axum::{
    body::Body,
    body::to_bytes,
    extract::{Path, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use hyper;
use std::sync::Arc;

use std::path::PathBuf;
use tauri::Manager;
use tauri_plugin_fs::FsExt;
use tokio;

#[derive(Clone)]
pub struct AppState {
    pub app_handle: tauri::AppHandle,
    pub client: Client,          // The HTTP client for forwarding requests
}

#[axum::debug_handler]
pub async fn proxy_all_requests(
    State(state): State<Arc<AppState>>,
    path: Option<Path<String>>, // Accept Option<String> for handling both cases
    req: Request<Body>,        // Captures the incoming request
) -> Response<Body> {
    let backend_url = "https://xjtu.app";
    let path = path.unwrap_or_else(|| axum::extract::Path("".to_string()));

    let client = &state.client; // Clone the reqwest client from the state
    let target_url = format!("{}/{}", backend_url.trim_end_matches('/'), path.as_str()); // Construct the target URL

    info!("Proxying request to: {}", target_url);

    // Prepare the forwarded request
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body = req.into_body();

    // Convert axum body to bytes
    let body_bytes = to_bytes(body, usize::MAX).await.unwrap_or_default();

    let mut forwarded_request = client.request(method, &target_url);

    // Forward headers
    for (key, value) in headers.iter() {
        forwarded_request = forwarded_request.header(key, value);
    }

    // Forward the body as bytes
    let forwarded_response = forwarded_request.body(body_bytes).send().await;

    // Relay the response back to the client
    match forwarded_response {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();
            let is_text = headers
                    .get("content-type")
                    .and_then(|ct| ct.to_str().ok())
                    .map(|ct| ct.starts_with("text/"))
                    .unwrap_or(false);

            let bytes = response.bytes().await.unwrap();
            let body_bytes = if is_text {
                let text = String::from_utf8_lossy(&bytes);
                let replaced = text
                        .replace("http://xjtu.app", "http://127.0.0.1:4805")
                        .replace("https://xjtu.app", "http://127.0.0.1:4805");
                replaced.into_bytes()
            } else {
                bytes.to_vec()
            };
            let mut response_builder = Response::builder().status(status);
            response_builder = response_builder.header("Access-Control-Allow-Origin", "*");
            for (key, value) in headers {
                if let Some(k) = key {
                    if k != "access-control-allow-origin" && k != "alt-svc" {
                        response_builder = response_builder.header(k, value);
                    }
                }
            }

            response_builder.body(Body::from(body_bytes)).unwrap()
        }
        Err(err) => {
            error!("Error proxying request: {}", err);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from("Bad Gateway"))
                .unwrap()
        }
    }
}

pub async fn proxy_uploads(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Response<Body> {
    let client = state.client.clone();
    let app_handle = state.app_handle.clone();
    let path_handle = app_handle.path();

    let target_url = format!("https://xjtu.app/uploads/{}", path);
    info!("proxy_uploads to: {}", target_url.clone());
    // Create cache directory path
    let cache_dir = path_handle
            .resolve("assets", tauri::path::BaseDirectory::AppCache)
            .unwrap();
    let mut current_dir = cache_dir.clone();
    let path_segments: Vec<&str> = path.split('/').collect();

    // Create nested directories from path segments
    for segment in &path_segments[..path_segments.len() - 1] {
        current_dir = current_dir.join(segment);
        if !current_dir.exists() {
            std::fs::create_dir_all(&current_dir).unwrap();
        }
    }

    let file_path = cache_dir.join(&path);
    if file_path.exists() {
        if let Ok(contents) = tokio::fs::read(&file_path).await {
            info!("Serving {} from cache: {:?}", path, file_path);
            return Response::builder()
                    .status(200)
                    .body(Body::from(contents))
                    .unwrap();
        } else {
            error!("{:?} read failed", file_path);
        }
        // If reading fails, fallback to downloading
    } else {
        warn!("{:?} does not exist", file_path);
    };
    return match client.get(&target_url).send().await {
        Ok(response) => {
            let bytes = response.bytes().await.unwrap();
            let file_path = cache_dir.join(&path);
            tokio::fs::write(&file_path, &bytes).await.unwrap();

            Response::builder()
                    .status(200)
                    .body(Body::from(bytes))
                    .unwrap()
        }
        Err(_) => Response::builder().status(404).body(Body::empty()).unwrap(),
    };
}

// List files in `cache_dir`
pub async fn list_files(State(state): State<Arc<AppState>>) -> Json<Vec<String>> {
    let app_handle = state.app_handle.clone();
    let cache_dir = app_handle
            .path()
            .resolve("assets", tauri::path::BaseDirectory::AppCache)
            .unwrap();

    let mut files = vec![];
    let mut entries = tokio::fs::read_dir(&cache_dir).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        if entry.file_type().await.unwrap().is_file() {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    Json(files)
}

// Download a file
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> axum::response::Result<Vec<u8>> {
    let app_handle = state.app_handle.clone();
    let cache_dir = app_handle
            .path()
            .resolve("assets", tauri::path::BaseDirectory::AppCache)
            .unwrap();
    let file_path = cache_dir.join(filename);

    let file_content = tokio::fs::read(file_path).await.unwrap();
    Ok(file_content)
}

use serde_json::json;

#[axum::debug_handler]
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let app_handle = state.app_handle.clone();

    // Try resolving the cache directory
    let cache_dir = match app_handle
            .path()
            .resolve("assets", tauri::path::BaseDirectory::AppCache)
    {
        Ok(dir) => dir,
        Err(e) => {
            let error_message = format!("Failed to resolve cache directory: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": error_message })));
        }
    };

    // Sanitize the filename
    let sanitized_filename = match std::path::Path::new(&filename)
            .file_name()
            .and_then(|name| name.to_str())
    {
        Some(valid_name) => valid_name.to_string(),
        None => {
            let error_message = "Invalid filename".to_string();
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": error_message })));
        }
    };

    let file_path = cache_dir.join(sanitized_filename);

    // Write the file
    if let Err(e) = tokio::fs::write(&file_path, body).await {
        let error_message = format!("Failed to write file to {:?}: {e}", file_path);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": error_message })),
        );
    }

    // Return a successful response
    (
        StatusCode::OK,
        Json(json!({
            "message": "File uploaded successfully"
        })),
    )
}