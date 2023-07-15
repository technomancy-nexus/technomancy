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
static_assertions::assert_impl_all!(Effect: Send, Sync);

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

static_assertions::assert_impl_all!(dyn InstantEffect: Send, Sync);
static_assertions::assert_obj_safe!(InstantEffect);

#[derive(Debug)]
pub enum ContinuousEffect {}
