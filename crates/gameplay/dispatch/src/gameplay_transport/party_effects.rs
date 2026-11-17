//! GAP-SV7 S13c read-only own party effects.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::party_effects::{CAPABILITY_PARTY_EFFECTS_V1, PartyEffects};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_PARTY_EFFECTS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn party_effects_available(&self) -> bool {
            false
        }
        fn party_effects_query(
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
    pub(super) async fn party_effects_page(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_EFFECTS_V1)
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let icons = self.observe_party_skull(admitted).await?;
        let shield = icons
            .iter()
            .find(|icon| icon.entity.identity == actor.placement_identity())?
            .shield;
        let runtime = self.runtime.lock().await;
        let fence = admitted.item_fence?;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .ok()?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        let buffs = states
            .get(&runtime, actor, admitted.game_session_id)
            .map_or(Default::default(), |state| {
                state.virtue_party_bonuses(self.owner_now().get())
            });
        let mut snapshot = from_shield(
            shield,
            [
                buffs.damage_reduction,
                buffs.auto_attack,
                buffs.spell_damage,
                buffs.spell_healing,
                buffs.mantra,
            ],
        )?;
        let now = self.owner_now().get();
        if let Some(state) = states.get(&runtime, actor, admitted.game_session_id) {
            let conditions = state.owned_conditions();
            for instance in conditions.instances() {
                if conditions
                    .active_at(instance.definition().condition_type().conflict_key(), now)
                    .is_none()
                {
                    continue;
                }
                let index = match instance.definition().key() {
                    "candidate:spell/train_party/effect-attributes" => 0,
                    "candidate:spell/protect_party/effect-attributes" => 1,
                    "candidate:spell/enchant_party/effect-attributes" => 2,
                    "candidate:spell/heal_party/effect-regeneration" => 3,
                    "candidate:spell/enlighten_party/effect-regeneration" => 4,
                    _ => continue,
                };
                snapshot.party_spell_remaining_ms[index] =
                    u32::try_from(instance.remaining_duration_ms(now)?)
                        .ok()?
                        .max(1);
            }
        }
        snapshot.encode()
    }
}
fn from_shield(shield: u8, bonuses: [u32; 5]) -> Option<PartyEffects> {
    if !matches!(shield, 0 | 3..=10) {
        return None;
    }
    Some(PartyEffects {
        in_party: shield != 0,
        shared_enabled: matches!(shield, 5..=10),
        shared_active: matches!(shield, 5 | 6),
        eligible: matches!(shield, 5 | 6 | 9 | 10),
        bonuses,
        party_spell_remaining_ms: [0; 5],
    })
}
macro_rules! composed_methods {
    () => {
        fn party_effects_available(&self) -> bool {
            true
        }
        async fn party_effects_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.party_effects_page(admitted).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_effects_native_capability_negotiates_without_party_skull_display()
    -> Result<(), &'static str> {
        use crate::gameplay_transport::capabilities::{
            BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
        };
        let selected = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_PARTY_EFFECTS_V1],
        )
        .ok_or("selection")?;
        assert_eq!(selected.as_slice(), &[CAPABILITY_PARTY_EFFECTS_V1]);
        assert!(!selected.contains(oteryn_protocol_oteryn::party_skull::CAPABILITY_PARTY_SKULL_V1));
        Ok(())
    }

    #[test]
    fn party_effects_retains_disabled_waiting_and_active_shield_distinctions()
    -> Result<(), &'static str> {
        for (shield, enabled, active, eligible) in [
            (0, false, false, false),
            (3, false, false, false),
            (4, false, false, false),
            (5, true, true, true),
            (6, true, true, true),
            (7, true, false, false),
            (8, true, false, false),
            (9, true, false, true),
            (10, true, false, true),
        ] {
            let snapshot = from_shield(shield, [6, 12, 6, 12, 50]).ok_or("shield")?;
            assert_eq!(
                (
                    snapshot.shared_enabled,
                    snapshot.shared_active,
                    snapshot.eligible
                ),
                (enabled, active, eligible)
            );
            assert_eq!(snapshot.in_party, shield != 0);
            assert_eq!(snapshot.bonuses, [6, 12, 6, 12, 50]);
        }
        for shield in [1, 2, 11, 255] {
            assert!(from_shield(shield, [0; 5]).is_none());
        }
        Ok(())
    }
}
