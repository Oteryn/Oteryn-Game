//! Source boss-death quest continuations on the existing captured lethal queue.
//! Crystal 201189f HeartBossDeath:26-138; CourtWarlock:323-329;
//! MagmaBubbleDeath:12-23 and GoblinLeaderDeath:3-8. Never infer social-party credit.
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
        for key in &snapshot.objectives {
            let Some(rule) = source_deaths::rule(key) else {
                continue;
            };
            if rule.creature != entry.creature
                || !rule.recipient(position, damaged)
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
                        top_damage_character: rule
                            .top_damage_only
                            .then_some(*principal.character.as_bytes()),
                    }),
                },
            });
        }
    }
    recipients
}
