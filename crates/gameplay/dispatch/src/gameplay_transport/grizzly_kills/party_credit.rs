//! Damage-map credit uses the same lethal capture, retry queue and fenced
//! settlement as party counters. Crystal 201189f8 functions.lua:1064-1082.

use super::{PendingKillSettlement, TaskSnapshot};
use crate::durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
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

pub(super) fn capture(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    snapshots: &[TaskSnapshot],
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
    for snapshot in snapshots {
        let direct_hit = direct.iter().find(|hit| {
            hit.total > 0
                && hit.character.as_bytes() == &snapshot.character
                && hit.actor == snapshot.actor
                && hit.session == snapshot.session
                && hit.lease_generation == snapshot.lease_generation
        });
        let owner_hit = attributed.iter().find(|hit| {
            hit.total > 0
                && hit.character.as_bytes() == &snapshot.character
                && hit.session == Some(snapshot.session)
                && hit.lease_generation == Some(snapshot.lease_generation)
        });
        let is_top = top.is_some_and(|winner| {
            winner.actor == snapshot.actor
                && winner.session == snapshot.session
                && winner.lease_generation == snapshot.lease_generation
        });
        for counter in &snapshot.native_counters {
            let Some(credit) = counter.damage_credit else {
                continue;
            };
            if counter.creature != entry.creature
                || credit.recipients == DamageRecipients::TopDamage && !is_top
                || !admits_death_position(counter.death_area, &counter.death_areas, point)
            {
                continue;
            }
            let hit = if credit.summon_owner {
                owner_hit.map(|share| (share.character, share.last_damage_at_ms))
            } else {
                direct_hit.map(|share| (share.character, share.last_damage_at_ms))
            };
            let Some((character, hit_at_ms)) = hit else {
                continue;
            };
            if !credit.admits_hit(hit_at_ms, entry.facts.death_at_ms) {
                continue;
            }
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
                        character,
                        lease_generation: snapshot.lease_generation,
                        position,
                        hit_at_ms,
                        death_at_ms: entry.facts.death_at_ms,
                    },
                },
            });
        }
    }
    recipients
}
