//! Damage-map credit uses the same lethal capture, retry queue and fenced
//! settlement as party counters. Crystal 201189f8 functions.lua:1064-1082.

use super::{PendingKillSettlement, TaskSnapshot};
use crate::durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
use crate::foundation::{
    ChannelRuntimeV1, CharacterId, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use oteryn_gameplay_quests::quest::{
    authored_bindings::kills::DamageRecipients, native_kills::admits_death_position,
};

pub(super) fn position(at: MovementLocalPosition) -> Option<(u16, u16, i8)> {
    Some((
        u16::try_from(at.x).ok()?,
        u16::try_from(at.y).ok()?,
        i8::try_from(at.floor).ok()?,
    ))
}

struct DamageCandidate {
    source_actor: Option<ExactActorRef>,
    character: CharacterId,
    session: GameSessionId,
    lease_generation: u64,
    total: u64,
    last_hit: Option<u64>,
}

impl DamageCandidate {
    fn matches(&self, snapshot: &TaskSnapshot) -> bool {
        self.character.as_bytes() == &snapshot.character
            && self
                .source_actor
                .is_none_or(|actor| actor == snapshot.actor)
            && self.session == snapshot.session
            && self.lease_generation == snapshot.lease_generation
    }
}

pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    snapshots: &[TaskSnapshot],
    outcome: Option<&super::encounter_outcomes::Run>,
) -> Vec<super::super::quest_kills::Recipient> {
    let mut recipients = Vec::new();
    if !snapshots.iter().any(|snapshot| {
        snapshot
            .native_counters
            .iter()
            .any(|counter| counter.creature == entry.creature && counter.damage_credit.is_some())
    }) || !runtime
        .companion_snapshot_including_dead(entry.target)
        .is_ok_and(|victim| victim.state.master.is_none())
    {
        return recipients;
    }
    let Some(position) = position(entry.facts.corpse) else {
        return recipients;
    };
    let corpse = entry.facts.corpse;
    let point = Some([corpse.x, corpse.y, i32::from(corpse.floor)]);
    let Ok(direct) = runtime.source_player_damage_shares(entry.target) else {
        return recipients;
    };
    let owner = runtime.borrow_combat_death();
    let top = owner
        .top_damage_contributor(entry.target)
        .ok()
        .and_then(|top| top.present());
    let Ok(attributed) = owner.current_session_damage_shares(entry.target) else {
        return recipients;
    };
    let Ok(census) = runtime.positioned_actor_census() else {
        return recipients;
    };
    let live = |candidate: &DamageCandidate| {
        candidate.total > 0
            && census.iter().any(|(actor, _, session)| {
                *session == Some(candidate.session)
                    && candidate.source_actor.is_none_or(|source| source == *actor)
                    && runtime
                        .bound_attacker_lease(*actor, candidate.session)
                        .is_ok_and(|lease| {
                            lease.character_id() == candidate.character
                                && lease.generation() == candidate.lease_generation
                        })
            })
    };
    let build_rank = |mut candidates: Vec<DamageCandidate>| {
        candidates.retain(live);
        candidates.sort_unstable_by_key(|candidate| {
            (
                std::cmp::Reverse(candidate.total),
                !top.is_some_and(|winner| winner.character == candidate.character),
                candidate.character,
            )
        });
        candidates
    };
    let direct = build_rank(
        direct
            .into_iter()
            .map(|hit| DamageCandidate {
                source_actor: Some(hit.actor),
                character: hit.character,
                session: hit.session,
                lease_generation: hit.lease_generation,
                total: hit.total,
                last_hit: hit.last_damage_at_ms,
            })
            .collect(),
    );
    let attributed = build_rank(
        attributed
            .into_iter()
            .filter_map(|hit| {
                Some(DamageCandidate {
                    source_actor: None,
                    character: hit.character,
                    session: hit.session?,
                    lease_generation: hit.lease_generation?,
                    total: hit.total,
                    last_hit: hit.last_damage_at_ms,
                })
            })
            .collect(),
    );
    for snapshot in snapshots {
        for counter in &snapshot.native_counters {
            let Some(credit) = counter.damage_credit else {
                continue;
            };
            if counter.creature != entry.creature
                || !admits_death_position(counter.death_area, &counter.death_areas, point)
            {
                continue;
            }
            let outcome_source = if let Some(required) = &counter.required_outcome {
                let Some(source) = outcome.and_then(|run| {
                    run.credit_source(required, (snapshot.actor, snapshot.session))
                }) else {
                    continue;
                };
                Some(source.to_owned())
            } else {
                None
            };
            let candidates = if credit.summon_owner {
                &attributed
            } else {
                &direct
            };
            // Crystal201189f creature.cpp:531-552 ranks damage after the fight
            // window, and :587 resolves the winning summon to its master.
            let mut eligible = candidates
                .iter()
                .filter(|hit| credit.admits_hit(hit.last_hit, entry.facts.death_at_ms));
            let hit = match credit.recipients {
                DamageRecipients::TopDamage => eligible.next().filter(|hit| hit.matches(snapshot)),
                DamageRecipients::AllDamagers => eligible.find(|hit| hit.matches(snapshot)),
            };
            let Some(hit) = hit else {
                continue;
            };
            recipients.push(super::super::quest_kills::Recipient {
                actor: snapshot.actor,
                session: snapshot.session,
                character: snapshot.character,
                lease_generation: snapshot.lease_generation,
                death: NativeQuestDeath {
                    transition: counter.transition.clone(),
                    creature: counter.creature.clone(),
                    definition_hash: counter.definition_hash,
                    observed_revision: snapshot.observed_revision,
                    source: NativeDeathSource::Damage {
                        death: entry.facts.death,
                        session: snapshot.session,
                        character: hit.character,
                        lease_generation: snapshot.lease_generation,
                        position,
                        hit_at_ms: hit.last_hit,
                        death_at_ms: entry.facts.death_at_ms,
                        died_at_unix_ms: entry.facts.died_at_unix_ms,
                        outcome_source,
                    },
                },
            });
        }
    }
    recipients
}
