use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(root))
        .route("/about", post(about));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!(?addr, "listening");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(Serialize)]
struct About {
    version: String,
}

async fn about() -> Json<About> {
    Json(About {
        version: String::from("0.0.0"),
    })
}
