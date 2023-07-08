use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{routing::get, Extension, Router};
use axum_login::{
    extractors::AuthContext, memory_store::MemoryStore as AuthMemoryStore, AuthLayer,
    RequireAuthorizationLayer,
};
use axum_sessions::{async_session::MemoryStore as SessionMemoryStore, SessionLayer};

use tokio::sync::RwLock;
use user::User;

mod user;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = app();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!(?addr, "listening");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

type Auth = AuthContext<String, User, AuthMemoryStore<String, User>>;
type RequireAuth = RequireAuthorizationLayer<String, User>;

fn app() -> Router {
    let secret = [0u8; 64];

    let session_store = SessionMemoryStore::new();
    let session_layer = SessionLayer::new(session_store, &secret);

    let store = Arc::new(RwLock::new(HashMap::from([(
        String::from("test"),
        User {
            name: "test".to_string(),
        },
    )])));

    let user_store: AuthMemoryStore<String, User> = AuthMemoryStore::new(&store);
    let auth_layer = AuthLayer::new(user_store, &secret);

    Router::new()
        .route("/", get(root))
        .route_layer(RequireAuth::login())
        .route("/login", get(login_handler))
        .layer(auth_layer)
        .layer(session_layer)
}

async fn root(Extension(user): Extension<User>) -> String {
    format!("Hello {}", user.name)
}

async fn login_handler(mut auth: Auth) -> &'static str {
    auth.login(&User {
        name: String::from("test"),
    })
    .await
    .unwrap();

    "Logged in!"
}
