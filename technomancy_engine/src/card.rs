use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effect::{Effect, EffectTrigger};

#[derive(Default)]
pub struct Cost {
    pub corp1_scrip: u64,
    pub corp2_scrip: u64,
    pub corp3_scrip: u64,
    pub corp4_scrip: u64,
    pub corp5_scrip: u64,
    pub any_scrip: u64,
}

pub struct CardKind {
    pub kind: BaseCardKind,
}

pub enum AgentSubKind {
    Mercenary,
}

pub enum BuildingSubKind {}

pub enum AgentPower {
    Fixed(u64),
    Special,
}

pub enum AgentToughness {
    Fixed(u64),
    Special,
}

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

pub struct TriggeredCardEffect {
    pub trigger: EffectTrigger,
    pub effect: Vec<Effect>,
}

pub struct ActivatedCardEffect {
    pub cost: Cost,
    pub effect: Vec<Effect>,
}

pub struct StaticCardEffect {
    pub effect: Effect,
}

pub enum CardEffect {
    Triggered(TriggeredCardEffect),
    Activated(ActivatedCardEffect),
    Static(StaticCardEffect),
}

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

pub struct Card {
    pub id: CardId,
    pub behaviour: CardBehaviour,
}

#[cfg(test)]
mod tests {
    use crate::effect::{tests::DealDamage, Effect, EffectTrigger};

    use super::{
        AgentPower, AgentSubKind, AgentToughness, Card, CardEffect, CardId, Cost,
        TriggeredCardEffect,
    };

    #[allow(unused)]
    fn _simple_cards() {
        let simple_agent = Card {
            id: CardId::with(uuid::uuid!("33505f5e-dce1-4b29-914d-748375d79303")),
            behaviour: super::CardBehaviour {
                cost: Some(Cost {
                    ..Default::default()
                }),
                kind: vec![super::CardKind {
                    kind: super::BaseCardKind::Agent {
                        subkind: AgentSubKind::Mercenary,
                        power: AgentPower::Fixed(3),
                        toughness: AgentToughness::Fixed(6),
                    },
                }],
                effects: vec![],
            },
        };

        let simple_quickhack = Card {
            id: CardId::with(uuid::uuid!("02663fb0-7eb5-4d4a-ad7f-a9397b7d7b13")),
            behaviour: super::CardBehaviour {
                cost: Some(Cost {
                    corp1_scrip: 1,
                    ..Default::default()
                }),
                kind: vec![super::CardKind {
                    kind: super::BaseCardKind::Quickhack,
                }],
                effects: vec![CardEffect::Triggered(TriggeredCardEffect {
                    trigger: EffectTrigger::OnSelfCast,
                    effect: vec![Effect::Instant(Box::new(DealDamage(3)))],
                })],
            },
        };
    }
}
