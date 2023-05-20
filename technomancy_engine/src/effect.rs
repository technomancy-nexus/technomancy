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
}

pub trait InstantEffect {
    fn get_required_info(&self) -> HashMap<String, EffectInfoRequest>;

    fn execute(
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

    impl InstantEffect for DealDamage {
        fn get_required_info(&self) -> HashMap<String, EffectInfoRequest> {
            [(
                String::from("target"),
                EffectInfoRequest::SingleTarget { restriction: None },
            )]
            .into()
        }

        fn execute(
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
}
