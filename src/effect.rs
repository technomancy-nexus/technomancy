#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use technomancy_core::effect::EffectInfo;
    use technomancy_core::effect::EffectInfoRequest;
    use technomancy_core::effect::ExecuteFailure;
    use technomancy_core::effect::InstantEffect;

    use crate::GameAtom;
    use crate::ObjectId;

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
