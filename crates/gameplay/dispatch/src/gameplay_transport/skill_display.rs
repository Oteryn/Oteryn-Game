//! NX-7 complete read-only skills projection over the existing fenced owners.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::skill_display::{CAPABILITY_SKILL_DISPLAY_V1, SkillDisplay};
use oteryn_protocol_oteryn::skill_progress::SkillProgressSnapshot;
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_SKILL_DISPLAY_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn skill_display_available(&self) -> bool {
            false
        }
        fn skill_display_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn skill_display_page(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SKILL_DISPLAY_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let _permit = self.spell_lane_permit().await?;
        let native = self.active_generation?.native_gameplay()?;
        let formula = native.training_formula()?;
        let progression = native.progression()?;
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
        let (build, points, progression_state) = self
            .root
            .read_skill_display_state(
                self.character,
                self.holder,
                &fence,
                command,
                native.source_digest(),
            )
            .await
            .ok()?;
        let progression_state = progression_state?;
        let (level, experience) = (progression_state.level, progression_state.experience);
        let policy = progression
            .policy_for(
                &progression_state.profile_revision,
                &progression_state.ruleset_revision,
                &progression_state.content_revision,
            )
            .ok()?;
        let minimum = u64::try_from(
            policy
                .thresholds
                .iter()
                .find(|entry| entry.level == level)?
                .minimum_experience
                .get(),
        )
        .ok()?;
        let maximum = level
            .checked_add(1)
            .and_then(|next| policy.thresholds.iter().find(|entry| entry.level == next))
            .and_then(|entry| u64::try_from(entry.minimum_experience.get()).ok());
        let level_percent = maximum
            .and_then(|next| {
                experience
                    .checked_sub(minimum)
                    .zip(next.checked_sub(minimum))
            })
            .map_or(0, |(progress, required)| {
                super::skill_progress::percent(progress, required)
            });
        let skills = std::array::from_fn(|family| {
            let (level, progress) = if family == 0 {
                build.magic()
            } else {
                build.skills()[family - 1]
            };
            super::skill_progress::project_skill(formula, build.vocation(), family, level, progress)
        });
        let loyalty_levels = std::array::from_fn(|family| {
            let skill = skills[family];
            oteryn_gameplay_progression::loyalty::projected_level(
                formula,
                build.vocation(),
                family,
                skill.level,
                skill.progress,
                points,
            )
        });
        let equipment = self
            .read_admission_effects(admitted.game_session_id)
            .await
            .ok()
            .flatten();
        // Live values are optional enrichment. Refuse stale actor/session owners and never
        // substitute trained levels for missing live condition/equipment facts.
        let current_levels = {
            let runtime = self.runtime.lock().await;
            if !super::battle_instinct_status::actor_is_current(&runtime, admitted) {
                return None;
            }
            let states = self.states.spell_states.try_lock().ok()?;
            states
                .get(&runtime, admitted.runtime_actor?, admitted.game_session_id)
                .and_then(|state| {
                    let now = self.owner_now().get();
                    let (effects, _, _, _, _, equipment) = equipment.as_ref()?;
                    state.owned_effective_skills(&build, now).ok()?;
                    let adjustments = state.owned_skill_adjustments(now).ok()?;
                    let mut values = [0; 8];
                    values[0] = adjustments.adjust_magic_level(u32::from(loyalty_levels[0]))?;
                    for (index, (value, loyal)) in
                        values[1..].iter_mut().zip(&loyalty_levels[1..]).enumerate()
                    {
                        *value = adjustments.adjust_skill(u32::from(*loyal), index)?;
                    }
                    use oteryn_gameplay_content::content::item_abilities::AbilitySkill;
                    for (kind, delta) in &effects.skills {
                        let family = match kind {
                            AbilitySkill::MagicLevel => 0,
                            AbilitySkill::Fist => 1,
                            AbilitySkill::Club => 2,
                            AbilitySkill::Sword => 3,
                            AbilitySkill::Axe => 4,
                            AbilitySkill::Distance => 5,
                            AbilitySkill::Shield => 6,
                        };
                        values[family] =
                            u32::try_from((i128::from(values[family]) + i128::from(*delta)).max(0))
                                .ok()?;
                    }
                    let battle =
                        state.wheel_battle_instinct(&runtime, admitted.runtime_actor?, now)?;
                    for value in &mut values[1..5] {
                        *value = value.checked_add(battle.melee)?;
                    }
                    values[6] = values[6].checked_add(battle.shielding)?;
                    if let Some(catalogue) = &self.proficiency_catalogue
                        && let Some(key) = equipment
                            .items
                            .iter()
                            .filter(|item| matches!(item.slot, 5 | 6 | 10))
                            .find_map(|item| {
                                catalogue
                                    .resolve_weapon(&item.definition.production_key)
                                    .map(|weapon| weapon.canonical_item_key)
                            })
                        && let Ok(sessions) = self.states.proficiencies.lock()
                        && let Some(track) = sessions
                            .get(&admitted.game_session_id)
                            .and_then(|p| p.track(&key))
                        && let Some(perks) = catalogue.cast_perks(&key, track, None)
                    {
                        values[0] = values[0].checked_add(perks.skills[7])?;
                        for (value, bonus) in values[1..].iter_mut().zip(perks.skills) {
                            *value = value.checked_add(bonus)?;
                        }
                    }
                    Some(values)
                })
        };
        SkillDisplay {
            trained: SkillProgressSnapshot { skills },
            level,
            experience,
            level_percent,
            loyalty_bonus_percent: oteryn_gameplay_progression::loyalty::bonus_percent(points),
            loyalty_levels,
            current_levels,
        }
        .encode()
    }
}
macro_rules! composed_methods {
    () => {
        fn skill_display_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some_and(|native| {
                    native.training_formula().is_some() && native.progression().is_some()
                })
        }
        async fn skill_display_query(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
        ) -> Option<Vec<u8>> {
            self.skill_display_page(admitted, command_id).await
        }
    };
}
pub(super) use composed_methods;
