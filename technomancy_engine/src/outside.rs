use std::time::{Duration, SystemTime};

use tarpc::client::RpcError;

use crate::{GameId, ObjectId, PlayerAction, PlayerId, TargetId};

#[tarpc::service]
pub trait Outside {
    async fn get_player_keeping(game_id: GameId, asked_players: Vec<PlayerId>) -> Vec<PlayerId>;
    async fn get_next_player_action_from(
        game_id: GameId,
        player: PlayerId,
        player_actions: Vec<PlayerAction>,
    ) -> usize;
    async fn get_target_choices_from_given(
        game_id: GameId,
        player: PlayerId,
        source: ObjectId,
        name: String,
        choices: Vec<TargetId>,
        count: usize,
    ) -> Vec<usize>;
    async fn get_player_passing(game_id: GameId, player: PlayerId) -> bool;
}

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
