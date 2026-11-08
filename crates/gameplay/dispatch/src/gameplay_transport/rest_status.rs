//! GAP-SV7 C10 read-only resting and food projection from the native physical owner.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::rest_status::CAPABILITY_REST_STATUS_V1;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_REST_STATUS_V1)
        .collect()
}

macro_rules! authority_methods {
    () => {
        fn rest_status_available(&self) -> bool {
            false
        }
        fn rest_status_query(
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
    pub(super) async fn rest_status_page(
        &self,
        admitted: AdmittedSession,
        _command_id: u64,
    ) -> Option<Vec<u8>> {
        use crate::domain::condition::ConflictKey;
        use oteryn_protocol_oteryn::rest_status::RestStatus;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_REST_STATUS_V1)
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.runtime_scope != actor.runtime_scope()
        {
            return None;
        }
        let mut runtime = self.runtime.lock().await;
        if fence.scope_ownership_generation != runtime.binding().scope_generation() {
            return None;
        }
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || runtime
                .player_control_facts(actor, admitted.game_session_id)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let inside = super::actor_spell::qualified_protection_zone(
            &mut runtime,
            self.movement_cells(),
            actor,
        )?;
        let states = self.states.spell_states.try_lock().ok()?;
        let state = states.get(&runtime, actor, admitted.game_session_id)?;
        // Sample after acquiring the owner state: a condition installed during the
        // runtime-lock wait must not be observed against an earlier clock.
        let now = self.owner_now().get();
        if !state.owned_conditions().accepts_time(now) {
            return None;
        }
        let streak = self
            .states
            .resting_areas
            .streak(admitted.game_session_id, actor)?;
        let premium = state.hunting_premium_current(now);
        let effective_streak = if premium { streak } else { streak.min(3) };
        let active = inside && state.vitals().health != 0;
        let food_remaining_ms = match state
            .owned_conditions()
            .active_at(ConflictKey::FoodRegeneration, now)
        {
            Some(condition) => match condition.remaining_duration_ms(now)? {
                // active_at already proves this finite instance has not expired.
                0 => 1,
                remaining => remaining,
            },
            None => 0,
        };
        RestStatus {
            inside_resting_area: inside,
            streak,
            food_remaining_ms,
            health_multiplier: if !active || effective_streak < 2 {
                0
            } else if effective_streak >= 5 {
                2
            } else {
                1
            },
            mana_multiplier: if !active || effective_streak < 3 {
                0
            } else if effective_streak >= 6 {
                2
            } else {
                1
            },
            stamina: active && premium && streak >= 4,
            soul: active && premium && streak >= 7,
        }
        .encode()
    }
}

macro_rules! composed_methods {
    () => {
        fn rest_status_available(&self) -> bool {
            self.bundle_map().is_some()
        }
        async fn rest_status_query(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
        ) -> Option<Vec<u8>> {
            self.rest_status_page(admitted, command_id).await
        }
    };
}
pub(super) use composed_methods;
