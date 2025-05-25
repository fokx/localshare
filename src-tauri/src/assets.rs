use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
    extract::{Path, State},
    response::Response,
    body::Body,
};
use tauri::Manager;
use tauri_plugin_fs::FsExt;
use tokio;

#[derive(Clone)]
pub struct AppState {
    pub app_handle: tauri::AppHandle,
    pub client: reqwest::Client,
}

pub async fn proxy_uploads(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>
) -> Response<Body> {
    let client = state.client.clone();
    let app_handle = state.app_handle.clone();
    let path_handle = app_handle.path();
    
    let target_url = format!("https://xjtu.app/uploads/{}", path);
    info!("proxing to: {}", target_url.clone());
    // Create cache directory path
    let cache_dir = path_handle.resolve("assets", tauri::path::BaseDirectory::AppCache).unwrap();
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
    } else{
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
        },
        Err(_) => {
            Response::builder()
                    .status(404)
                    .body(Body::empty())
                    .unwrap()
        }
    }

}

