#![allow(dead_code, clippy::too_many_arguments)]
use std::{collections::HashMap, sync::Arc};

use outside::OutsideGameClient;
use rand::{seq::SliceRandom, Rng};
use rand_xoshiro::Xoshiro256StarStar;
use technomancy_core::{
    card::{Card, CardEffect, CardId, TriggeredCardEffect},
    effect::{Effect, EffectInfo, EffectInfoRequest, EffectTrigger},
    Game, GameAtom, GameError, GameId, GameObject, GameStage, GameState, GameZone, ObjectId,
    Player, PlayerAction, PlayerId, TargetId, VerificationError, ZoneId,
};
use tracing::trace;

use crate::outside::OutsideGame;

pub mod card;
pub mod effect;
pub mod outside;

fn assert_send<'u, R>(
    fut: impl 'u + Send + std::future::Future<Output = R>,
) -> impl 'u + Send + std::future::Future<Output = R> {
    fut
}

#[derive(Debug)]
pub struct GameImplV1 {
    game: Game,
}

impl GameImplV1 {
    pub fn new(
        id: GameId,
        mut rand: Xoshiro256StarStar,
        cards: Arc<std::collections::HashMap<CardId, Card>>,
        players: std::collections::HashMap<PlayerId, Player>,
        order: Vec<PlayerId>,
    ) -> GameImplV1 {
        let initial_game_state = new_game_state_with(&mut rand, &players, &order);
        GameImplV1 {
            game: Game {
                id,
                cards,
                players,
                rand,
                game_states: vec![initial_game_state],
                history: vec![],
            },
        }
    }

