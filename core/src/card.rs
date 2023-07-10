use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effect::{Effect, EffectTrigger};

#[derive(Default, Debug)]
pub struct Cost {
    pub corp1_scrip: u64,
    pub corp2_scrip: u64,
    pub corp3_scrip: u64,
    pub corp4_scrip: u64,
    pub corp5_scrip: u64,
    pub any_scrip: u64,
}

#[derive(Debug)]
pub struct CardKind {
    pub kind: BaseCardKind,
}

#[derive(Debug)]
pub enum AgentSubKind {
    Mercenary,
}

#[derive(Debug)]
pub enum BuildingSubKind {}

#[derive(Debug)]
pub enum AgentPower {
    Fixed(u64),
    Special,
}

#[derive(Debug)]
pub enum AgentToughness {
    Fixed(u64),
    Special,
}

#[derive(Debug)]
pub enum BaseCardKind {
    Agent {
        subkind: AgentSubKind,
        power: AgentPower,
        toughness: AgentToughness,
    },
    Building {
        subkind: BuildingSubKind,
    },
    Quickhack,
    Program,
}

#[derive(Debug)]
pub struct TriggeredCardEffect {
    pub trigger: EffectTrigger,
    pub effects: Vec<Effect>,
}

#[derive(Debug)]
pub struct ActivatedCardEffect {
    pub cost: Cost,
    pub effect: Vec<Effect>,
}

#[derive(Debug)]
pub struct StaticCardEffect {
    pub effect: Effect,
}

#[derive(Debug)]
pub enum CardEffect {
    Triggered(TriggeredCardEffect),
    Activated(ActivatedCardEffect),
    Static(StaticCardEffect),
}

#[derive(Debug)]
pub struct CardBehaviour {
    pub cost: Option<Cost>,
    pub kind: Vec<CardKind>,
    pub effects: Vec<CardEffect>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct CardId(uuid::Uuid);

impl CardId {
    pub fn with(id: Uuid) -> CardId {
        Self(id)
    }
}

#[derive(Debug)]
pub struct Card {
    pub id: CardId,
    pub behaviour: CardBehaviour,
}
