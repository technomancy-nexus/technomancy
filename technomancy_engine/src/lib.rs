#![allow(dead_code)]
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use card::{Card, CardId};
use effect::{EffectInfo, ExecuteFailure};
use outside::{OutsideGame, OutsideGameClient};
use rand::{seq::SliceRandom, Fill, Rng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};
use tarpc::client::RpcError;
use tracing::trace;
use uuid::Uuid;

use crate::card::TriggeredCardEffect;

pub mod card;
pub mod effect;
pub mod outside;

/// The technomancy engine
pub struct Engine {
    cards: Arc<std::collections::HashMap<CardId, Card>>,
    games: std::collections::HashMap<GameId, Game>,
}

impl Engine {
    pub fn new(cards: std::collections::HashMap<CardId, Card>) -> Self {
        Self {
            cards: Arc::new(cards),
            games: std::collections::HashMap::new(),
        }
    }

    fn card_exists(&self, card: CardId) -> bool {
        self.cards.contains_key(&card)
    }
}

pub fn get_seeded_uuid(rng: &mut impl Rng) -> uuid::Uuid {
    let mut random_bytes: [u8; 16] = [0; 16];
    random_bytes.try_fill(rng).unwrap();

    uuid::Builder::from_random_bytes(random_bytes).into_uuid()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PlayerAction {
    PlayCard { from: ZoneId, object: ObjectId },
    PassPriority,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PlayerId(Uuid);

impl PlayerId {
    fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    id: PlayerId,
    initial_cards: Vec<CardId>,
    starting_health: usize,
    health: isize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    Hand(PlayerId),
    Library(PlayerId),
    Discard(PlayerId),
    Battlefield,
    Stack,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct Objects(pub Vec<GameObject>);

impl std::ops::Deref for Objects {
    type Target = Vec<GameObject>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Objects {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameZone {
    objects: Objects,
}

impl GameZone {
    fn empty() -> GameZone {
        GameZone {
            objects: Objects(vec![]),
        }
    }

    fn with(objects: Vec<GameObject>) -> GameZone {
        GameZone {
            objects: Objects(objects),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct LibraryCardId(uuid::Uuid);

impl LibraryCardId {
    fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ObjectId(uuid::Uuid);

impl ObjectId {
    fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameObject {
    id: ObjectId,
    /// An identifier for that tracks a card throughout the whole game, no two objects in the same
    /// game should have the same library card id
    library_card_id: Option<LibraryCardId>,
    /// The identifier of the card it represents if any, several objects may have the same card id
    underlying_card: Option<CardId>,
    /// Objects only have a controller on the stack and battlefield
    controller: Option<PlayerId>,
    /// Any choices associated to the object
    choices: HashMap<(usize, String), EffectInfo>,
}
impl GameObject {
    fn from_card(rand: &mut impl Rng, underlying_card: CardId) -> GameObject {
        GameObject {
            id: ObjectId::new(rand),
            library_card_id: Some(LibraryCardId::new(rand)),
            underlying_card: Some(underlying_card),
            controller: None,
            choices: HashMap::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Game was already running when a GameStart atom was sent")]
    GameAlreadyRunning,
    #[error("An RPC error was encountered")]
    RPCError(#[from] RpcError),
    #[error("A keep hand atom was generated during normal game running")]
    KeepHandDuringGame,
    #[error("An invalid action was selected")]
    InvalidAction,
    #[error("The expected object could not be found")]
    ObjectNotFound { object: ObjectId },
    #[error("A player was marked as passing although they either already passed, or its not their moment to pass")]
    InvalidPlayerPassing { player: PlayerId },
    #[error("An object was expected to contain an underlying card, but it did not")]
    NoUnderlyingCard { object: ObjectId },
    #[error("A card id was given without the existing card underneath")]
    CardNotFound { card: CardId },
    #[error("A response was given with more or less than the required amount")]
    InvalidChoiceAmount { expected: usize, received: usize },
    #[error("A response was given with more or less than the required amount")]
    EffectExecuteFailure {
        #[source]
        failure: ExecuteFailure,
    },
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GameAtom {
    /// Starts the game
    ///
    /// Only valid at the beginning of the game
    StartGame,
    KeepHand {
        player: PlayerId,
    },
    ShuffleHandIntoLibrary {
        player: PlayerId,
    },
    DrawCards {
        player: PlayerId,
        count: usize,
    },
    DealDamage {
        amount: usize,
        source: ObjectId,
        target: TargetId,
    },
    PassPriority {
        player: PlayerId,
    },
    PlayerPlayCard {
        from: ZoneId,
        object: ObjectId,
        stack_id: ObjectId,
    },
    AttachChoicesTo {
        zone: ZoneId,
        object: ObjectId,
        choices: HashMap<(usize, String), EffectInfo>,
    },
    PlayerFinishPlayingCard,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct GameId(Uuid);

impl GameId {
    fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GameStage {
    KeepHand { players_keeping: HashSet<PlayerId> },
    GameRunning,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameState {
    zones: hashbrown::HashMap<ZoneId, GameZone>,
    /// The turn order, index 0 is the active player
    active_player_order: Vec<PlayerId>,
    /// Players who have not yet passed since the last stack-modifying action
    unpassed_players: Vec<PlayerId>,
    game_stage: GameStage,
}
impl GameState {
    fn get_hand(&self, p: PlayerId) -> &GameZone {
        self.zones.get(&ZoneId::Hand(p)).unwrap()
    }

    fn get_stack(&self) -> &GameZone {
        self.zones.get(&ZoneId::Stack).unwrap()
    }

    fn get_battlefield(&self) -> &GameZone {
        self.zones.get(&ZoneId::Battlefield).unwrap()
    }

    fn get_object_from_zone(&self, from: ZoneId, obj: ObjectId) -> Option<&GameObject> {
        let zone = self.zones.get(&from)?;
        zone.objects.iter().find(|o| o.id == obj)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    #[serde(skip)]
    cards: Arc<std::collections::HashMap<CardId, Card>>,
    id: GameId,
    players: std::collections::HashMap<PlayerId, Player>,
    rand: Xoshiro256StarStar,
    game_states: Vec<GameState>,
    history: Vec<(usize, Vec<GameAtom>)>,
}

impl Game {
    pub fn new(
        mut rand: Xoshiro256StarStar,
        cards: Arc<std::collections::HashMap<CardId, Card>>,
        players: std::collections::HashMap<PlayerId, Player>,
        order: Vec<PlayerId>,
    ) -> Self {
        let initial_game_state = new_game_state_with(&mut rand, &players, &order);
        Self {
            cards,
            players,
            id: GameId::new(&mut rand),
            rand,
            game_states: vec![initial_game_state],
            history: vec![],
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

    pub fn get_controller_of(&self, object: ObjectId) -> Option<PlayerId> {
        let state = self.latest_gamestate();
        let bf = state.zones.get(&ZoneId::Battlefield).unwrap();
        let stack = state.zones.get(&ZoneId::Stack).unwrap();

        let obj = bf
            .objects
            .iter()
            .chain(stack.objects.iter())
            .find(|o| o.id == object)?;

        obj.controller
    }

    fn latest_gamestate(&self) -> &GameState {
        self.game_states.last().unwrap()
    }

    pub fn apply_atoms(&mut self, atoms: Vec<GameAtom>) -> Result<(), GameError> {
        self.history
            .push((self.game_states.len() - 1, atoms.clone()));
        let mut next_state = self.latest_gamestate().clone();
        for atom in atoms {
            match atom {
                GameAtom::StartGame => {
                    if next_state.game_stage == GameStage::GameRunning {
                        return Err(GameError::GameAlreadyRunning);
                    } else {
                        next_state.game_stage = GameStage::GameRunning
                    }
                }
                GameAtom::DealDamage {
                    amount,
                    source: _,
                    target,
                } => match target {
                    TargetId::Player(ply) => {
                        let player = self.players.get_mut(&ply).unwrap();
                        player.health -= amount as isize;
                    }
                    TargetId::Object(_) => todo!(),
                },
                GameAtom::KeepHand { player } => {
                    if let GameStage::KeepHand { players_keeping } = &mut next_state.game_stage {
                        players_keeping.insert(player);
                    } else {
                        return Err(GameError::KeepHandDuringGame);
                    }
                }
                GameAtom::ShuffleHandIntoLibrary { player } => {
                    let Some([hand, library]) = next_state.zones.get_many_mut([&ZoneId::Hand(player), &ZoneId::Library(player)]) else { unreachable!() };
                    library.objects.extend(hand.objects.drain(..));
                    library.objects.shuffle(&mut self.rand);
                }
                GameAtom::DrawCards { player, count } => {
                    let Some([hand, library]) = next_state.zones.get_many_mut([&ZoneId::Hand(player), &ZoneId::Library(player)]) else { unreachable!() };
                    let new_count = library.objects.len().saturating_sub(count);
                    hand.objects.extend(library.objects.drain(new_count..));
                }
                GameAtom::PassPriority { player } => {
                    if next_state.unpassed_players.first() == Some(&player) {
                        next_state.unpassed_players.pop();
                    } else {
                        return Err(GameError::InvalidPlayerPassing { player });
                    }
                }
                GameAtom::PlayerPlayCard {
                    from,
                    object,
                    stack_id,
                } => {
                    let Some([from, to]) = next_state.zones.get_many_mut([&from, &ZoneId::Stack]) else { unreachable!() };
                    if let Some(obj_idx) = from.objects.iter().position(|o| o.id == object) {
                        let mut obj = from.objects.remove(obj_idx);
                        obj.id = stack_id;
                        to.objects.push(obj);
                    } else {
                        return Err(GameError::ObjectNotFound { object });
                    }
                }
                GameAtom::AttachChoicesTo {
                    zone,
                    object,
                    choices,
                } => {
                    let object = next_state
                        .zones
                        .get_mut(&zone)
                        .unwrap()
                        .objects
                        .iter_mut()
                        .find(|o| o.id == object)
                        .ok_or_else(|| GameError::ObjectNotFound { object })?;

                    object.choices.extend(choices);
                }
                GameAtom::PlayerFinishPlayingCard => {
                    next_state.unpassed_players = next_state.active_player_order.clone();
                }
            }
        }
        self.game_states.push(next_state);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip_all, fields(game = ?self.id), err)]
    async fn run(&mut self, outside: &OutsideGameClient) -> Result<(), GameError> {
        match self.latest_gamestate().game_stage.clone() {
            GameStage::KeepHand { players_keeping } => {
                trace!("Checking for potential mulligans");
                let latest_gamestate = self.latest_gamestate();
                let atoms: Vec<_> = self
                    .players
                    .keys()
                    .filter(|p| !players_keeping.contains(p))
                    .flat_map(|p| {
                        let hand = latest_gamestate.get_hand(*p);

                        match hand.objects.len() {
                            1 => vec![
                                GameAtom::ShuffleHandIntoLibrary { player: *p },
                                GameAtom::KeepHand { player: *p },
                            ],
                            0 => vec![GameAtom::DrawCards {
                                player: *p,
                                count: 7,
                            }],
                            count => vec![
                                GameAtom::ShuffleHandIntoLibrary { player: *p },
                                GameAtom::DrawCards {
                                    player: *p,
                                    count: count - 1,
                                },
                            ],
                        }
                    })
                    .collect();
                self.apply_atoms(atoms)?;

                let latest_gamestate = self.latest_gamestate();

                let GameStage::KeepHand { players_keeping } = &latest_gamestate.game_stage else {
                    unreachable!()
                };

                let players_not_kept_yet = self
                    .players
                    .keys()
                    .filter(|p| !players_keeping.contains(p))
                    .copied()
                    .collect();
                let players_keeping = outside.get_player_keeping(players_not_kept_yet).await?;

                self.apply_atoms(
                    players_keeping
                        .into_iter()
                        .map(|p| GameAtom::KeepHand { player: p })
                        .collect(),
                )?;

                let latest_gamestate = self.latest_gamestate();

                let GameStage::KeepHand { players_keeping } = &latest_gamestate.game_stage else {
                        unreachable!()
                    };

                if players_keeping.len() == self.players.len() {
                    trace!("All players have kept, we can start the game");
                    self.apply_atoms(vec![GameAtom::StartGame])?;
                    return Ok(());
                }
            }
            GameStage::GameRunning => {
                let latest_gamestate = self.latest_gamestate();

                let stack = latest_gamestate.get_stack();

                if stack.objects.is_empty() {
                    let active_player = latest_gamestate.active_player_order.first().unwrap();

                    let mut possible_actions = vec![PlayerAction::PassPriority];
                    possible_actions.extend(
                        latest_gamestate
                            .get_hand(*active_player)
                            .objects
                            .iter()
                            .map(|hand_obj| PlayerAction::PlayCard {
                                from: ZoneId::Hand(*active_player),
                                object: hand_obj.id,
                            }),
                    );
                    let action_idx = outside
                        .get_next_player_action_from(*active_player, possible_actions.clone())
                        .await?;

                    let Some(action) = possible_actions.get(action_idx) else {
                        return Err(GameError::InvalidAction);
                    };

                    match action {
                        PlayerAction::PassPriority => {
                            let atoms = vec![GameAtom::PassPriority {
                                player: *active_player,
                            }];
                            self.apply_atoms(atoms)?;
                        }
                        PlayerAction::PlayCard { from, object } => {
                            // Playing a card is a fairly involved process as it needs to be as
                            // intuitive as possible
                            //
                            // The order of operations is thus:
                            //
                            // 1. Put the card on the stack
                            // 2. Get all choices made (First modes, then targets)
                            // 3. Calculate the total cost of the card
                            // 4. Let the player pay the cost
                            // 5. Attach the info to the stack object
                            // 6. Done playing the card, resume normal game

                            // Step 1

                            let stack_id;
                            let atoms = vec![GameAtom::PlayerPlayCard {
                                from: ZoneId::Hand(*active_player),
                                object: *object,
                                stack_id: {
                                    stack_id = ObjectId::new(&mut self.rand);
                                    stack_id
                                },
                            }];
                            self.apply_atoms(atoms)?;

                            let latest_gamestate = self.latest_gamestate();

                            let active_player =
                                latest_gamestate.active_player_order.first().unwrap();

                            let obj = latest_gamestate
                                .get_object_from_zone(*from, *object)
                                .ok_or_else(|| GameError::InvalidAction)?;

                            let card = obj
                                .underlying_card
                                .as_ref()
                                .ok_or_else(|| GameError::NoUnderlyingCard { object: *object })?;

                            let card = self
                                .cards
                                .get(card)
                                .ok_or_else(|| GameError::CardNotFound { card: *card })?;

                            let self_cast_effects = card
                                .behaviour
                                .effects
                                .iter()
                                .filter_map(|e| match e {
                                    card::CardEffect::Triggered(TriggeredCardEffect {
                                        trigger: effect::EffectTrigger::OnSelfCast,
                                        effects,
                                    }) => Some(effects),
                                    _ => None,
                                })
                                .flatten();

                            // Step 2

                            let mut gathered_info = HashMap::new();
                            for (idx, e) in self_cast_effects.enumerate() {
                                match e {
                                    effect::Effect::Continuous(_) => todo!(),
                                    effect::Effect::Instant(instant) => {
                                        let required_info = instant.get_required_info();
                                        for (name, question) in required_info {
                                            match question {
                                                effect::EffectInfoRequest::SingleTarget {
                                                    restriction,
                                                } => {
                                                    if let Some(_) = restriction {
                                                        todo!()
                                                    } else {
                                                        // Without any restrictions targets can
                                                        // _only_ be agents on the battlefield _or_
                                                        // players
                                                        let mut possible_choices = vec![];
                                                        possible_choices.extend(
                                                            self.players
                                                                .keys()
                                                                .map(|p| TargetId::Player(*p)),
                                                        );
                                                        possible_choices.extend(
                                                            latest_gamestate
                                                                .get_battlefield()
                                                                .objects
                                                                .iter()
                                                                .filter(|_o| todo!())
                                                                .map(|o| TargetId::Object(o.id)),
                                                        );
                                                        let choices = outside
                                                            .get_target_choices_from_given(
                                                                *active_player,
                                                                stack_id,
                                                                name.clone(),
                                                                possible_choices.clone(),
                                                                1,
                                                            )
                                                            .await?;

                                                        if choices.len() != 1 {
                                                            return Err(
                                                                GameError::InvalidChoiceAmount {
                                                                    expected: 1,
                                                                    received: choices.len(),
                                                                },
                                                            );
                                                        }

                                                        let selected_choices: Vec<TargetId> =
                                                            possible_choices
                                                                .into_iter()
                                                                .enumerate()
                                                                .filter_map(|(idx, choice)| {
                                                                    choices
                                                                        .contains(&idx)
                                                                        .then_some(choice)
                                                                })
                                                                .collect();
                                                        gathered_info.insert(
                                                            (idx, name),
                                                            effect::EffectInfo::SingleTarget(
                                                                selected_choices[0],
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Step 3

                            // Step 4

                            // Step 5

                            let atoms = vec![
                                GameAtom::AttachChoicesTo {
                                    zone: ZoneId::Stack,
                                    object: stack_id,
                                    choices: gathered_info,
                                },
                                GameAtom::PlayerFinishPlayingCard,
                            ];
                            self.apply_atoms(atoms)?;
                        }
                    }
                } else {
                    todo!()
                }
            }
        }

        Ok(())
    }
}

fn new_game_state_with(
    rand: &mut impl Rng,
    players: &std::collections::HashMap<PlayerId, Player>,
    order: &[PlayerId],
) -> GameState {
    GameState {
        game_stage: GameStage::KeepHand {
            players_keeping: Default::default(),
        },
        active_player_order: order.to_vec(),
        unpassed_players: order.to_vec(),
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
                                .map(|c| GameObject::from_card(rand, *c))
                                .collect(),
                        ),
                    ),
                    (ZoneId::Discard(p.id), GameZone::empty()),
                ]
            })
            .chain(vec![
                (ZoneId::Battlefield, GameZone::empty()),
                (ZoneId::Stack, GameZone::empty()),
            ])
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256StarStar;
    use tarpc::{server::Channel, transport::channel::UnboundedChannel, ClientMessage, Response};
    use tokio::sync::Mutex;

    use crate::{
        card::{
            BaseCardKind, Card, CardBehaviour, CardEffect, CardId, CardKind, Cost,
            TriggeredCardEffect,
        },
        effect::{
            tests::{DealDamage, DrawCards},
            Effect, EffectTrigger,
        },
        outside::{Outside, OutsideClient, OutsideRequest, OutsideResponse},
        Engine, Game, GameId, ObjectId, OutsideGameClient, Player, PlayerAction, PlayerId,
        TargetId, ZoneId,
    };

    const BLAST_CARD: uuid::Uuid = uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48");
    const DRAW_CARD: uuid::Uuid = uuid::uuid!("ddfbf54b-2750-41c6-b657-1d6ce1e754ef");

    fn existing_cards() -> HashMap<CardId, Card> {
        let blast = Card {
            id: CardId::with(BLAST_CARD),
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
                    effects: vec![Effect::Instant(Box::new(DealDamage(3)))],
                })],
            },
        };

        let draw = Card {
            id: CardId::with(BLAST_CARD),
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
                    effects: vec![Effect::Instant(Box::new(DrawCards(3)))],
                })],
            },
        };

        [(blast.id, blast), (draw.id, draw)].into()
    }

    fn simple_deck() -> Vec<CardId> {
        vec![
            CardId::with(BLAST_CARD),
            CardId::with(BLAST_CARD),
            CardId::with(BLAST_CARD),
            CardId::with(BLAST_CARD),
            CardId::with(DRAW_CARD),
            CardId::with(DRAW_CARD),
            CardId::with(DRAW_CARD),
            CardId::with(DRAW_CARD),
        ]
    }

    fn playtesters(rand: &mut impl Rng) -> HashMap<PlayerId, Player> {
        vec![
            Player {
                id: PlayerId::new(rand),
                initial_cards: simple_deck(),
                starting_health: 25,
                health: 25,
            },
            Player {
                initial_cards: simple_deck(),
                id: PlayerId::new(rand),
                starting_health: 25,
                health: 25,
            },
        ]
        .into_iter()
        .map(|p| (p.id, p))
        .collect()
    }

    fn new_engine() -> Engine {
        Engine::new(existing_cards())
    }

    fn outside_client(
        game_id: GameId,
    ) -> (
        tarpc::transport::channel::UnboundedChannel<
            tarpc::ClientMessage<OutsideRequest>,
            tarpc::Response<OutsideResponse>,
        >,
        OutsideGameClient,
    ) {
        let (left, right) = tarpc::transport::channel::unbounded();
        let client = OutsideClient::new(tarpc::client::Config::default(), left).spawn();
        (right, OutsideGameClient { game_id, client })
    }

    struct ServerAnswers {
        get_player_keeping: Option<Box<dyn FnMut(Vec<PlayerId>) -> Vec<PlayerId> + Send>>,
        get_next_player_action_from:
            Option<Box<dyn FnMut(PlayerId, Vec<PlayerAction>) -> usize + Send>>,
        get_target_choices_from_given: Option<
            Box<dyn FnMut(PlayerId, ObjectId, String, Vec<TargetId>, usize) -> Vec<usize> + Send>,
        >,
    }

    impl Default for ServerAnswers {
        fn default() -> Self {
            Self {
                get_player_keeping: Some(Box::new(|players| players)),
                get_next_player_action_from: Default::default(),
                get_target_choices_from_given: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    struct SimpleOutsideServer {
        answers: Arc<Mutex<ServerAnswers>>,
    }

    #[tarpc::server]
    impl Outside for SimpleOutsideServer {
        async fn get_player_keeping(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
            asked_players: Vec<PlayerId>,
        ) -> Vec<PlayerId> {
            self.answers
                .lock()
                .await
                .get_player_keeping
                .as_mut()
                .expect("No method set")(asked_players)
        }
        async fn get_next_player_action_from(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
            player: PlayerId,
            player_actions: Vec<PlayerAction>,
        ) -> usize {
            self.answers
                .lock()
                .await
                .get_next_player_action_from
                .as_mut()
                .expect("No method set")(player, player_actions)
        }
        async fn get_target_choices_from_given(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
            player: PlayerId,
            source: ObjectId,
            name: String,
            choices: Vec<TargetId>,
            count: usize,
        ) -> Vec<usize> {
            self.answers
                .lock()
                .await
                .get_target_choices_from_given
                .as_mut()
                .expect("No method set")(player, source, name, choices, count)
        }
    }

    struct SimpleTestHarness {
        engine: Engine,
        player_order: Vec<PlayerId>,
        game: Game,
        outside_client: OutsideGameClient,
        answers: Arc<Mutex<ServerAnswers>>,
    }

    fn init_harness(
        seed: Option<u64>,
    ) -> (
        Vec<PlayerId>,
        Engine,
        Game,
        tarpc::transport::channel::UnboundedChannel<
            tarpc::ClientMessage<OutsideRequest>,
            tarpc::Response<OutsideResponse>,
        >,
        OutsideGameClient,
    ) {
        let mut rand = Xoshiro256StarStar::seed_from_u64(seed.unwrap_or(1337));
        let players = playtesters(&mut rand);
        let player_order: Vec<_> = players.keys().copied().collect();
        let engine = new_engine();

        let game = Game::new(rand, engine.cards.clone(), players, player_order.clone());

        let (server, outside_client) = outside_client(game.id);

        (player_order, engine, game, server, outside_client)
    }

    impl SimpleTestHarness {
        fn new(seed: Option<u64>, answers: ServerAnswers) -> Self {
            let (harness, server) = Self::new_with_server(seed, answers);

            let server = tarpc::server::BaseChannel::with_defaults(server);
            let _outside_server = tokio::spawn(
                server.execute(
                    SimpleOutsideServer {
                        answers: harness.answers.clone(),
                    }
                    .serve(),
                ),
            );

            harness
        }
        fn new_with_server(
            seed: Option<u64>,
            answers: ServerAnswers,
        ) -> (
            SimpleTestHarness,
            UnboundedChannel<ClientMessage<OutsideRequest>, Response<OutsideResponse>>,
        ) {
            let (player_order, engine, game, server, outside_client) = init_harness(seed);

            (
                SimpleTestHarness {
                    engine,
                    player_order,
                    game,
                    outside_client,
                    answers: Arc::new(Mutex::new(answers)),
                },
                server,
            )
        }
    }

    macro_rules! game_steps {
        (@set $harness:ident $action:ident = $($func:tt)*) => {
            $harness.answers.lock().await.$action = Some(Box::new($($func)*));
        };
        (@step_game $harness:ident) => {
            $harness.game.run(&$harness.outside_client).await.unwrap();
        };
        (@run $harness:ident { $($normal:tt)* }) => {
            $($normal)*
        };
        ($harness:ident, [ $(@$kind:tt { $($val:tt)* };)+ ]) => {
            $(game_steps!(@$kind $harness $($val)*));+
        };
    }

    macro_rules! async_test {
        (async fn $name:ident() $($tt:tt)*) => {
            #[test]
            fn $name() {
                use tracing_subscriber::layer::SubscriberExt;
                use tracing_subscriber::util::SubscriberInitExt;
                use tracing::Instrument;

                let filter = tracing_subscriber::filter::EnvFilter::from_default_env();
                let fmt_layer = tracing_subscriber::fmt::layer()
                    .with_timer(tracing_subscriber::fmt::time::uptime())
                    .with_level(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_test_writer()
                    .pretty();

                let _ = tracing_subscriber::registry()
                    .with(filter)
                    .with(fmt_layer)
                    .try_init();

                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    $($tt)*
                }.instrument(tracing::info_span!("Running test", name = stringify!($name))));

                rt.shutdown_background();
            }
        };
    }

    async_test!(
        async fn check_initial_game_creation() {
            let mut harness = SimpleTestHarness::new(None, ServerAnswers::default());
            harness.game.run(&harness.outside_client).await.unwrap();

            assert!(!harness.game.game_states.is_empty());
        }
    );

    async_test!(
        async fn check_initial_game_zones() {
            let mut harness = SimpleTestHarness::new(
                None,
                ServerAnswers {
                    get_player_keeping: Some(Box::new(|players| players)),
                    ..Default::default()
                },
            );
            harness.game.run(&harness.outside_client).await.unwrap();
            let state = harness.game.latest_gamestate();

            let first_player = harness.player_order.first().copied().unwrap();

            assert_eq!(harness.player_order.len() * 3 + 2, state.zones.len());
            assert_eq!(
                simple_deck().len(),
                state
                    .zones
                    .get(&ZoneId::Library(first_player))
                    .unwrap()
                    .objects
                    .len()
                    + state
                        .zones
                        .get(&ZoneId::Hand(first_player))
                        .unwrap()
                        .objects
                        .len()
            );
        }
    );

    async_test!(
        async fn check_game_starts_with_initial_player_order() {
            let mut harness = SimpleTestHarness::new(
                None,
                ServerAnswers {
                    ..Default::default()
                },
            );
            harness.game.run(&harness.outside_client).await.unwrap();

            let state = harness.game.latest_gamestate();

            assert_eq!(&state.active_player_order, &harness.player_order);
        }
    );

    async_test!(
        async fn check_game_mulligan() {
            let mut harness = SimpleTestHarness::new(
                None,
                ServerAnswers {
                    ..Default::default()
                },
            );
            let player = harness.player_order.first().unwrap().clone();

            game_steps!(
                harness,
                [
                    @set {
                        get_player_keeping = move |mut players| {
                            players.retain(|p| p != &player);
                            players
                        }
                    };
                    @step_game { };
                    @set {
                        get_player_keeping = |players| {
                            players
                        }
                    };
                    @step_game { };
                ]
            );

            let state = harness.game.latest_gamestate();
            assert!(
                matches!(state.game_stage, crate::GameStage::GameRunning),
                "Game is still not running!"
            );
            assert_eq!(
                6,
                state
                    .zones
                    .get(&ZoneId::Hand(player))
                    .unwrap()
                    .objects
                    .len()
            );
        }
    );

    async_test!(
        async fn check_game_asks_for_player_actions() {
            let mut harness = SimpleTestHarness::new(
                None,
                ServerAnswers {
                    ..Default::default()
                },
            );

            harness.game.run(&harness.outside_client).await.unwrap();
        }
    );

    async_test!(
        async fn check_game_player_plays_card() {
            let mut harness = SimpleTestHarness::new(
                None,
                ServerAnswers {
                    ..Default::default()
                },
            );

            harness.game.run(&harness.outside_client).await.unwrap();
        }
    );
}
