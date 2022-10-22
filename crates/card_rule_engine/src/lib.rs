use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Card;

#[derive(Debug, Clone, Default)]
pub struct GameActionInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ZoneName(String);

#[derive(Debug, Clone)]
pub struct TargetZone {
    player_id: PlayerId,
    zone_name: ZoneName,
}

impl TargetZone {
    pub fn player_id(&self) -> PlayerId {
        self.player_id
    }

    pub fn zone_name(&self) -> &ZoneName {
        &self.zone_name
    }
}

#[derive(Debug)]
pub enum GameAction {
    AddPlayer(Player),
    AddZone(ZoneName),
    AddCardsTo {
        target_zone: TargetZone,
        cards: Vec<Card>,
    },
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerId(Uuid);

impl PlayerId {
    pub fn new() -> PlayerId {
        PlayerId(Uuid::new_v4())
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    id: PlayerId,
    zones: HashMap<ZoneName, Vec<Card>>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: PlayerId::new(),
            zones: Default::default(),
        }
    }
}

impl Player {
    pub fn id(&self) -> PlayerId {
        self.id
    }

    pub fn zones(&self) -> &HashMap<ZoneName, Vec<Card>> {
        &self.zones
    }

    pub fn get_zone_mut(
        &mut self,
        zone_name: &ZoneName,
    ) -> Result<&mut Vec<Card>, GameUpdateError> {
        Ok(self.zones.entry(zone_name.clone()).or_default())
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameState {
    configured_zones: HashSet<ZoneName>,
    players: HashMap<PlayerId, Player>,
}

#[derive(Debug)]
pub enum GameUpdateError {
    UnknownPlayer(PlayerId),
    UnknownZone(ZoneName),
}

impl GameState {
    pub fn update(
        &self,
        action: GameAction,
        _state: GameActionInput,
    ) -> Result<GameState, GameUpdateError> {
        let mut new_state = self.clone();

        match action {
            GameAction::AddPlayer(ply) => {
                new_state.players.insert(ply.id, ply);
            }
            GameAction::AddZone(new_zone) => {
                new_state.configured_zones.insert(new_zone);
            }
            GameAction::AddCardsTo { target_zone, cards } => {
                let player = new_state.get_player_mut(target_zone.player_id())?;
                let zone = player.get_zone_mut(target_zone.zone_name())?;
                zone.extend(cards);
            }
        }

        Ok(new_state)
    }

    pub fn players(&self) -> &HashMap<PlayerId, Player> {
        &self.players
    }

    pub fn get_player_mut(&mut self, player_id: PlayerId) -> Result<&mut Player, GameUpdateError> {
        self.players
            .get_mut(&player_id)
            .ok_or_else(|| GameUpdateError::UnknownPlayer(player_id))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Card, GameAction, GameActionInput, GameState, Player, PlayerId, ZoneName};

    #[test]
    fn check_adding_players() {
        let old_state = GameState::default();

        let new_state = old_state
            .update(
                GameAction::AddPlayer(Player {
                    id: PlayerId::new(),
                    ..Default::default()
                }),
                GameActionInput::default(),
            )
            .unwrap();

        assert_eq!(new_state.players().len(), 1);
        assert_eq!(old_state.players().len(), 0);
    }

    #[test]
    fn check_adding_zones_and_cards_to_players() {
        let old_state = GameState::default();

        let first_player = PlayerId::new();
        let new_state = old_state
            .update(
                GameAction::AddPlayer(Player {
                    id: first_player,
                    ..Default::default()
                }),
                GameActionInput::default(),
            )
            .unwrap()
            .update(
                GameAction::AddZone(crate::ZoneName(String::from("hand"))),
                GameActionInput::default(),
            )
            .unwrap()
            .update(
                GameAction::AddCardsTo {
                    target_zone: crate::TargetZone {
                        player_id: first_player,
                        zone_name: ZoneName(String::from("hand")),
                    },
                    cards: vec![Card],
                },
                GameActionInput::default(),
            )
            .unwrap();

        assert_eq!(
            new_state.players()[&first_player].zones()[&ZoneName(String::from("hand"))].len(),
            1
        );
    }
}
