use super::PendingKillSettlement;
use crate::durability::boss_encounter::reward_chest::{BossOutfit, BossOutfitRecipient};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};

fn pale_worm_spectator(at: MovementLocalPosition) -> bool {
    // Crystal 201189f8 feaster_of_souls/creaturescripts_pale_worm_death.lua:1–12.
    at.floor == 14 && at.x.abs_diff(33805) <= 11 && at.y.abs_diff(31504) <= 11
}

pub(super) fn capture(runtime: &ChannelRuntimeV1, entry: &mut PendingKillSettlement) {
    let Some(outfit) = BossOutfit::for_death(&entry.creature) else {
        return;
    };
    if !runtime
        .companion_snapshot_including_dead(entry.target)
        .is_ok_and(|view| view.state.master.is_none() && !view.state.policy.is_familiar)
    {
        return;
    }
    let Ok(census) = runtime.positioned_actor_census() else {
        return;
    };
    // Crystal reads Player(actor) from the damage map for Ratmiral, Arbaziloth
    // and Lord Retro. Physical blows exclude summons and shared-XP bystanders.
    let damage = if outfit == BossOutfit::Poltergeist {
        Vec::new()
    } else {
        let Ok(damage) = runtime.all_source_player_damage_shares(entry.target) else {
            return;
        };
        damage
    };
    let recipients: Vec<_> = census
        .into_iter()
        .filter_map(|(actor, at, session)| {
            let session = session?;
            let lease = runtime.bound_attacker_lease(actor, session).ok()?;
            let eligible = if outfit == BossOutfit::Poltergeist {
                pale_worm_spectator(at.position())
            } else {
                damage.iter().any(|share| {
                    share.actor == actor
                        && share.character == lease.character_id()
                        && share.total > 0
                })
            };
            if !eligible {
                return None;
            }
            Some(BossOutfitRecipient {
                character: crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes())
                    .ok()?,
                session,
                lease_generation: lease.generation(),
            })
        })
        .collect();
    if !recipients.is_empty() {
        entry.boss_outfit = Some((outfit, recipients));
    }
}
