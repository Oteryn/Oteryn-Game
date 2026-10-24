//! Resolve durable World selection before the session-independent spawn loop and wire reply.
use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn initialize_cyclopedia_respawn(
        &self,
    ) -> Result<(), ()> {
        let Some(area) = self.cyclopedia_content() else {
            return Ok(());
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Err(());
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        runtime.owner_fence().map_err(|_| ())?;
        let improved = self
            .root
            .prepare_cyclopedia_epoch(
                self.character,
                self.holder,
                &permit,
                crate::durability::character_cyclopedia::CyclopediaWorldBinding {
                    scope: RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                    generation: binding.scope_generation().get(),
                    bundle: area.digest,
                    epoch: self.reset_epoch,
                },
            )
            .await
            .map_err(|_| ())?;
        // Oteryn default, not Reference-qualified: only explicitly normal Bestiary creatures
        // with complete details; unknown, boss, familiar and reward creatures stay excluded.
        let normal: std::collections::BTreeSet<_> = native
            .creature_profiles()
            .records
            .iter()
            .filter_map(|record| {
                let crate::content::ProjectV2AuthoringProfileData::Creature(creature) =
                    &record.profile.data
                else {
                    return None;
                };
                let details = creature.details.as_ref()?;
                (creature.bestiary.is_some()
                    && creature.bosstiary.is_none()
                    && creature.familiar.is_none()
                    && details.bosstiary.is_none()
                    && !details.system_eligibility.reward_boss
                    && !details.summoning.is_familiar)
                    .then_some(record.profile.target.key.as_str())
            })
            .collect();
        runtime
            .apply_cyclopedia_respawn(|cell, key| {
                improved
                    && normal.contains(key)
                    && area.tiles.contains(&Point {
                        x: cell.x,
                        y: cell.y,
                        floor: -cell.floor,
                    })
            })
            .map_err(|_| ())?;
        Ok(())
    }
}
