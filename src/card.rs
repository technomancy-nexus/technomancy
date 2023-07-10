#[cfg(test)]
mod tests {
    use technomancy_core::{
        card::{
            AgentPower, AgentSubKind, AgentToughness, BaseCardKind, Card, CardBehaviour,
            CardEffect, CardId, CardKind, Cost, TriggeredCardEffect,
        },
        effect::{Effect, EffectTrigger},
    };

    use crate::effect::tests::DealDamage;

    #[allow(unused)]
    fn _simple_cards() {
        let simple_agent = Card {
            id: CardId::with(uuid::uuid!("33505f5e-dce1-4b29-914d-748375d79303")),
            behaviour: CardBehaviour {
                cost: Some(Cost {
                    ..Default::default()
                }),
                kind: vec![CardKind {
                    kind: BaseCardKind::Agent {
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
            behaviour: CardBehaviour {
                cost: Some(Cost {
                    corp1_scrip: 1,
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
    }
}
