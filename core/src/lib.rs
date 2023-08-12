use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use card::Card;
use card::CardId;
use effect::EffectInfo;
use effect::ExecuteFailure;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

pub mod card;
pub mod effect;
pub mod meta;
pub mod outside;

pub fn get_seeded_uuid(rng: &mut impl Rng) -> uuid::Uuid {
    let mut random_bytes: [u8; 16] = [0; 16];
    rand::Fill::try_fill(&mut random_bytes, rng).unwrap();

    uuid::Builder::from_random_bytes(random_bytes).into_uuid()
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct GameId(Uuid);

impl GameId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
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
        player: PlayerId,
        from: ZoneId,
        object: ObjectId,
        choices: HashMap<(usize, String), EffectInfo>,
    },
    ResetPriority,
    PopStack,
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("Game was already running when a GameStart atom was sent")]
    GameAlreadyRunning,
    #[error("An RPC error was encountered")]
    RPCError(#[from] tarpc::client::RpcError),
    #[error("A keep hand atom was generated during normal game running")]
    KeepHandDuringGame,
    #[error("An invalid action was selected")]
    InvalidAction {
        list_length: usize,
        selected_action: usize,
    },
    #[error("The expected object ({object:?}) could not be found in {zone:?}")]
    ObjectNotFoundInZone { zone: ZoneId, object: ObjectId },
    #[error("A player was marked as passing although they either already passed, or its not their moment to pass")]
    InvalidPlayerPassing { player: PlayerId },
    #[error("An object was expected to contain an underlying card, but it did not")]
    NoUnderlyingCard { object: ObjectId },
    #[error("A card id was given without the existing card underneath")]
    CardNotFound { card: CardId },
    #[error("A response was given with more or less than the required amount")]
    InvalidChoiceAmount { expected: usize, received: usize },
    #[error("An effect failed to execute")]
    EffectExecuteFailure {
        #[source]
        failure: ExecuteFailure,
    },
    #[error("A given card was not implemented correctly")]
    InvalidCardState,
}

pub enum VerificationError {
    PlayerInvalidCard { id: PlayerId, card: CardId },
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
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub initial_cards: Vec<CardId>,
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
    pub objects: Objects,
}

impl GameZone {
    pub fn empty() -> GameZone {
        GameZone {
            objects: Objects(vec![]),
        }
    }

    pub fn with(objects: Vec<GameObject>) -> GameZone {
        GameZone {
            objects: Objects(objects),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct LibraryCardId(uuid::Uuid);

impl LibraryCardId {
    pub fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct ObjectId(pub uuid::Uuid);

impl ObjectId {
    pub fn new(rng: &mut impl Rng) -> Self {
        Self(get_seeded_uuid(rng))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameObject {
    pub id: ObjectId,
    /// An identifier for that tracks a card throughout the whole game, no two objects in the same
    /// game should have the same library card id
    pub library_card_id: Option<LibraryCardId>,
    /// The identifier of the card it represents if any, several objects may have the same card id
    pub underlying_card: Option<CardId>,
    /// Objects only have a controller on the stack and battlefield
    pub controller: Option<PlayerId>,
    /// Any choices associated to the object
    pub choices: HashMap<(usize, String), EffectInfo>,
}
impl GameObject {
    pub fn from_card(rand: &mut impl Rng, underlying_card: CardId) -> GameObject {
        GameObject {
            id: ObjectId::new(rand),
            library_card_id: Some(LibraryCardId::new(rand)),
            underlying_card: Some(underlying_card),
            controller: None,
            choices: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum GameStage {
    KeepHand { players_keeping: HashSet<PlayerId> },
    GameRunning,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameState {
    pub zones: hashbrown::HashMap<ZoneId, GameZone>,
    /// The turn order, index 0 is the active player
    pub active_player_order: Vec<PlayerId>,
    /// Players who have not yet passed since the last stack-modifying action
    pub unpassed_players: Vec<PlayerId>,
    pub game_stage: GameStage,
}
impl GameState {
    pub fn get_hand(&self, p: PlayerId) -> &GameZone {
        self.zones.get(&ZoneId::Hand(p)).unwrap()
    }

    pub fn get_stack(&self) -> &GameZone {
        self.zones.get(&ZoneId::Stack).unwrap()
    }

    pub fn get_battlefield(&self) -> &GameZone {
        self.zones.get(&ZoneId::Battlefield).unwrap()
    }

    pub fn get_object_from_zone(&self, from: ZoneId, obj: ObjectId) -> Option<&GameObject> {
        let zone = self.zones.get(&from)?;
        zone.objects.iter().find(|o| o.id == obj)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    #[serde(skip)]
    pub cards: Arc<std::collections::HashMap<CardId, Card>>,
    pub id: GameId,
    pub players: std::collections::HashMap<PlayerId, Player>,
    pub rand: rand_xoshiro::Xoshiro256StarStar,
    pub game_states: Vec<GameState>,
    pub history: Vec<(usize, Vec<GameAtom>)>,
}

impl Game {
    pub fn latest_gamestate(&self) -> &GameState {
        self.game_states.last().unwrap()
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
}
