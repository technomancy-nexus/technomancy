use std::collections::HashMap;

use crate::{Game, GameAtom, ObjectId, TargetId};

pub enum EffectTrigger {
    OnSelfCast,
}

pub enum Effect {
    Instant(Box<dyn InstantEffect>),
    Continuous(ContinuousEffect),
}

pub enum EffectInfoRequest {
    SingleTarget { restriction: Option<()> },
}

pub enum EffectInfo {
    SingleTarget(TargetId),
}

pub enum ExecuteFailure {
    InvalidEffectInfo { name: String },
    NoControllerFound,
}

#[async_trait::async_trait]
pub trait InstantEffect {
    fn get_required_info(&self) -> HashMap<String, EffectInfoRequest>;

    async fn execute(
        &self,
        info: HashMap<String, EffectInfo>,
        source: ObjectId,
        game: &Game,
    ) -> Result<Vec<GameAtom>, ExecuteFailure>;
}

pub enum ContinuousEffect {}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use crate::{GameAtom, ObjectId};

    use super::{EffectInfo, EffectInfoRequest, ExecuteFailure, InstantEffect};

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
