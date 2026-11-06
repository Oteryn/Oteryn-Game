//! CL6 read-only projection. Existing owners remain authoritative.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::mechanics_status::CAPABILITY_MECHANICS_STATUS_V1;
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_MECHANICS_STATUS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn mechanics_status_available(&self) -> bool {
            false
        }
        fn mechanics_status_query(
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
    pub(super) async fn mechanics_status_page(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_MECHANICS_STATUS_V1)
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
        // No runtime or spell-state guard crosses this durable await.
        let mut observation = self
            .root
            .read_mechanics_status(self.character, self.holder, fence)
            .await
            .ok()?;
        let runtime = self.runtime.lock().await;
        if !super::battle_instinct_status::actor_is_current(&runtime, admitted)
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation().get() != fence.scope_ownership_generation.get()
        {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        let actor = admitted.runtime_actor?;
        let now = self.owner_now().get();
        let state = states.get(&runtime, actor, admitted.game_session_id)?;
        let conditions = state.owned_conditions();
        if !conditions.accepts_time(now) {
            return None;
        }
        for instance in conditions.instances() {
            let definition = instance.definition();
            if oteryn_gameplay_progression::condition_potions::PotionCondition::from_key(
                definition.key(),
            )
            .is_none()
                || conditions
                    .active_at(definition.condition_type().conflict_key(), now)
                    .is_none()
            {
                continue;
            }
            observation
                .status
                .timers
                .push(oteryn_protocol_oteryn::mechanics_status::EffectTimer {
                    key: definition.key().to_owned(),
                    remaining_ms: u64::try_from(instance.remaining_duration_ms(now)?.max(1))
                        .ok()?,
                    online_only: false,
                });
        }
        observation.status.timers.sort_by(|a, b| a.key.cmp(&b.key));
        let census = runtime.positioned_actor_census().ok()?;
        for (member, live) in observation
            .status
            .members
            .iter_mut()
            .zip(observation.live_members)
        {
            let Some((session, generation)) = live else {
                continue;
            };
            let target = census.iter().find_map(|(target, _, held)| {
                if *held != Some(session) {
                    return None;
                }
                let lease = runtime.bound_attacker_lease(*target, session).ok()?;
                (lease.character_id().as_bytes() == &member.character
                    && lease.generation() == generation)
                    .then_some(*target)
            });
            let Some(target) = target else {
                continue;
            };
            member.conditions =
                super::condition_status::snapshot(&runtime, &states, target, session, now);
            if member.conditions.is_some() {
                let state = states.get(&runtime, target, session)?;
                let buffs = state.virtue_party_bonuses(now);
                member.bonuses = [
                    buffs.damage_reduction,
                    buffs.auto_attack,
                    buffs.spell_damage,
                    buffs.spell_healing,
                    buffs.mantra,
                ];
                let owned = state.owned_conditions();
                member.party_spell_remaining_ms = party_spell_remaining(owned, now);
            }
        }
        observation.status.encode()
    }
}
pub(super) fn party_spell_remaining(
    conditions: &oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
    now: u64,
) -> [u32; 5] {
    let mut remaining = [0; 5];
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
        if let Some(ms) = instance
            .remaining_duration_ms(now)
            .and_then(|ms| u32::try_from(ms).ok())
        {
            remaining[index] = ms.max(1);
        }
    }
    remaining
}
macro_rules! composed_methods {
    () => {
        fn mechanics_status_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn mechanics_status_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.mechanics_status_page(admitted).await
        }
    };
}
pub(super) use composed_methods;
