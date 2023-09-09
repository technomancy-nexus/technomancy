use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::{rejection::MatchedPathRejection, FromRef, FromRequestParts, MatchedPath, State},
    http::request::Parts,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Extension, Form, RequestPartsExt, Router,
};
use axum_login::{
    extractors::AuthContext, memory_store::MemoryStore as AuthMemoryStore, AuthLayer,
    RequireAuthorizationLayer,
};
use axum_sessions::{async_session::MemoryStore as SessionMemoryStore, SessionLayer};

use axum_template::{engine::Engine, RenderHtml};
use handlebars::Handlebars;
use lobby::Lobby;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tracing::trace;
use user::User;

mod lobby;
mod user;

type TemplateEngine = Engine<Handlebars<'static>>;
type UserStorage = Arc<RwLock<HashMap<String, User>>>;
type LobbyStorage = Arc<RwLock<HashMap<String, Lobby>>>;

pub struct PathKey(pub String);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for PathKey
where
    S: Send + Sync,
{
    type Rejection = MatchedPathRejection;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let key = parts
            // `axum_template::Key` internally uses `axum::extract::MatchedPath`
            .extract::<MatchedPath>()
            .await?
            .as_str()
            // Cargo doesn't allow `:` as a file name
            .replace(':', "&")
            .chars()
            // Remove the first character `/`
            .skip(1)
            .collect();
        Ok(PathKey(key))
    }
}

#[derive(Clone, FromRef)]
struct AppState {
    engine: TemplateEngine,
    user_storage: UserStorage,
    lobby_storage: LobbyStorage,
}

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

    let mut hbs = Handlebars::new();
    hbs.set_dev_mode(true);
    hbs.register_templates_directory(".hbs", "templates")
        .unwrap();

    let templates = hbs.get_templates().keys().collect::<Vec<_>>();
    trace!(?templates, "Registered templates");

    let lobby_storage = Arc::new(RwLock::new(HashMap::from([(
        "default".to_string(),
        Lobby {
            id: "default".to_string(),
            owner: "Nobody".to_string(),
            name: "The Default Lobby".to_string(),
            users: Default::default(),
        },
    )])));

    let state = AppState {
        engine: Engine::from(hbs),
        user_storage: store,
        lobby_storage,
    };

    Router::new()
        .route("/", get(root))
        .route("/lobbies", get(lobby::list_lobbies))
        .route("/lobbies", post(lobby::create_lobby))
        .route("/lobbies/:lobby_id/join", post(lobby::join_lobby))
        .route("/lobbies/:lobby_id", get(lobby::show_lobby))
        .route_layer(RequireAuth::login())
        .route("/login", get(login_handler))
        .route("/login", post(do_login))
        .nest_service("/static", ServeDir::new("static"))
        .layer(auth_layer)
        .layer(session_layer)
        .with_state(state)
}

async fn root(Extension(user): Extension<User>, engine: TemplateEngine) -> impl IntoResponse {
    #[derive(Debug, Serialize)]
    struct TplData {
        current_user: User,
    }

    RenderHtml("root", engine, TplData { current_user: user })
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
}

async fn do_login(
    State(store): State<UserStorage>,
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

async fn login_handler(engine: TemplateEngine, PathKey(key): PathKey) -> impl IntoResponse {
    RenderHtml(key, engine, ())
}
