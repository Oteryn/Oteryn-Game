//! Current party minimum and shared incoming Hazard scaling from the actual actor owner.
use crate::creature_auto_attack::AttackFacts;
use oteryn_gameplay_encounter_rules::hazard::{
    HazardAreaEffects, HazardError, gnomprona_runtime_contains, provisional_effects,
};
use oteryn_gameplay_entities::{ChannelRuntimeV1, ExactActorRef, GameSessionId};

const WORLD_BUNDLE_FRAME: &str = "oteryn:frame/world-bundle-v3";
const CRITICAL_INTERVAL_US: u64 = 2_000_000;

fn source_position(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> bool {
    runtime.read_actor_position(actor).is_ok_and(|snapshot| {
        let at = snapshot.position();
        gnomprona_runtime_contains(WORLD_BUNDLE_FRAME, at.x, at.y, at.floor)
    })
}

/// A missing present member cannot increase a party's hazard difficulty or rewards.
pub fn effective_level(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Option<u16> {
    let points = |actor, session| {
        if !runtime.hazard_world_frame(actor, session) || !source_position(runtime, actor) {
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

/// Both exact actors must be in the qualified source zone of this runtime.
/// The player projection proves the active map frame; summoned creatures are not hazard mobs.
pub fn effects_for_monster(
    runtime: &ChannelRuntimeV1,
    player: ExactActorRef,
    session: GameSessionId,
    monster: ExactActorRef,
) -> Option<HazardAreaEffects> {
    if !source_position(runtime, monster)
        || runtime
            .companion_snapshot_including_dead(monster)
            .ok()?
            .state
            .master
            .is_some()
    {
        return None;
    }
    provisional_effects(effective_level(runtime, player, session)?).ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncomingHazardDamage {
    pub amount: u32,
    pub critical: bool,
}

/// Crystal player.cpp:5458-5514. Read-only preflight: commits stamp the shared player
/// critical window only after the actual HP owner succeeds. Healing and mana drain never call it.
pub fn incoming_damage(
    runtime: &ChannelRuntimeV1,
    facts: &AttackFacts,
    amount: u32,
    draw_ppm: u32,
    now_us: u64,
    source_critical: bool,
) -> Result<IncomingHazardDamage, HazardError> {
    let unchanged = IncomingHazardDamage {
        amount,
        critical: false,
    };
    if amount == 0 {
        return Ok(unchanged);
    }
    let Some(effects) = effects_for_monster(runtime, facts.target, facts.session, facts.issuer)
    else {
        return Ok(unchanged);
    };
    let ready = runtime
        .last_hazard_critical_us(facts.target, facts.session)
        .is_none_or(|last| now_us.saturating_sub(last) >= CRITICAL_INTERVAL_US);
    let critical = ready && !source_critical && effects.creature_critical(draw_ppm);
    let base = if critical {
        effects.critical_damage(u64::from(amount))?
    } else {
        u64::from(amount)
    };
    let amount =
        u32::try_from(effects.creature_damage(base)?).map_err(|_| HazardError::Overflow)?;
    Ok(IncomingHazardDamage { amount, critical })
}
