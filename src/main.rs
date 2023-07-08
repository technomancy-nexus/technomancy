use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Extension, Form, Router,
};
use axum_login::{
    extractors::AuthContext, memory_store::MemoryStore as AuthMemoryStore, AuthLayer,
    RequireAuthorizationLayer,
};
use axum_sessions::{async_session::MemoryStore as SessionMemoryStore, SessionLayer};

use serde::Deserialize;
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
        .route("/login", post(do_login))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(store)
}

async fn root(Extension(user): Extension<User>) -> String {
    format!("Hello {}", user.name)
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
}

async fn do_login(
    State(store): State<Arc<RwLock<HashMap<String, User>>>>,
    mut auth: Auth,
    data: Form<LoginForm>,
) -> Redirect {
    let user = User {
        name: data.username.clone(),
    };
    auth.login(&user).await.unwrap();
    store.write().await.insert(data.username.clone(), user);

    Redirect::to("/")
}

async fn login_handler() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <body>
            <form action="/login" method="POST">
                <input name="username"></input>
            </form>
        </body>
    "#,
    )
}
