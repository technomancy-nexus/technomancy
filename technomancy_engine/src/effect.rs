use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};

use crate::{Game, GameAtom, ObjectId, TargetId};

#[derive(Debug)]
pub enum EffectTrigger {
    /// These are the 'main' card effects. This is only useful on cards played onto the stack.
    ///
    /// For cards staying on the battlefield this is for example usually empty.
    OnResolve,
    /// This effect triggers whenever a card is played onto the stack
    OnPlay,
    /// This effect triggers whenever a player draws a card
    ///
    /// Note: This does not trigger when something 'moves' between zones.
    OnDraw,
}

#[derive(Debug)]
pub enum Effect {
    Instant(Box<dyn InstantEffect>),
    Continuous(ContinuousEffect),
}

#[derive(Debug)]
pub enum EffectInfoRequest {
    SingleTarget { restriction: Option<()> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffectInfo {
    SingleTarget(TargetId),
}

#[derive(Debug, thiserror::Error)]
pub enum ExecuteFailure {
    #[error("An invalid effect info was given for {}", .name)]
    InvalidEffectInfo { name: String },
    #[error("No controller was found for effect")]
    NoControllerFound,
}

#[async_trait::async_trait]
pub trait InstantEffect: Debug + Sync + Send {
    fn get_required_info(&self) -> HashMap<String, EffectInfoRequest>;

    async fn execute(
        &self,
        info: HashMap<String, EffectInfo>,
        source: ObjectId,
        game: &Game,
    ) -> Result<Vec<GameAtom>, ExecuteFailure>;
}

#[derive(Debug)]
pub enum ContinuousEffect {}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use crate::{GameAtom, ObjectId};

    use super::{EffectInfo, EffectInfoRequest, ExecuteFailure, InstantEffect};

    #[derive(Debug)]
    pub struct DealDamage(pub usize);

    #[async_trait::async_trait]
    impl InstantEffect for DealDamage {
        fn get_required_info(&self) -> HashMap<String, EffectInfoRequest> {
            [(
                String::from("target"),
                EffectInfoRequest::SingleTarget { restriction: None },
            )]
            .into()
        }

        async fn execute(
            &self,
            info: HashMap<String, EffectInfo>,
            source: ObjectId,
            _game: &crate::Game,
        ) -> Result<Vec<GameAtom>, ExecuteFailure> {
            let Some(EffectInfo::SingleTarget(target)) = info.get("target") else {
                return Err(ExecuteFailure::InvalidEffectInfo { name: "target".into() });
            };

            Ok(vec![GameAtom::DealDamage {
                amount: self.0,
                source,
                target: *target,
            }])
        }
    }

    /// For effects that say "You draw X cards"
    #[derive(Debug)]
    pub struct DrawCards(pub usize);

    #[async_trait::async_trait]
    impl InstantEffect for DrawCards {
        fn get_required_info(&self) -> HashMap<String, EffectInfoRequest> {
            Default::default()
        }

        async fn execute(
            &self,
            _info: HashMap<String, EffectInfo>,
            source: ObjectId,
            game: &crate::Game,
        ) -> Result<Vec<GameAtom>, ExecuteFailure> {
            Ok(vec![GameAtom::DrawCards {
                count: self.0,
                player: game
                    .get_controller_of(source)
                    .ok_or(ExecuteFailure::NoControllerFound)?,
            }])
        }
    }
}
