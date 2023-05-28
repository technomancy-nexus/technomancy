#![allow(dead_code)]
use std::collections::HashSet;

use card::{Card, CardId};
use outside::{OutsideGame, OutsideGameClient};
use rand::{seq::SliceRandom, Fill, Rng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};
use tarpc::client::RpcError;
use uuid::Uuid;

pub mod card;
pub mod effect;
pub mod outside;

/// The technomancy engine
pub struct Engine {
    cards: std::collections::HashMap<CardId, Card>,
    games: std::collections::HashMap<GameId, Game>,
}

impl Engine {
    pub fn new(cards: std::collections::HashMap<CardId, Card>) -> Self {
        Self {
            cards,
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
}
impl GameObject {
    fn from_card(rand: &mut impl Rng, underlying_card: CardId) -> GameObject {
        GameObject {
            id: ObjectId::new(rand),
            library_card_id: Some(LibraryCardId::new(rand)),
            underlying_card: Some(underlying_card),
            controller: None,
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
    active_player_order: Vec<PlayerId>,
    game_stage: GameStage,
}
impl GameState {
    fn get_hand(&self, p: PlayerId) -> &GameZone {
        self.zones.get(&ZoneId::Hand(p)).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    id: GameId,
    players: std::collections::HashMap<PlayerId, Player>,
    rand: Xoshiro256StarStar,
    game_states: Vec<GameState>,
    history: Vec<(usize, Vec<GameAtom>)>,
}

impl Game {
    pub fn new(
        mut rand: Xoshiro256StarStar,
        players: std::collections::HashMap<PlayerId, Player>,
        order: Vec<PlayerId>,
    ) -> Self {
        let initial_game_state = new_game_state_with(&mut rand, &players, &order);
        Self {
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
            }
        }
        self.game_states.push(next_state);
        Ok(())
    }

    async fn run(&mut self, outside: &OutsideGameClient) -> Result<(), GameError> {
        while let GameStage::KeepHand { players_keeping } =
            self.latest_gamestate().game_stage.clone()
        {
            if players_keeping.len() == self.players.len() {
                self.apply_atoms(vec![GameAtom::StartGame])?;
                continue;
            }
            let latest_gamestate = self.latest_gamestate();
            let atoms: Vec<_> = self
                .players
                .keys()
                .filter(|p| !players_keeping.contains(p))
                .flat_map(|p| {
                    let hand = latest_gamestate.get_hand(*p);

                    match hand.objects.len() {
                        count @ 2.. => vec![
                            GameAtom::ShuffleHandIntoLibrary { player: *p },
                            GameAtom::DrawCards {
                                player: *p,
                                count: count - 1,
                            },
                        ],
                        1 => vec![
                            GameAtom::ShuffleHandIntoLibrary { player: *p },
                            GameAtom::KeepHand { player: *p },
                        ],
                        0 => vec![GameAtom::DrawCards {
                            player: *p,
                            count: 7,
                        }],
                        _ => unreachable!(),
                    }
                })
                .collect();
            self.apply_atoms(atoms)?;

            let latest_gamestate = self.latest_gamestate();

            let GameStage::KeepHand { players_keeping } = &latest_gamestate.game_stage else {
                unreachable!()
            };

            let players = self
                .players
                .keys()
                .filter(|p| !players_keeping.contains(p))
                .copied()
                .collect();
            let players_keeping = outside.get_player_keeping(players).await?;

            self.apply_atoms(
                players_keeping
                    .into_iter()
                    .map(|p| GameAtom::KeepHand { player: p })
                    .collect(),
            )?;
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
    use std::collections::HashMap;

    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256StarStar;
    use tarpc::server::Channel;

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
        Engine, Game, GameId, OutsideGameClient, Player, PlayerId, ZoneId,
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
                    effect: vec![Effect::Instant(Box::new(DealDamage(3)))],
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
                    effect: vec![Effect::Instant(Box::new(DrawCards(3)))],
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

    #[derive(Clone)]
    struct OutsideServer(Vec<PlayerId>);

    #[tarpc::server]
    impl Outside for OutsideServer {
        async fn get_player_order(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
        ) -> Vec<PlayerId> {
            self.0
        }

        async fn get_player_keeping(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
            asked_players: Vec<PlayerId>,
        ) -> Vec<PlayerId> {
            asked_players
        }
    }

    struct SimpleTestHarness {
        engine: Engine,
        player_order: Vec<PlayerId>,
        game: Game,
        outside_client: OutsideGameClient,
        outside_server: tokio::task::JoinHandle<()>,
    }

    impl SimpleTestHarness {
        fn new(seed: Option<u64>) -> Self {
            let mut rand = Xoshiro256StarStar::seed_from_u64(seed.unwrap_or(1337));
            let players = playtesters(&mut rand);
            let player_order: Vec<_> = players.keys().copied().collect();
            let engine = new_engine();

            let game = Game::new(rand, players, player_order.clone());

            let (server, outside_client) = outside_client(game.id);

            let server = tarpc::server::BaseChannel::with_defaults(server);
            let outside_server =
                tokio::spawn(server.execute(OutsideServer(player_order.clone()).serve()));

            SimpleTestHarness {
                engine,
                player_order,
                game,
                outside_client,
                outside_server,
            }
        }
    }

    macro_rules! async_test {
        (async fn $name:ident() $($tt:tt)*) => {
            #[test]
            fn $name() {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async {
                    $($tt)*
                });

                rt.shutdown_background();
            }
        };
    }

    async_test!(
        async fn check_initial_game_creation() {
            let mut harness = SimpleTestHarness::new(None);
            harness.game.run(&harness.outside_client).await.unwrap();

            assert!(!harness.game.game_states.is_empty());
        }
    );

    async_test!(
        async fn check_initial_game_zones() {
            let mut harness = SimpleTestHarness::new(None);
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
        async fn check_game_asks_for_initial_player_order() {
            let mut harness = SimpleTestHarness::new(None);
            harness.game.run(&harness.outside_client).await.unwrap();

            let state = harness.game.latest_gamestate();

            assert_eq!(&state.active_player_order, &harness.player_order);
        }
    );
}
