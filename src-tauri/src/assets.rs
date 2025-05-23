use axum::{
    routing::{get, post},
    Router,
    extract::Path,
    response::Response,
    body::Body,
};

pub async fn proxy_uploads(Path(path): Path<String>) -> Response<Body> {
    let target_url = format!("https://xjtu.app/uploads/{}", path);
    info!("proxing to: {}", target_url.clone());
    match reqwest::get(&target_url).await {
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

