use std::collections::HashMap;

use card::{Card, CardId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod card;
pub mod effect;

/// The technomancy engine
pub struct Engine {
    cards: HashMap<CardId, Card>,
}

impl Engine {
    pub fn new(cards: HashMap<CardId, Card>) -> Self {
        Self { cards }
    }

    fn card_exists(&self, card: CardId) -> bool {
        self.cards.contains_key(&card)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PlayerId(Uuid);

impl PlayerId {
    fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    id: PlayerId,
    initial_cards: Vec<CardId>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    Hand(PlayerId),
    Library(PlayerId),
    Discard(PlayerId),
    Battlefield,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameZone {
    objects: Vec<GameObject>,
}

impl GameZone {
    fn empty() -> GameZone {
        GameZone { objects: vec![] }
    }

    fn with(objects: Vec<GameObject>) -> GameZone {
        GameZone { objects }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ObjectId(uuid::Uuid);

impl ObjectId {
    fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameObject {
    id: ObjectId,
    underlying_card: Option<CardId>,
}
impl GameObject {
    fn from_card(underlying_card: CardId) -> GameObject {
        GameObject {
            id: ObjectId::new(),
            underlying_card: Some(underlying_card),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameState {
    zones: HashMap<ZoneId, GameZone>,
}

pub struct GameError;

pub enum GameResult {
    NextState(GameState),
    GameOver { winners: Vec<PlayerId> },
    Error(GameError),
}

impl GameResult {
    pub fn unwrap_next_state(self) -> GameState {
        match self {
            GameResult::NextState(state) => state,
            _ => panic!("Tried to unwrap a non-next state"),
        }
    }
}

pub enum VerificationError {
    PlayerInvalidCard { id: PlayerId, card: CardId },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    Player(PlayerId),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetId {
    Player(PlayerId),
    Object(ObjectId),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GameAtom {
    DealDamage {
        amount: usize,
        source: ObjectId,
        target: TargetId,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    players: HashMap<PlayerId, Player>,
    game_states: Vec<GameState>,
}

impl Game {
    pub fn new(players: HashMap<PlayerId, Player>) -> Self {
        Self {
            players,
            game_states: vec![],
        }
    }

    pub fn verify(&self, engine: &Engine) -> Result<(), Vec<VerificationError>> {
        let mut errors = vec![];

        for (id, player) in &self.players {
            for card in &player.initial_cards {
                if !engine.card_exists(*card) {
                    errors.push(VerificationError::PlayerInvalidCard {
                        id: *id,
                        card: *card,
                    });
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(())
    }

    pub async fn run(&self, engine: &Engine) -> GameResult {
        if self.game_states.is_empty() {
            return GameResult::NextState(new_game_state_with(&self.players));
        }

        GameResult::NextState(self.game_states.last().unwrap().clone())
    }
}

fn new_game_state_with(players: &HashMap<PlayerId, Player>) -> GameState {
    GameState {
        zones: players
            .values()
            .flat_map(|p| {
                vec![
                    (ZoneId::Hand(p.id), GameZone::empty()),
                    (
                        ZoneId::Library(p.id),
                        GameZone::with(
                            p.initial_cards
                                .iter()
                                .map(|c| GameObject::from_card(*c))
                                .collect(),
                        ),
                    ),
                    (ZoneId::Discard(p.id), GameZone::empty()),
                ]
            })
            .chain(vec![(ZoneId::Battlefield, GameZone::empty())])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        card::{
            BaseCardKind, Card, CardBehaviour, CardEffect, CardId, CardKind, Cost,
            TriggeredCardEffect,
        },
        effect::{tests::DealDamage, Effect, EffectTrigger},
        Engine, Game, GameResult, Player, PlayerId, ZoneId,
    };

    fn existing_cards() -> HashMap<CardId, Card> {
        let blast = Card {
            id: CardId::with(uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48")),
            behaviour: CardBehaviour {
                cost: Some(Cost {
                    corp1_scrip: 2,
                    ..Default::default()
                }),
                kind: vec![CardKind {
                    kind: BaseCardKind::Quickhack,
                }],
                effects: vec![CardEffect::Triggered(TriggeredCardEffect {
                    trigger: EffectTrigger::OnSelfCast,
                    effect: vec![Effect::Instant(Box::new(DealDamage(3)))],
                })],
            },
        };

        [(blast.id, blast)].into()
    }

    fn simple_deck() -> Vec<CardId> {
        vec![
            CardId::with(uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48")),
            CardId::with(uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48")),
            CardId::with(uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48")),
            CardId::with(uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48")),
        ]
    }

    fn playtesters() -> HashMap<PlayerId, Player> {
        vec![
            Player {
                id: PlayerId::new(),
                initial_cards: simple_deck(),
            },
            Player {
                initial_cards: simple_deck(),
                id: PlayerId::new(),
            },
        ]
        .into_iter()
        .map(|p| (p.id, p))
        .collect()
    }

    fn new_engine() -> Engine {
        Engine::new(existing_cards())
    }

    #[tokio::test]
    async fn check_initial_game_creation() {
        let engine = new_engine();

        let game = Game::new(playtesters());

        match game.run(&engine).await {
            GameResult::NextState(_) => (),
            _ => panic!("Expected a new state"),
        }
    }

    #[tokio::test]
    async fn check_initial_game_zones() {
        let engine = new_engine();
        let players = playtesters();
        let first_player = *players.keys().next().unwrap();
        let num_playtesters = players.len();
        let game = Game::new(players);

        let state = game.run(&engine).await.unwrap_next_state();

        assert_eq!(num_playtesters * 3 + 1, state.zones.len());
        assert_eq!(
            simple_deck().len(),
            state
                .zones
                .get(&ZoneId::Library(first_player))
                .unwrap()
                .objects
                .len()
        );
    }
}
