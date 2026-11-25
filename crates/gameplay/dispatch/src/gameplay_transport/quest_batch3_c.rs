//! Source boss-death quest continuations on the existing captured lethal queue.
//! Crystal 201189f HeartBossDeath:26-138; CourtWarlock:323-329;
//! MagmaBubbleDeath:12-23 and GoblinLeaderDeath:3-8.
//! Herald's qualified local recipient is the present most-damage player only.
use super::{kill_reward::PendingKillSettlement, quest_kills::Recipient};
use crate::durability::quest_state::{
    native_kills::{NativeDeathSource, NativeQuestDeath},
    source_deaths::SourceDeathContext,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
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
    let Ok(rules) = source_deaths::rules() else {
        return Vec::new();
    };
    let mut rules = rules
        .iter()
        .filter(|rule| rule.matches_creature(&entry.creature))
        .peekable();
    if rules.peek().is_none() {
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
    // Use the newest observed roster for this party, including the leader's
    // projection when neither the top damager nor the kicked member has refreshed.
    let principal_view = if principal_present && rules.any(|rule| rule.top_damage_party) {
        runtime
            .cached_world_party(principal.actor, principal.session)
            .ok()
            .flatten()
            .filter(|party| party.character == *principal.character.as_bytes())
    } else {
        None
    };
    let party = principal_view
        .and_then(|principal_view| {
            let party_id = principal_view.party_id?;
            census
                .iter()
                .filter_map(|(actor, _, session)| {
                    let session = (*session)?;
                    let view = runtime.cached_world_party(*actor, session).ok()??;
                    let lease = runtime.bound_attacker_lease(*actor, session).ok()?;
                    (view.party_id == Some(party_id)
                        && lease.character_id().as_bytes() == &view.character)
                        .then_some(view)
                })
                .chain(std::iter::once(principal_view))
                .max_by_key(|view| (view.revision, view.observed_database_micros))
        })
        .filter(|party| {
            party.shared_xp_enabled
                && party
                    .members
                    .iter()
                    .any(|member| &member.character == principal.character.as_bytes())
        });
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
        let in_top_damage_party = principal_view
            .is_some_and(|view| view.character == snapshot.character)
            || party.is_some_and(|party| {
                party
                    .members
                    .iter()
                    .any(|member| member.character == snapshot.character)
                    && runtime
                        .cached_world_party(snapshot.actor, snapshot.session)
                        .ok()
                        .flatten()
                        .is_some_and(|view| {
                            view.character == snapshot.character && view.party_id == party.party_id
                        })
            });
        for key in &snapshot.objectives {
            let Some(rule) = source_deaths::rule(key) else {
                continue;
            };
            if !rule.matches_creature(&entry.creature)
                || !rule.recipient_with_party(position, damaged, in_top_damage_party)
                || (rule.top_damage_only
                    && (!principal_present
                        || principal.actor != snapshot.actor
                        || principal.session != snapshot.session
                        || principal.character.as_bytes() != &snapshot.character
                        || principal.lease_generation != snapshot.lease_generation))
            {
                continue;
            }
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
                    source: NativeDeathSource::Source(SourceDeathContext {
                        scope,
                        scope_generation: generation,
                        character: snapshot.character,
                        session: snapshot.session,
                        lease_generation: snapshot.lease_generation,
                        position,
                        dealt_damage: damaged,
                        top_damage_character: (rule.top_damage_only || rule.top_damage_party)
                            .then_some(*principal.character.as_bytes()),
                    }),
                },
            });
        }
    }
    recipients
}
