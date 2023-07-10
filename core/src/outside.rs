#![allow(clippy::too_many_arguments)]

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
