//! Crystal 201189f8 data-global/scripts/spells/monster/the_ravager_heal.lua:6-16.
use super::*;
use crate::ai_think::profile_schedule::ScheduleList;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::map::boot::BundleCollisionIndexExt;

pub(super) fn execute(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    map: &CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
) {
    let rules: std::collections::BTreeMap<_, _> = native
        .creature_profiles()
        .records
        .iter()
        .filter_map(|row| {
            Some((
                row.profile.target.key.as_str(),
                row.nearby_self_heal.as_ref()?,
            ))
        })
        .collect();
    if rules.is_empty() || runtime.content_pin().server_artifact_digest() != native.source_digest()
    {
        return;
    }
    let Ok(creatures) = runtime.current_live_creature_census() else {
        return;
    };
    for proposal in proposals.iter().filter(|proposal| {
        proposal.list == ScheduleList::Defence && proposal.issuer == proposal.target
    }) {
        let actor = proposal.issuer;
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            continue;
        };
        let key = &snapshot.state.policy.definition_key;
        let Some(rule) = rules
            .get(key.as_str())
            .filter(|rule| rule.ability == proposal.ability)
        else {
            continue;
        };
        let Ok(origin) = runtime.read_actor_position(actor) else {
            continue;
        };
        let origin = origin.position();
        let radius = u32::from(rule.radius);
        let nearby = creatures.iter().any(|&other| {
            other != actor
                && runtime.current_live_creature_identity(other)
                    == Ok(rule.nearby_creature.as_bytes())
                && runtime.read_actor_position(other).is_ok_and(|other| {
                    let other = other.position();
                    other.floor == origin.floor
                        && other.x.abs_diff(origin.x) <= radius
                        && other.y.abs_diff(origin.y) <= radius
                })
        });
        if !nearby {
            continue;
        }
        let Ok(maximum) = u64::try_from(snapshot.maximum_health) else {
            continue;
        };
        if runtime
            .commit_source_creature_heal_batch(
                native.source_digest(),
                &[(actor, key.clone(), maximum, u64::from(rule.amount))],
            )
            .is_ok()
            && let NativeMovementCollisionIndex::Bundle(index) = map.room().movement_cells().index()
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(origin.x),
                u16::try_from(origin.y),
                i8::try_from(origin.floor),
            )
        {
            index.map().facts.emit_movement_effect(
                crate::map::overlay::TilePos { x, y, floor },
                rule.magic_effect,
            );
        }
    }
}
