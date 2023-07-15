use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use tarpc::client::RpcError;
use technomancy_core::outside::OutsideClient;

use crate::{GameId, ObjectId, PlayerAction, PlayerId, TargetId};

#[async_trait::async_trait]
pub trait OutsideGame {
    async fn get_player_keeping(
        &self,
        asked_players: Vec<PlayerId>,
    ) -> Result<Vec<PlayerId>, RpcError>;
    async fn get_next_player_action_from(
        &self,
        player: PlayerId,
        player_actions: Vec<PlayerAction>,
    ) -> Result<usize, RpcError>;
    async fn get_target_choices_from_given(
        &self,
        player: PlayerId,
        source: ObjectId,
        name: String,
        choices: Vec<TargetId>,
        count: usize,
    ) -> Result<Vec<usize>, RpcError>;
    async fn get_player_passing(&self, player: PlayerId) -> Result<bool, RpcError>;
}

#[derive(Debug)]
pub struct OutsideGameClient {
    pub game_id: GameId,
    pub client: Arc<OutsideClient>,
}

impl OutsideGameClient {
    pub fn new(game_id: GameId, client: Arc<OutsideClient>) -> Self {
        Self { game_id, client }
    }
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

    async fn get_next_player_action_from(
        &self,
        player: PlayerId,
        player_actions: Vec<PlayerAction>,
    ) -> Result<usize, RpcError> {
        self.client
            .get_next_player_action_from(get_context(), self.game_id, player, player_actions)
            .await
    }

    async fn get_target_choices_from_given(
        &self,
        player: PlayerId,
        source: ObjectId,
        name: String,
        choices: Vec<TargetId>,
        count: usize,
    ) -> Result<Vec<usize>, RpcError> {
        self.client
            .get_target_choices_from_given(
                get_context(),
                self.game_id,
                player,
                source,
                name,
                choices,
                count,
            )
            .await
    }

    async fn get_player_passing(&self, player: PlayerId) -> Result<bool, RpcError> {
        self.client
            .get_player_passing(get_context(), self.game_id, player)
            .await
    }
}

#[cfg(test)]
const TIMEOUT: Duration = Duration::from_millis(100);

#[cfg(not(test))]
const TIMEOUT: Duration = Duration::from_secs(60 * 60 * 24);

fn get_context() -> tarpc::context::Context {
    let mut ctx = tarpc::context::current();
    ctx.deadline = SystemTime::now() + TIMEOUT;
    ctx
}
