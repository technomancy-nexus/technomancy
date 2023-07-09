use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    Extension, Form,
};
use axum_template::RenderHtml;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{user::User, LobbyStorage, PathKey, TemplateEngine};

#[derive(Debug, Serialize, Clone)]
pub struct Lobby {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) users: HashSet<String>,
}

pub async fn list_lobbies(
    State(lobbies): State<LobbyStorage>,
    engine: TemplateEngine,
    PathKey(key): PathKey,
) -> impl IntoResponse {
    let lobbies = lobbies.read().await;
    let all_lobbies = lobbies.values().collect::<Vec<_>>();
    RenderHtml(key, engine, json!({ "lobbies": all_lobbies }))
}

#[derive(Debug, Deserialize)]
pub struct NewLobbyForm {
    name: String,
}

pub async fn create_lobby(
    State(lobbies): State<LobbyStorage>,
    Extension(user): Extension<User>,
    Form(new_lobby): Form<NewLobbyForm>,
) -> impl IntoResponse {
    let mut lobbies = lobbies.write().await;
    let id = format!("{}_lobby", user.name);
    let new_lobby = Lobby {
        name: new_lobby.name,
        users: [user.name.clone()].into(),
        id: id.clone(),
    };
    lobbies.insert(id.clone(), new_lobby);

    Redirect::to(&format!("/lobbies/{id}"))
}

pub async fn join_lobby(
    State(lobbies): State<LobbyStorage>,
    Extension(user): Extension<User>,
    Path(lobby_id): Path<String>,
) -> impl IntoResponse {
    let mut lobbies = lobbies.write().await;
    let lobby = lobbies.get_mut(&lobby_id).unwrap();
    lobby.users.insert(user.name.clone());

    Redirect::to(&format!("/lobbies/{lobby_id}"))
}

pub async fn show_lobby(
    State(lobbies): State<LobbyStorage>,
    engine: TemplateEngine,
    PathKey(key): PathKey,
    Path(lobby_id): Path<String>,
) -> impl IntoResponse {
    let lobbies = lobbies.read().await;
    let lobby = lobbies.get(&lobby_id).unwrap();
    RenderHtml(key, engine, json!({ "lobby": lobby }))
}
