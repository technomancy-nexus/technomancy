use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::game_dsl::{
    EvaluationError, Expression, ExpressionKind, GameDsl, Method, MethodFunction, SimpleMethod,
};

#[derive(Debug, Clone, Default)]
pub struct Card;
impl Card {
    fn as_game_object(&self) -> Expression {
        Expression::GameObject(crate::game_dsl::GameObject {
            kind: String::from("card"),
            methods: [].into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct GameActionInput;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ZoneName(String);

impl std::borrow::Borrow<str> for ZoneName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

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
    StartGame,
    AddPlayer(Player),
    AddZone(ZoneName),
    AddCardsTo {
        target_zone: TargetZone,
        cards: Vec<Card>,
    },
    AddRule {
        game_event: GameEvent,
        game_rule: GameRule,
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

    fn as_game_object(&self) -> Expression {
        #[derive(Debug, Clone)]

        struct GetZoneMethod {
            zones: HashSet<ZoneName>,
        }

        impl SimpleMethod for GetZoneMethod {
            fn call(
                &self,
                args: Vec<Expression>,
            ) -> Result<Expression, crate::game_dsl::EvaluationError> {
                let zone = args.first().ok_or(EvaluationError::InvalidType {
                    expected: ExpressionKind::String,
                    found: ExpressionKind::Void,
                })?;

                match zone {
                    Expression::String(val) => {
                        if self.zones.contains(val.as_str()) {
                            return Ok(todo!());
                        } else {
                            return Ok(Expression::Void);
                        }
                    }
                    _ => {
                        return Err(EvaluationError::InvalidType {
                            expected: ExpressionKind::String,
                            found: ExpressionKind::Void,
                        });
                    }
                }
            }
        }

        Expression::GameObject(crate::game_dsl::GameObject {
            kind: "player".to_string(),
            methods: [(
                String::from("get_zone"),
                Method::new(
                    Box::new(GetZoneMethod {
                        zones: self.zones.keys().cloned().collect(),
                    }),
                    crate::game_dsl::ExpressionKind::MethodCall {
                        arguments: vec![ExpressionKind::String],
                        return_kind: Box::new(ExpressionKind::GameObject("zone".to_string())),
                    },
                ),
            )]
            .into_iter()
            .collect(),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GameUpdateError {
    #[error("An unknown player id was given")]
    UnknownPlayer(PlayerId),
    #[error("An unknown zone id was given")]
    UnknownZone(ZoneName),
    #[error("Could not succesfully evaluate")]
    Evaluation(#[from] crate::game_dsl::EvaluationError),
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
            GameAction::AddRule {
                game_event,
                game_rule,
            } => {
                new_state
                    .rules
                    .entry(game_event)
                    .or_default()
                    .push(game_rule);
            }
            GameAction::StartGame => {
                let no_rules = &vec![];
                let applicable_rules = new_state
                    .rules
                    .get(&GameEvent::GameStart)
                    .unwrap_or(no_rules);

                let mut actions: Vec<GameDsl> = vec![];

                for rule in applicable_rules {
                    actions.extend(rule.on_trigger.clone());
                }

                let mut context = new_state.get_evaluation_context();

                for action in actions {
                    action.evaluate(&mut context)?;
                }
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

    fn get_evaluation_context(&self) -> crate::game_dsl::EvaluationContext {
        let players: Vec<Expression> = self
            .players()
            .iter()
            .map(|(_, ply)| ply.as_game_object())
            .collect();

        let game = Expression::GameObject(crate::game_dsl::GameObject {
            kind: "game".to_string(),
            methods: [(
                String::from("all_players"),
                Method::new(
                    todo!(),
                    ExpressionKind::MethodCall {
                        arguments: vec![],
                        return_kind: Box::new(ExpressionKind::Array(Box::new(
                            ExpressionKind::GameObject("player".to_string()),
                        ))),
                    },
                ),
            )]
            .into_iter()
            .collect(),
        });

        crate::game_dsl::EvaluationContext {
            values: [("game".to_string(), game)].into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TurnStepName(String);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum TurnEvent {
    TurnStart,
    TurnStep(TurnStepName),
    TurnEnd,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum GameEvent {
    GameStart,
    TurnEvent {
        player_id: PlayerId,
        turn_event: TurnEvent,
    },
}

#[derive(Debug, Clone)]
pub struct GameRule {
    on_trigger: Vec<GameDsl>,
}

#[derive(Debug, Clone, Default)]
pub struct GameState {
    pub(crate) configured_zones: HashSet<ZoneName>,
    pub(crate) players: HashMap<PlayerId, Player>,
    pub(crate) rules: HashMap<GameEvent, Vec<GameRule>>,
}

#[cfg(test)]
mod tests {
    use crate::{
        game_dsl::GameDsl,
        game_state::{
            Card, GameAction, GameActionInput, GameEvent, GameRule, GameState, Player, PlayerId,
            TargetZone, ZoneName,
        },
    };

    macro_rules! run_game {
        (@input $input:expr) => { $input };
        (@input) => { GameActionInput::default() };
        ($game:expr => [ $($update:expr $(=> $input:expr)?),+ $(,)? ]) => {{
            let game = &$game;
            $( let game = game.update( $update, run_game!(@input $($input)?) ).unwrap(); )+
            game
        }};
    }

    #[test]
    fn check_adding_players() {
        let old_state = GameState::default();

        let new_state = run_game!( old_state => [
            GameAction::AddPlayer(Player {
                id: PlayerId::new(),
                ..Default::default()
            }),
        ]);

        assert_eq!(new_state.players().len(), 1);
        assert_eq!(old_state.players().len(), 0);
    }

    #[test]
    fn check_adding_zones_and_cards_to_players() {
        let old_state = GameState::default();

        let first_player = PlayerId::new();
        let new_state = run_game!(old_state => [
                GameAction::AddPlayer(Player {
                    id: first_player,
                    ..Default::default()
                }),
                GameAction::AddZone(ZoneName(String::from("hand"))),
                GameAction::AddCardsTo {
                    target_zone: TargetZone {
                        player_id: first_player,
                        zone_name: ZoneName(String::from("hand")),
                    },
                    cards: vec![Card],
                },
        ]);

        assert_eq!(
            new_state.players()[&first_player].zones()[&ZoneName(String::from("hand"))].len(),
            1
        );
    }

    #[test]
    fn check_start_game_rules() {
        let old_state = GameState::default();

        let first_player = PlayerId::new();
        let new_state = run_game!(old_state => [
                GameAction::AddPlayer(Player {
                    id: first_player,
                    ..Default::default()
                }),
                GameAction::AddZone(ZoneName(String::from("hand"))),
                GameAction::AddZone(ZoneName(String::from("deck"))),
                GameAction::AddCardsTo {
                    target_zone: TargetZone {
                        player_id: first_player,
                        zone_name: ZoneName(String::from("deck")),
                    },
                    cards: vec![Card; 10],
                },
                GameAction::AddRule {
                    game_event: GameEvent::GameStart,
                    game_rule: GameRule { on_trigger: vec![ GameDsl::parse_from(r#"
                        for player in game.all_players() {
                            let deck = player.get_zone("deck");
                            let hand_cards = deck.take_cards_from_top(7);
                            let hand = players.get_zone("hand");
                            hand.add_cards_to_start(hand_cards);
                        }
                    "#).unwrap() ] }
                },
                GameAction::StartGame,
        ]);

        assert_eq!(
            new_state.players()[&first_player].zones()[&ZoneName(String::from("hand"))].len(),
            7
        );
    }
}
