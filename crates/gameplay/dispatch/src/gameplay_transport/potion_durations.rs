//! NX-8 W09: read the active effect owners, never activation receipt history.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::potion_durations::{
    CAPABILITY_POTION_DURATIONS_V1, PotionDuration, PotionDurations,
};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_POTION_DURATIONS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn potion_durations_available(&self) -> bool {
            false
        }
        fn potion_durations_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn potion_durations_available(&self) -> bool {
            true
        }
        async fn potion_durations_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.potion_durations_page(admitted).await
        }
    };
}
pub(super) use composed_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn potion_durations_page(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_POTION_DURATIONS_V1)
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        let valid = |runtime: &crate::foundation::ChannelRuntimeV1| {
            let Ok(lease) = runtime.bound_attacker_lease(actor, admitted.game_session_id) else {
                return false;
            };
            fence.game_session_id == admitted.game_session_id
                && fence.connection_generation.get() == admitted.continuity.connection_generation
                && lease.character_id().as_bytes() == fence.character_id.as_bytes()
                && lease.generation() == fence.character_lease_generation
        };
        {
            let runtime = self.runtime.lock().await;
            if !valid(&runtime) {
                return None;
            }
        }
        let durable = self
            .root
            .read_active_potion_durations(
                self.character,
                self.holder,
                fence,
                *self.channel_id.as_bytes(),
            )
            .await
            .ok()?;
        let mut snapshot = PotionDurations {
            active: durable
                .into_iter()
                .map(|entry| PotionDuration {
                    source: entry.source,
                    item_id: entry.item_id,
                    percent: entry.percent,
                    remaining_ms: entry.remaining_ms,
                })
                .collect(),
        };
        let runtime = self.runtime.lock().await;
        if !valid(&runtime) {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        let state = states.get(&runtime, actor, admitted.game_session_id)?;
        let conditions = state.owned_conditions();
        let now = self.owner_now().get();
        for instance in conditions.instances() {
            let key = instance.definition().key();
            let Some(potion) =
                oteryn_gameplay_progression::condition_potions::PotionCondition::from_key(key)
            else {
                continue;
            };
            if conditions.active_at(instance.definition().condition_type().conflict_key(), now)
                != Some(instance)
            {
                continue;
            }
            let Some(remaining_ms) = instance
                .remaining_duration_ms(now)
                .filter(|remaining| *remaining > 0)
            else {
                continue;
            };
            let item_id = potion
                .key()
                .strip_prefix("oteryn:item.tibia.i")?
                .parse()
                .ok()?;
            snapshot.active.push(PotionDuration {
                source: 2,
                item_id,
                percent: 0,
                remaining_ms: u64::try_from(remaining_ms).ok()?,
            });
        }
        snapshot
            .active
            .sort_by_key(|entry| (entry.source, entry.item_id, entry.percent));
        snapshot.encode()
    }
}
