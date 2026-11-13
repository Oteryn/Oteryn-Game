//! C08: read the admitted actor's two existing condition owners under their locks.
use super::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, capabilities::OfferedCapability,
    connection::AdmittedSession,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_domain::domain::condition::{
    ConditionGate, ConditionType, DotElement, StatusKind,
};
use oteryn_gameplay_foundation::foundation::condition::ConditionStore;
use oteryn_protocol_oteryn::condition_status::{
    ActiveCondition, CAPABILITY_CONDITION_STATUS_V1, ConditionStatus,
};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_CONDITION_STATUS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn condition_status_available(&self) -> bool {
            false
        }
        fn condition_status_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

fn catalogue(kind: ConditionType) -> Option<(u8, u8)> {
    Some(match kind {
        ConditionType::DamageOverTime(element) => (
            match element {
                DotElement::Poison => 1,
                DotElement::Fire => 2,
                DotElement::Energy => 3,
                DotElement::Bleeding => 4,
                DotElement::Drown => 5,
                DotElement::Freezing => 6,
                DotElement::Dazzled => 7,
                DotElement::Cursed => 8,
            },
            0,
        ),
        ConditionType::Status(status) => (
            match status {
                StatusKind::Invisible => 9,
                StatusKind::Drunk => 10,
                StatusKind::Rooted => 11,
                StatusKind::Feared => 12,
            },
            0,
        ),
        ConditionType::Invisible => (9, 0),
        ConditionType::Haste => (13, 0),
        ConditionType::Paralysis => (14, 0),
        ConditionType::FoodRegeneration => (15, 0),
        ConditionType::Recovery => (16, 0),
        ConditionType::ManaShield => (17, 0),
        ConditionType::Light => (18, 0),
        ConditionType::Attributes => (19, 0),
        ConditionType::SpellSkills(id) => (20, id),
        ConditionType::SpellRegeneration(id) => (21, u8::try_from(id).ok()?),
        ConditionType::Outfit => (22, 0),
        ConditionType::IntenseHex => (23, 0),
        ConditionType::Gate(gate) => (
            match gate {
                ConditionGate::Muted => 24,
                ConditionGate::Pacified => 25,
                ConditionGate::Powerless => 26,
            },
            0,
        ),
        ConditionType::Agony => (27, 0),
    })
}
fn append<S: Clone>(
    conditions: &mut std::collections::BTreeMap<(u8, u8), i64>,
    store: &ConditionStore<S>,
    now: u64,
) -> Option<()> {
    for (kind, remaining) in store.visible_statuses(now) {
        // The monster-only source marker is not a byte-sized player icon.
        // Refuse an unrepresentable player snapshot instead of truncating it.
        let key = catalogue(kind)?;
        conditions
            .entry(key)
            .and_modify(|current| {
                *current = if *current == -1 || remaining == -1 {
                    -1
                } else {
                    (*current).max(remaining)
                };
            })
            .or_insert(remaining);
    }
    Some(())
}
pub(super) fn snapshot(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    now: u64,
) -> Option<ConditionStatus> {
    if runtime
        .player_control_facts(actor, session)
        .ok()?
        .control_loss
        .is_some()
    {
        return None;
    }
    let state = states.get(runtime, actor, session)?;
    if !state.owned_conditions().accepts_time(now) {
        return None;
    }
    let native = runtime
        .actor_conditions_at(actor, Some(session), now)
        .ok()?;
    let mut conditions = std::collections::BTreeMap::new();
    append(&mut conditions, state.owned_conditions(), now)?;
    append(&mut conditions, native, now)?;
    Some(ConditionStatus {
        conditions: conditions
            .into_iter()
            .map(|((icon, sub_id), remaining_ms)| ActiveCondition {
                icon,
                sub_id,
                remaining_ms,
            })
            .collect(),
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn condition_status_page(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CONDITION_STATUS_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let runtime = self.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        snapshot(
            &runtime,
            &states,
            actor,
            admitted.game_session_id,
            self.owner_now().get(),
        )?
        .encode()
    }
}
macro_rules! composed_methods {
    () => {
        fn condition_status_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some()
        }
        async fn condition_status_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.condition_status_page(admitted).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::super::actor_spell::tests::{FACTS, runtime_with_player};
    use super::*;
    use oteryn_simulation_determinism::SemanticTimeMicros;

    #[test]
    fn condition_status_owner_projects_live_gate_then_expiry_and_rejects_stale_session()
    -> Result<(), String> {
        let (runtime, actor, session) = runtime_with_player(0x7b);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                FACTS,
                (0, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize")?;
        super::super::actor_spell::tests::gate_player(&mut states, actor, session, "muted");
        let live = snapshot(&runtime, &states, actor, session, 1_000_000).ok_or("live status")?;
        assert_eq!(
            live.conditions,
            vec![ActiveCondition {
                icon: 24,
                sub_id: 0,
                remaining_ms: 9_000
            }]
        );
        assert_eq!(
            snapshot(&runtime, &states, actor, session, 10_000_000)
                .ok_or("expired")?
                .conditions,
            []
        );
        let mut stale_id = *session.as_bytes();
        stale_id[15] ^= 1;
        let stale = GameSessionId::decode(&stale_id).map_err(|e| format!("{e:?}"))?;
        assert!(snapshot(&runtime, &states, actor, stale, 1_000_000).is_none());
        Ok(())
    }
}

#[cfg(test)]
mod srx3_source_marker_tests {
    use super::*;
    #[test]
    fn srx3_source_marker_cannot_be_truncated_into_player_condition_status() {
        assert_eq!(catalogue(ConditionType::SpellRegeneration(88_888)), None);
        assert_eq!(
            catalogue(ConditionType::SpellRegeneration(1)),
            Some((21, 1))
        );
    }
}