    pub fn verify(&self) -> Result<(), Vec<VerificationError>> {
        let mut errors = vec![];

        for (id, player) in &self.game.players {
            for card in &player.initial_cards {
                if !self.game.cards.contains_key(card) {
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

    pub fn latest_gamestate(&self) -> &GameState {
        self.game.latest_gamestate()
    }

    pub fn apply_atoms(&mut self, atoms: Vec<GameAtom>) -> Result<(), GameError> {
        self.game
            .history
            .push((self.game.game_states.len() - 1, atoms.clone()));
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
                    TargetId::Player(_ply) => {
                        todo!("Do something with health {amount}")
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
                    library.objects.shuffle(&mut self.game.rand);
                }
                GameAtom::DrawCards { player, count } => {
                    let Some([hand, library]) = next_state.zones.get_many_mut([&ZoneId::Hand(player), &ZoneId::Library(player)]) else { unreachable!() };
                    let new_count = library.objects.len().saturating_sub(count);
                    hand.objects.extend(library.objects.drain(new_count..));
                }
                GameAtom::PassPriority { player } => {
                    if next_state.unpassed_players.first() == Some(&player) {
                        next_state.unpassed_players.remove(0);
                    } else {
                        return Err(GameError::InvalidPlayerPassing { player });
                    }
                }
                GameAtom::PlayerPlayCard {
                    player,
                    from,
                    object,
                    choices,
                } => {
                    let from_id = from;
                    let Some([from, to]) = next_state.zones.get_many_mut([&from, &ZoneId::Stack]) else { unreachable!() };
                    if let Some(obj_idx) = from.objects.iter().position(|o| o.id == object) {
                        let mut obj = from.objects.remove(obj_idx);
                        obj.choices = choices;
                        obj.controller = Some(player);
                        to.objects.push(obj);
                    } else {
                        return Err(GameError::ObjectNotFoundInZone {
                            zone: from_id,
                            object,
                        });
                    }
                }
                GameAtom::ResetPriority => {
                    next_state.unpassed_players = next_state.active_player_order.clone();
                }
                GameAtom::PopStack => {
                    next_state
                        .zones
                        .get_mut(&ZoneId::Stack)
                        .unwrap()
                        .objects
                        .pop();
                }
            }
        }
        self.game.game_states.push(next_state);
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip_all, fields(game = ?self.game.id), err)]
    pub async fn run(&mut self, outside: &OutsideGameClient) -> Result<(), GameError> {
        match self.latest_gamestate().game_stage.clone() {
            GameStage::KeepHand { players_keeping } => {
                trace!("Checking for potential mulligans");
                let latest_gamestate = self.latest_gamestate();
                let atoms: Vec<_> = self
                    .game
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
                    .game
                    .players
                    .keys()
                    .filter(|p| !players_keeping.contains(p))
                    .copied()
                    .collect();
                let players_keeping =
                    assert_send(outside.get_player_keeping(players_not_kept_yet)).await?;

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

                if players_keeping.len() == self.game.players.len() {
                    trace!("All players have kept, we can start the game");
                    self.apply_atoms(vec![GameAtom::StartGame])?;
                    return Ok(());
                }
            }
            GameStage::GameRunning => {
                let latest_gamestate = self.latest_gamestate();

                let stack = latest_gamestate.get_stack();

                if latest_gamestate.unpassed_players.is_empty() {
                    // All players passed, resolve the top most stack item
                    trace!("All players passed");

                    if let Some(top_item) = stack.objects.last() {
                        // Resolve!
                        trace!(?top_item.id, "Attemption resolution");
                        let card = top_item.underlying_card.as_ref().ok_or(
                            GameError::NoUnderlyingCard {
                                object: top_item.id,
                            },
                        )?;

                        let card = self
                            .game
                            .cards
                            .get(card)
                            .ok_or(GameError::CardNotFound { card: *card })?;

                        let resolve_effects = card
                            .behaviour
                            .effects
                            .iter()
                            .filter_map(|e| match e {
                                CardEffect::Triggered(TriggeredCardEffect {
                                    trigger: EffectTrigger::OnResolve,
                                    effects,
                                }) => Some(effects),
                                _ => None,
                            })
                            .flatten()
                            .enumerate()
                            .collect::<Vec<_>>();

                        let mut atoms = vec![];
                        for (idx, effect) in resolve_effects {
                            if let Effect::Instant(eff) = effect {
                                let info = top_item
                                    .choices
                                    .iter()
                                    .filter(|((i, _), _)| *i == idx)
                                    .map(|((_, k), v)| (k.clone(), v.clone()))
                                    .collect();

                                let effect_atoms =
                                    assert_send(eff.execute(info, top_item.id, &self.game))
                                        .await
                                        .map_err(|e| GameError::EffectExecuteFailure {
                                            failure: e,
                                        })?;
                                atoms.extend(effect_atoms);
                            }
                        }

                        atoms.push(GameAtom::PopStack);
                        atoms.push(GameAtom::ResetPriority);

                        self.apply_atoms(atoms)?;
                    } else {
                        // Pass phases/turns
                        todo!()
                    }
                } else {
                    let active_player = latest_gamestate.unpassed_players.first().unwrap();

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
                    let action_idx = assert_send(
                        outside
                            .get_next_player_action_from(*active_player, possible_actions.clone()),
                    )
                    .await?;

                    let Some(action) = possible_actions.get(action_idx) else {
                        return Err(GameError::InvalidAction { list_length: possible_actions.len(), selected_action: action_idx });
                    };

                    trace!(?action, "Player selected action");

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

                            // Step 2

                            let latest_gamestate = self.latest_gamestate();

                            let active_player =
                                latest_gamestate.active_player_order.first().unwrap();

                            let obj = latest_gamestate
                                .get_object_from_zone(*from, *object)
                                .ok_or(GameError::ObjectNotFoundInZone {
                                    zone: *from,
                                    object: *object,
                                })?;

                            let card = obj
                                .underlying_card
                                .as_ref()
                                .ok_or(GameError::NoUnderlyingCard { object: *object })?;

                            let card = self
                                .game
                                .cards
                                .get(card)
                                .ok_or(GameError::CardNotFound { card: *card })?;

                            let resolve_effects = card
                                .behaviour
                                .effects
                                .iter()
                                .filter_map(|e| match e {
                                    CardEffect::Triggered(TriggeredCardEffect {
                                        trigger: EffectTrigger::OnResolve,
                                        effects,
                                    }) => Some(effects),
                                    _ => None,
                                })
                                .flatten()
                                .enumerate()
                                .collect::<Vec<_>>();

                            let mut gathered_info = HashMap::new();
                            for (idx, e) in resolve_effects {
                                match e {
                                    Effect::Continuous(_) => {
                                        return Err(GameError::InvalidCardState)
                                    }
                                    Effect::Instant(instant) => {
                                        let required_info = instant.get_required_info();
                                        for (name, question) in required_info {
                                            match question {
                                                EffectInfoRequest::SingleTarget { restriction } => {
                                                    if restriction.is_some() {
                                                        todo!()
                                                    } else {
                                                        // Without any restrictions targets can
                                                        // _only_ be agents on the battlefield _or_
                                                        // players
                                                        let mut possible_choices = vec![];
                                                        possible_choices.extend(
                                                            self.game
                                                                .players
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
                                                        let choices = assert_send(
                                                            outside.get_target_choices_from_given(
                                                                *active_player,
                                                                *object,
                                                                name.clone(),
                                                                possible_choices.clone(),
                                                                1,
                                                            ),
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
                                                            EffectInfo::SingleTarget(
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
                            // Calculate costs
                            // Step 4
                            // Pay costs
                            // Step 5

                            let player_passing =
                                assert_send(outside.get_player_passing(*active_player)).await?;

                            let mut atoms = vec![GameAtom::PlayerPlayCard {
                                player: *active_player,
                                from: *from,
                                object: *object,
                                choices: gathered_info,
                            }];
                            atoms.extend(player_passing.then_some(GameAtom::PassPriority {
                                player: *active_player,
                            }));
                            self.apply_atoms(atoms)?;
                        }
                    }
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
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256StarStar;
    use tarpc::{server::Channel, transport::channel::UnboundedChannel, ClientMessage, Response};
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use technomancy_core::{
        card::{
            BaseCardKind, Card, CardBehaviour, CardEffect, CardId, CardKind, Cost,
            TriggeredCardEffect,
        },
        effect::{Effect, EffectTrigger},
        outside::{Outside, OutsideClient, OutsideRequest, OutsideResponse},
        GameId, ObjectId, Player, PlayerAction, PlayerId, TargetId, ZoneId,
    };

    use crate::{
        effect::tests::{DealDamage, DrawCards},
        outside::OutsideGameClient,
        GameImplV1,
    };

    const BLAST_CARD: uuid::Uuid = uuid::uuid!("4abc4619-b61c-44a4-9d37-8a31bda65b48");
    const DRAW_CARD: uuid::Uuid = uuid::uuid!("ddfbf54b-2750-41c6-b657-1d6ce1e754ef");

    #[allow(unused)]
    fn check_send() {
        let mut harness = SimpleTestHarness::new(None, ServerAnswers::default());
        crate::assert_send(harness.game_impl.run(&harness.outside_client));
    }

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
                    trigger: EffectTrigger::OnResolve,
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
                    trigger: EffectTrigger::OnResolve,
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
        (
            right,
            OutsideGameClient {
                game_id,
                client: Arc::new(client),
            },
        )
    }

    struct ServerAnswers {
        get_player_keeping: Option<Box<dyn FnMut(Vec<PlayerId>) -> Vec<PlayerId> + Send>>,
        get_next_player_action_from:
            Option<Box<dyn FnMut(PlayerId, Vec<PlayerAction>) -> usize + Send>>,
        get_target_choices_from_given: Option<
            Box<dyn FnMut(PlayerId, ObjectId, String, Vec<TargetId>, usize) -> Vec<usize> + Send>,
        >,
        get_player_passing: Option<Box<dyn FnMut(PlayerId) -> bool + Send>>,
    }

    impl Default for ServerAnswers {
        fn default() -> Self {
            Self {
                get_player_keeping: Some(Box::new(|players| players)),
                get_next_player_action_from: Default::default(),
                get_target_choices_from_given: Default::default(),
                get_player_passing: Default::default(),
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
                .expect("No method set: get_player_keeping")(asked_players)
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
                .expect("No method set: get_next_player_action_from")(
                player, player_actions
            )
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
                .expect("No method set: get_target_choices_from_given")(
                player, source, name, choices, count,
            )
        }

        async fn get_player_passing(
            self,
            _context: tarpc::context::Context,
            _game_id: GameId,
            player: PlayerId,
        ) -> bool {
            self.answers
                .lock()
                .await
                .get_player_passing
                .as_mut()
                .expect("No method set: get_player_passing")(player)
        }
    }

    struct SimpleTestHarness {
        player_order: Vec<PlayerId>,
        game_impl: GameImplV1,
        outside_client: OutsideGameClient,
        answers: Arc<Mutex<ServerAnswers>>,
    }

    fn init_harness(
        seed: Option<u64>,
    ) -> (
        Vec<PlayerId>,
        GameImplV1,
        tarpc::transport::channel::UnboundedChannel<
            tarpc::ClientMessage<OutsideRequest>,
            tarpc::Response<OutsideResponse>,
        >,
        OutsideGameClient,
    ) {
        let rand = Xoshiro256StarStar::seed_from_u64(seed.unwrap_or(1337));
        let players = playtesters();
        let player_order: Vec<_> = players.keys().copied().collect();
        let cards = existing_cards();

        let id = GameId::new();
        let game_impl = GameImplV1::new(id, rand, Arc::new(cards), players, player_order.clone());

        let (server, outside_client) = outside_client(game_impl.game.id);

        (player_order, game_impl, server, outside_client)
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
            let (player_order, game_impl, server, outside_client) = init_harness(seed);

            (
                SimpleTestHarness {
                    player_order,
                    game_impl,
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
        (@unset $harness:ident) => {
            *$harness.answers.lock().await = ServerAnswers::default();
        };
        (@step_game $harness:ident) => {
            $harness.game_impl.run(&$harness.outside_client).await.unwrap();
        };
        (@run $harness:ident $($normal:tt)*) => {
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
            harness
                .game_impl
                .run(&harness.outside_client)
                .await
                .unwrap();

            assert!(!harness.game_impl.game.game_states.is_empty());
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
            harness
                .game_impl
                .run(&harness.outside_client)
                .await
                .unwrap();
            let state = harness.game_impl.latest_gamestate();

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
            harness
                .game_impl
                .run(&harness.outside_client)
                .await
                .unwrap();

            let state = harness.game_impl.latest_gamestate();

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
            let player = *harness.player_order.first().unwrap();

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

            let state = harness.game_impl.latest_gamestate();
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
        async fn check_game_player_plays_card() {
            let mut harness = SimpleTestHarness::new(
                Some(1234),
                ServerAnswers {
                    ..Default::default()
                },
            );

            let player = *harness.player_order.first().unwrap();

            game_steps!(
                harness,
                [
                    @set {
                        get_player_keeping = |players| {
                            players
                        }
                    };
                    @step_game {};
                    @run {
                        assert_eq!(
                            harness.game_impl.latest_gamestate().game_stage,
                            crate::GameStage::GameRunning
                        );
                    };
                    @set {
                        get_next_player_action_from = |_player, player_actions| {
                            let id = ObjectId(Uuid::from_str("2eaec1b5-94a9-4994-b038-54826e4e3ca6").unwrap());
                            player_actions.iter().position(|i| matches!(i, PlayerAction::PlayCard { object, ..} if *object == id)).unwrap()
                        }
                    };
                    @set {
                        get_target_choices_from_given = | player: PlayerId, _source: ObjectId, _name: String, choices: Vec<TargetId>, _count: usize,| {
                            choices.iter().enumerate().filter(|(_, c)| match c { TargetId::Player(ply) => *ply != player, _ => false }).map(|(idx, _c)| idx).collect()
                        }
                    };
                    @set {
                        get_player_passing = |_player: PlayerId| { true }
                    };
                    @step_game {};
                    @run {
                        let state = harness.game_impl.latest_gamestate();
                        assert_eq!(state.get_stack().objects.len(), 1);
                    };
                    @unset {};
                    @set {
                        get_next_player_action_from = |_player, _player_actions| {
                            0
                        }
                    };
                    @step_game {};
                    @step_game {};
                    @step_game {};
                    @step_game {};
                    @run {
                        let state = harness.game_impl.latest_gamestate();
                        assert_eq!(state.get_hand(player).objects.len(), 7);
                    };
                ]
            );
        }
    );
}
