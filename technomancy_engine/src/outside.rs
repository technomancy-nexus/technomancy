use tarpc::client::RpcError;

use crate::{GameId, PlayerId};

#[tarpc::service]
pub trait Outside {
    async fn get_player_keeping(game_id: GameId, asked_players: Vec<PlayerId>) -> Vec<PlayerId>;
}

#[async_trait::async_trait]
pub trait OutsideGame {
    async fn get_player_keeping(
        &self,
        asked_players: Vec<PlayerId>,
    ) -> Result<Vec<PlayerId>, RpcError>;
}

pub struct OutsideGameClient {
    pub game_id: GameId,
    pub client: OutsideClient,
}

#[async_trait::async_trait]
impl OutsideGame for OutsideGameClient {
    async fn get_player_keeping(
        &self,
        asked_players: Vec<PlayerId>,
    ) -> Result<Vec<PlayerId>, RpcError> {
        self.client
            .get_player_keeping(get_context(), self.game_id, asked_players)
            .await
    }
}

fn get_context() -> tarpc::context::Context {
    tarpc::context::current()
}
