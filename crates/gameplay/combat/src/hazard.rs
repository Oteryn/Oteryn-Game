//! Current party minimum for hazard combat. Membership comes from the existing World cache.
use oteryn_gameplay_encounter_rules::hazard::gnomprona_contains;
use oteryn_gameplay_entities::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

/// A missing present member cannot increase a party's hazard difficulty or rewards.
pub fn effective_level(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<u16> {
    let points = |actor, session| {
        let at = runtime.read_actor_position(actor).ok()?.position();
        if !gnomprona_contains(at.x, at.y, at.floor) {
            return None;
        }
        runtime.hazard_level(actor, session)
    };
    let mut level = points(actor, session)?;
    if let Ok(Some(party)) = runtime.cached_world_party(actor, session)
        && party.party_id.is_some()
    {
        if !party.is_well_formed() {
            return None;
        }
        let census = runtime.positioned_actor_census().ok()?;
        for member in &party.members {
            if member.presence
                == oteryn_gameplay_entities::carrier::runtime_actor_party::PartyPresence::Offline
            {
                continue;
            }
            let (actor, _, session) = census.iter().find(|(actor, _, session)| {
                session.is_some_and(|session| {
                    runtime
                        .bound_attacker_lease(*actor, session)
                        .is_ok_and(|lease| lease.character_id().as_bytes() == &member.character)
                })
            })?;
            level = level.min(points(*actor, (*session)?)?);
        }
    }
    Some(level)
}
