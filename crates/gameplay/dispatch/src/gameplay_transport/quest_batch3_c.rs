//! Source boss-death quest continuations on the existing captured lethal queue.
//! Crystal 201189f HeartBossDeath:26-138; CourtWarlock:323-329;
//! MagmaBubbleDeath:12-23 and GoblinLeaderDeath:3-8.
//! Herald uses the present most-damage player's shared party (Crystal functions.lua:1064-1072).
use super::{kill_reward::PendingKillSettlement, quest_kills::Recipient};
use crate::durability::quest_state::{
    native_kills::{NativeDeathSource, NativeQuestDeath},
    source_deaths::SourceDeathContext,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use oteryn_gameplay_quests::quest::source_deaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: [u8; 16],
    pub lease_generation: u64,
    pub observed_revision: u64,
    pub objectives: Vec<&'static str>,
}

/// Called under the original lethal turn. The enclosing capture cache retains
/// this exact result for corpse/command replay; later entrants are never added.
pub(super) fn capture(
    runtime: &ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    principal_present: bool,
    snapshots: impl Iterator<Item = Snapshot>,
) -> Vec<Recipient> {
    if !source_deaths::tracks_creature(&entry.creature) {
        return Vec::new();
    }
    let Ok(census) = runtime.positioned_melee_census() else {
        return Vec::new();
    };
    let scope = entry.facts.death.runtime_scope();
    let generation = entry.facts.death.scope_ownership_generation().get();
    let principal = entry.facts.principal;
    if runtime.binding().runtime_scope() != scope || runtime.owner_fence().is_err() {
        return Vec::new();
    }
    let shared = (principal_present && entry.creature == "oteryn:creature.herald_of_fire")
        .then(|| {
            runtime
                .cached_world_party(principal.actor, principal.session)
                .ok()
                .flatten()
        })
        .flatten()
        .filter(|party| {
            party.character == *principal.character.as_bytes()
                && party.world == *scope.world_id().as_bytes()
                && party.party_id.is_some()
                && party.shared_xp_enabled
                && party.is_well_formed()
        })
        .cloned()
        .map(std::sync::Arc::new);
    let mut recipients = Vec::new();
    for snapshot in snapshots {
        if snapshot.observed_revision == 0
            || !runtime
                .bound_attacker_lease(snapshot.actor, snapshot.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &snapshot.character
                        && lease.generation() == snapshot.lease_generation
                })
        {
            continue;
        }
        let Some((_, at, _)) = census.iter().find(|(actor, _, session)| {
            *actor == snapshot.actor && *session == Some(snapshot.session)
        }) else {
            continue;
        };
        let at = at.position();
        let position = [at.x, at.y, -i32::from(at.floor)];
        let damaged = entry.damage_shares.iter().any(|share| {
            share.character.as_bytes() == &snapshot.character
                && share.total > 0
                && share.session == Some(snapshot.session)
                && share.lease_generation == Some(snapshot.lease_generation)
        });
        let top = principal_present
            && principal.actor == snapshot.actor
            && principal.session == snapshot.session
            && principal.character.as_bytes() == &snapshot.character
            && principal.lease_generation == snapshot.lease_generation;
        let shared_peer = shared.as_ref().filter(|party| {
            !top && party
                .members
                .iter()
                .any(|member| member.character == snapshot.character)
        });
        for key in &snapshot.objectives {
            let Some(rule) = source_deaths::rule(key) else {
                continue;
            };
            if rule.creature != entry.creature
                || !rule.recipient(position, damaged || shared_peer.is_some())
                || (rule.top_damage_only && !top && shared_peer.is_none())
            {
                continue;
            }
            let source = SourceDeathContext {
                scope,
                scope_generation: generation,
                character: snapshot.character,
                session: snapshot.session,
                lease_generation: snapshot.lease_generation,
                position,
                dealt_damage: damaged,
                top_damage_character: rule
                    .top_damage_only
                    .then_some(*principal.character.as_bytes()),
                shared_party: shared_peer.cloned(),
            };
            let source = if rule.creature == "oteryn:creature.herald_of_fire"
                && matches!(scope, RuntimeScopeRefV1::Instance { .. })
            {
                NativeDeathSource::HeraldInstance {
                    source,
                    death: entry.facts.death,
                    died_at_unix_ms: entry.facts.died_at_unix_ms,
                }
            } else {
                NativeDeathSource::Source(source)
            };
            recipients.push(Recipient {
                actor: snapshot.actor,
                session: snapshot.session,
                character: snapshot.character,
                lease_generation: snapshot.lease_generation,
                death: NativeQuestDeath {
                    transition: (*key).into(),
                    creature: entry.creature.clone(),
                    observed_revision: snapshot.observed_revision,
                    definition_hash: None,
                    source,
                },
            });
        }
    }
    recipients
}
