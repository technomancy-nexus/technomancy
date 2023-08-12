
#[cfg(test)] use std::net::SocketAddr;

use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use dashmap::DashMap;
use futures::{FutureExt, StreamExt};
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256StarStar;
use tarpc::{
    context::Context,
    server::{BaseChannel, Channel},
};
use technomancy_core::{
    card::{Card, CardId},
    meta::{spawn_twoway, Meta},
    outside::OutsideClient,
    GameId, Player,
};
use technomancy_engine::{outside::OutsideGameClient, GameImplV1};
use tokio::{sync::oneshot::Sender, task::AbortHandle};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct GameInfo {
    handle: AbortHandle,
}

#[derive(Debug, Clone)]
struct EngineServer {
    client: Arc<OutsideClient>,
    cards: Arc<std::collections::HashMap<CardId, Card>>,
    games: Arc<DashMap<GameId, GameInfo>>,
}

impl EngineServer {
    fn new(client: OutsideClient, cards: Arc<HashMap<CardId, Card>>) -> Self {
        EngineServer {
            client: Arc::new(client),
            cards,
            games: Default::default(),
        }
    }

    fn get_outside_client(&self, game_id: GameId) -> OutsideGameClient {
        OutsideGameClient {
            game_id,
            client: self.client.clone(),
        }
    }
}

#[tarpc::server]
impl Meta for EngineServer {
    async fn create_game(self, _ctx: Context, players: Vec<Player>) -> GameId {
        let id = GameId::new();

        let rand = Xoshiro256StarStar::seed_from_u64(rand::random());

        let players: HashMap<_, _> = players.into_iter().map(|p| (p.id, p)).collect();
        let order = players.keys().copied().collect();
        let game = GameImplV1::new(id, rand, self.cards.clone(), players, order);
        let client = self.get_outside_client(id);

        fn assert_send<'u, R>(
            fut: impl 'u + Send + std::future::Future<Output = R>,
        ) -> impl 'u + Send + std::future::Future<Output = R> {
            fut
        }

        let handle = tokio::spawn(async move {
            let mut game = game;
            let client = client;
            loop {
                let res = assert_send(game.run(&client).boxed()).await;

                match res {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Encountered an error: {e}");
                        break;
                    }
                }
            }
        })
        .abort_handle();

        let info = GameInfo { handle };

        self.games.insert(id, info);

        id
    }

    async fn destroy_game(self, _ctx: Context, game: GameId) {
        if let Some((_, game)) = self.games.remove(&game) {
            info!("Aborting game");
            game.handle.abort();
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// What interface and port to listen to
    #[clap(long)]
    listen_interface: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_timer(tracing_subscriber::fmt::time::uptime());

    tracing_subscriber::registry().with(fmt_layer).init();

    let cards = Arc::new(std::collections::HashMap::new());

    let (sender, recv) = tokio::sync::oneshot::channel();

    let handle = tokio::spawn(start_server(args, cards, sender));

    let info = recv.await.unwrap();

    info!(?info, "Server started");

    handle.await.unwrap();
}

#[derive(Debug)]
struct ServerInfo {
    #[cfg(test)]
    local_addr: SocketAddr,
}

async fn start_server(
    args: Args,
    cards: Arc<HashMap<CardId, Card>>,
    server_info: Sender<ServerInfo>,
) {
    info!("Starting technomancy engine on {}", args.listen_interface);
    let mut conn = tarpc::serde_transport::tcp::listen(
        &args.listen_interface,
        tarpc::tokio_serde::formats::Json::default,
    )
    .await
    .unwrap();

    let info = ServerInfo {
        #[cfg(test)]
        local_addr: conn.local_addr(),
    };

    let _ = server_info.send(info);

    while let Some(Ok(inc)) = conn.next().await {
        let addr = inc.peer_addr().unwrap();
        info!("New connection from {addr}");
        let (server, client) = spawn_twoway(inc);
        let outside_client = OutsideClient::new(tarpc::client::Config::default(), client).spawn();
        let engine_server = EngineServer::new(outside_client, cards.clone());

        tokio::spawn(BaseChannel::with_defaults(server).execute(engine_server.serve()));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tarpc::context::Context;
    use technomancy_core::{
        meta::{spawn_twoway, MetaClient},
        outside::{OutsideRequest, OutsideResponse},
    };
    use tokio::task::JoinHandle;
    use tracing::info;

    use crate::{start_server, Args, ServerInfo};

    async fn get_server() -> (ServerInfo, JoinHandle<()>) {
        let args = Args {
            listen_interface: "localhost:0".to_string(),
        };
        let cards = Arc::new(std::collections::HashMap::new());

        let (sender, recv) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(start_server(args, cards, sender));

        let info = recv.await.unwrap();

        (info, handle)
    }

    #[test_log::test(tokio::test)]
    async fn check_connection() {
        let (info, handle) = get_server().await;

        info!(?info, "Listening");

        handle.abort();
        handle.await.unwrap_err();
    }

    #[test_log::test(tokio::test)]
    async fn check_start_game() {
        let (info, handle) = get_server().await;
        let client_conn = tarpc::serde_transport::tcp::connect(
            info.local_addr,
            tarpc::tokio_serde::formats::Json::default,
        )
        .await
        .unwrap();

        let (_outside_server, meta_client) =
            spawn_twoway::<OutsideRequest, OutsideResponse, _, _, _>(client_conn);

        let client = MetaClient::new(Default::default(), meta_client).spawn();

        client
            .create_game(Context::current(), vec![])
            .await
            .unwrap();

        handle.abort();

        handle.await.unwrap_err();
    }
}
