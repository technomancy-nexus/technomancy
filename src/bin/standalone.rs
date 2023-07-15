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
use tokio::task::AbortHandle;

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
                assert_send(game.run(&client).boxed()).await;
            }
        })
        .abort_handle();

        let info = GameInfo { handle };

        self.games.insert(id, info);

        id
    }

    async fn destroy_game(self, _ctx: Context, game: GameId) {
        self.games.remove(&game);
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// What interface and port to listen to
    listen_interface: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let cards = Arc::new(std::collections::HashMap::new());

    let mut conn = tarpc::serde_transport::tcp::listen(
        &args.listen_interface,
        tarpc::tokio_serde::formats::Json::default,
    )
    .await
    .unwrap();

    while let Some(Ok(inc)) = conn.next().await {
        let (server, client) = spawn_twoway(inc);
        let outside_client = OutsideClient::new(tarpc::client::Config::default(), client).spawn();
        let engine_server = EngineServer::new(outside_client, cards.clone());

        tokio::spawn(BaseChannel::with_defaults(server).execute(engine_server.serve()));
    }
}
