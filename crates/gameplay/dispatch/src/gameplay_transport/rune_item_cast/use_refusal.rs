//! Diagnose a native refusal using the same current owners and authored legality checks.
use super::*;
use oteryn_gameplay_spell_rules::dispatch_plans::cast_legality as legality;

/// Preserve the USE distinctions that the spell wire intentionally folds into
/// TargetIllegal/Rejected. Read the same current owners in the same transaction;
/// this diagnoses a refusal and never authorizes or prepares another cast.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport::actor_spell::native_combat_cast) async fn use_refusal(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &super::super::ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &crate::world_runtime::LocalObjectRuntime,
    spell: &SpellDefinition,
    actor: crate::foundation::ExactActorRef,
    intent: &ItemSpellCastIntent,
    legacy: &super::super::LegacyRuneUse,
    reason: oteryn_protocol_oteryn::actor_spell::SpellCastDisposition,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> oteryn_protocol_oteryn::world_object::UseDisposition {
    use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition as S, SpellTarget};
    use oteryn_protocol_oteryn::world_object::UseDisposition as U;
    let session = authority.game_session_id();
    let Some(state) = states.get(runtime, actor, session) else {
        return U::Rejected;
    };
    if crate::spell::cast::admit_condition_gates(state, spell, now).is_err() {
        return U::RequirementNotMet;
    }
    if !matches!(reason, S::Rejected | S::TargetIllegal | S::TargetRequired) {
        return super::super::super::super::rune_use_bridge::spell_disposition(reason);
    }
    if spell.aggressive
        && runtime
            .current_player_reentry_protection(actor, session, now.get())
            .unwrap_or(true)
    {
        return U::TargetIllegal;
    }
    let Ok(position) = runtime.read_actor_position(actor) else {
        return U::Rejected;
    };
    let from = super::super::tile(position.position());
    let Ok(source) = super::super::read_tile(tx, authority, room, runtime, objects, from).await
    else {
        return U::Rejected;
    };
    let Some(profile) = spell.authored.as_ref() else {
        return U::Rejected;
    };
    if legality::protection_rejected(profile, source.protection) {
        return U::ProtectionZone;
    }
    let target = match intent.intent.target {
        SpellTarget::Position(p) => Some(crate::spell::chain::TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        }),
        SpellTarget::AttackTarget => legacy.creature.and_then(|wanted| {
            runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| {
                    c.actor.placement_identity() == wanted.identity
                        && c.generation == wanted.generation
                })
                .and_then(|c| runtime.read_actor_position(c.actor).ok())
                .map(|p| super::super::tile(p.position()))
        }),
        SpellTarget::None => Some(from),
    };
    if let Some(target) = target {
        match legality::reach_refusal(
            profile,
            (from.x, from.y, from.floor),
            (target.x, target.y, target.floor),
        ) {
            Some(legality::PositionRefusal::Range) => return U::TooFar,
            Some(legality::PositionRefusal::Spatial) => return U::TargetIllegal,
            None => (),
        }
        let Ok(tile) = super::super::read_tile(tx, authority, room, runtime, objects, target).await
        else {
            return U::Rejected;
        };
        if legality::protection_rejected(profile, tile.protection)
            || !tile.present
            || tile.floor_change
        {
            return U::TargetIllegal;
        }
        let mut sight_clear = true;
        if reason == S::Rejected {
            let Ok(steps) = super::super::sight_steps(from, target) else {
                return U::Rejected;
            };
            for (position, exempt) in steps {
                if exempt {
                    continue;
                }
                let Ok(tile) =
                    super::super::read_tile(tx, authority, room, runtime, objects, position).await
                else {
                    return U::Rejected;
                };
                if tile.projectile {
                    sight_clear = false;
                    break;
                }
            }
        }
        let occupied = runtime.positioned_actor_census().is_ok_and(|actors| {
            actors
                .iter()
                .any(|(_, p, _)| super::super::tile(p.position()) == target)
        });
        match legality::position_refusal(
            profile,
            (from.x, from.y, from.floor),
            (target.x, target.y, target.floor),
            Some(sight_clear),
            Some(tile.solid),
            Some(occupied),
        ) {
            Some(legality::PositionRefusal::Range) => return U::TooFar,
            Some(legality::PositionRefusal::Spatial) => return U::TargetIllegal,
            None => (),
        }
    }
    super::super::super::super::rune_use_bridge::spell_disposition(reason)
}
