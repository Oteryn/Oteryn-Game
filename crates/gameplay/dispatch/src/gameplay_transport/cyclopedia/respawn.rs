//! Resolve each canonical main area's durable selection before the native spawn loop.
use super::*;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn initialize_cyclopedia_respawn(
        &self,
    ) -> Result<(), ()> {
        // Other admitted map features do not imply qualified W12 content.
        if self.cyclopedia_content().is_none() {
            return Ok(());
        }
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        let table = &map.overlay.base().subareas;
        if table.masks.is_empty() {
            return Ok(());
        }
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Err(());
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let binding = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            runtime.binding()
        };
        let entry = map.entry_start;
        let legacy_area = table.resolve(entry.x, entry.y, entry.floor).ok_or(())?.0;
        let mains: std::collections::BTreeSet<_> = table
            .masks
            .iter()
            .filter_map(|mask| {
                table
                    .areas
                    .iter()
                    .find(|area| area.id == mask.subarea)?
                    .parent
            })
            .collect();
        let mut improved = std::collections::BTreeSet::new();
        for area in mains {
            if self
                .root
                .prepare_cyclopedia_epoch(
                    self.character,
                    self.holder,
                    &permit,
                    crate::durability::character_cyclopedia::CyclopediaWorldBinding {
                        scope: binding.runtime_scope(),
                        generation: binding.scope_generation().get(),
                        bundle: map.overlay.base().digest(),
                        epoch: self.reset_epoch,
                        area,
                        legacy_area,
                    },
                )
                .await
                .map_err(|_| ())?
            {
                improved.insert(area);
            }
        }
        // Keep the existing admitted normal-creature eligibility boundary.
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
        let mut runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        let current = runtime.binding();
        if current.runtime_scope() != binding.runtime_scope()
            || current.scope_generation() != binding.scope_generation()
        {
            return Err(());
        }
        runtime
            .apply_cyclopedia_respawn(|cell, key| {
                normal.contains(key)
                    && u16::try_from(cell.x)
                        .ok()
                        .zip(u16::try_from(cell.y).ok())
                        .zip(i8::try_from(-cell.floor).ok())
                        .and_then(|((x, y), floor)| table.resolve(x, y, floor))
                        .is_some_and(|(area, _)| improved.contains(&area))
            })
            .map_err(|_| ())?;
        Ok(())
    }
}
