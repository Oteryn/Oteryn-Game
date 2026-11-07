//! Timed World participation is a descendant of the committed native death.
use super::*;
use crate::durability::world_event::changes::timed::{NativeKillOutcome, NativeKillRequest};

pub(super) fn capture(
    runtime: &ChannelRuntimeV1,
    facts: &ProjectedCreatureDeathFacts,
    target: ExactActorRef,
    damage: &[crate::foundation::RewardDamageShare],
) -> Option<Box<NativeKillRequest>> {
    let source = runtime.timed_world_raid_source(target).ok()??;
    let mut participants: Vec<_> = damage
        .iter()
        .filter(|share| {
            share.total > 0 && share.lease_generation.is_some() && share.session.is_some()
        })
        .map(|share| share.character)
        .collect();
    if facts.principal_last_damage_at_ms.is_some() {
        participants.push(facts.principal.character);
    }
    participants.sort_by_key(|id| *id.as_bytes());
    participants.dedup();
    if participants.is_empty() || participants.len() > 17 {
        return None;
    }
    let participants = participants
        .iter()
        .map(|id| crate::domain::CharacterId::from_bytes(*id.as_bytes()))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(Box::new(NativeKillRequest {
        source,
        occurrence: crate::durability::party_kill_xp::encode_death_key(facts.death),
        occurred_at_ms: i64::try_from(facts.died_at_unix_ms).ok()?,
        participants,
    }))
}

pub(super) async fn settle(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> bool {
    let Some(event) = &entry.timed else {
        return true;
    };
    let current = {
        let runtime = admission.runtime.lock().await;
        let digest = runtime.content_pin().server_artifact_digest();
        let revision = format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        (runtime.owner_fence().is_ok() && revision == entry.ground.content_revision).then(|| {
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
        })
    };
    let Some((scope, generation)) = current else {
        return false;
    };
    match admission
        .root
        .credit_timed_world_kill(
            admission.character,
            admission.holder,
            scope,
            generation,
            admission.reset_epoch,
            (**event).clone(),
        )
        .await
    {
        Ok(NativeKillOutcome::Applied | NativeKillOutcome::AlreadyApplied) => true,
        Ok(NativeKillOutcome::Refused) => {
            operator_event("world_event_native_credit_refused reason=stale_source_or_cycle");
            true
        }
        Ok(NativeKillOutcome::Pending) | Err(_) => false,
    }
}
