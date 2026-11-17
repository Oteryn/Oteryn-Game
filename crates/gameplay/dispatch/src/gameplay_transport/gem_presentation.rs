//! Read-only catalogue from the real Atelier owner and executed modifier calculations.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_progression::{wheel::base_vocation, wheel_gem_data::WheelGemData};
use oteryn_protocol_oteryn::gem_presentation as wire;
#[path = "gem_presentation/catalogue.rs"]
mod catalogue;

async fn current(owner: &ComposedFreshAdmission<'_, '_, '_>, a: AdmittedSession) -> bool {
    let runtime = owner.runtime.lock().await;
    let (Some(actor), Some(fence)) = (a.runtime_actor, a.item_fence) else {
        return false;
    };
    if fence.game_session_id != a.game_session_id
        || fence.connection_generation.get() != a.continuity.connection_generation
        || runtime.owner_fence().is_err()
    {
        return false;
    }
    if matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) {
        return runtime
            .house_actor_state(actor, a.game_session_id)
            .is_ok_and(|state| {
                state.scope == fence.runtime_scope
                    && state.generation == fence.scope_ownership_generation
                    && state.lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && state.lease.generation() == fence.character_lease_generation
            })
            && runtime
                .house_player_control_facts(actor, a.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none());
    }
    let binding = runtime.binding();
    binding.world_id() == a.world_id
        && binding.runtime_scope() == fence.runtime_scope
        && binding.scope_generation() == fence.scope_ownership_generation
        && runtime
            .player_control_facts(actor, a.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        && runtime
            .bound_attacker_lease(actor, a.game_session_id)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}
pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: AdmittedSession,
    r: wire::Request,
) -> Option<Vec<u8>> {
    if !a.continuity.selected_capabilities.contains(148)
        || !current(owner, a).await
    {
        return None;
    }
    let expected = a.item_fence?;
    let before = owner.current_quest_fence(a.game_session_id).await.ok()??;
    if before.character_id != expected.character_id
        || before.game_session_id != expected.game_session_id
        || before.connection_generation != expected.connection_generation
        || before.character_lease_generation != expected.character_lease_generation
        || before.runtime_scope != expected.runtime_scope
        || before.scope_ownership_generation != expected.scope_ownership_generation
    {
        return None;
    }
    let state = owner
        .root
        .read_character_atelier(owner.character, before.character_id)
        .await
        .ok()??;
    if state.revision != r.revision {
        return None;
    }
    let data = WheelGemData::embedded().ok()?;
    let vocation = base_vocation(&state.vocation)?;
    let modifiers = catalogue::project(&data, vocation)?;
    let after = owner.current_quest_fence(a.game_session_id).await.ok()??;
    if before != after || !current(owner, a).await {
        return None;
    }
    wire::Reply {
        grades_current: state.grade_ruleset_revision.as_deref() == Some(data.revision()),
        qualified_gems: state
            .gems
            .values()
            .filter(|g| g.ruleset_revision == data.revision())
            .map(|g| g.id.to_be_bytes())
            .collect(),
        request: r,
        catalogue: format!("{}@{}", data.revision(), data.source_candidate_sha256()),
        vocation: vocation.into(),
        modifiers,
    }
    .encode()
}
macro_rules! authority_methods {
    () => {
        fn gem_presentation_available(&self) -> bool {
            false
        }
        fn gem_presentation_query(
            &self,
            _a: $crate::gameplay_transport::connection::AdmittedSession,
            _r: oteryn_protocol_oteryn::gem_presentation::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn gem_presentation_available(&self) -> bool {
            oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded().is_ok()
        }
        async fn gem_presentation_query(
            &self,
            a: connection::AdmittedSession,
            r: oteryn_protocol_oteryn::gem_presentation::Request,
        ) -> Option<Vec<u8>> {
            gem_presentation::execute(self, a, r).await
        }
    };
}
pub(super) use composed_methods;
