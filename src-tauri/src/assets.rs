use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
    extract::{Path, State},
    response::Response,
    body::Body,
};

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
    let target_url = format!("https://xjtu.app/uploads/{}", path);
    info!("proxing to: {}", target_url.clone());
    match client.get(&target_url).send().await {
        Ok(response) => {
            let bytes = response.bytes().await.unwrap();
            Response::builder()
                    .status(200)
                    .body(Body::from(bytes))
                    .unwrap()
        },
        Err(_) => Response::builder()
                .status(404)
                .body(Body::empty())
                .unwrap()
    }
}

