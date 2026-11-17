//! Canonical HP, condition and movement consumers for admitted Drome effects.
use super::*;
pub(super) fn apply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: Option<&mut crate::gameplay_transport::actor_spell::ChannelSpellStates>,
    recipients: Recipients<'_>,
    ordinal: u64,
    now: crate::foundation::owner_timer::SemanticTimeMicros,
    effect: Effect,
) -> bool {
    let Recipients {
        opponents,
        players,
        death_source,
    } = recipients;
    match effect {
        Effect::CurrentHealthDamage { target, percent } => {
            let Some((_, session)) = players.iter().find(|(actor, _)| *actor == target) else {
                if runtime.live_creature_health(target).is_none() || !runtime.is_tibiadrome(target)
                {
                    return true;
                }
                if percent != 40 {
                    return false;
                }
                let Some(dying) = death_source else {
                    return false;
                };
                return match runtime.commit_tibiadrome_familiar_death_blast(target, dying, ordinal)
                {
                    Ok(_) | Err(crate::foundation::CarrierError::StaleAttackerSequence) => true,
                    Err(_) => false,
                };
            };
            let Some(states) = states else {
                return false;
            };
            use crate::gameplay_transport::actor_spell::DromeHealthLoss;
            let loss = match percent {
                40 => DromeHealthLoss::GoingDown,
                25 => DromeHealthLoss::ExplodingCorpse,
                60 => DromeHealthLoss::Lava,
                _ => return false,
            };
            states
                .apply_drome_health_loss(runtime, target, *session, loss, now)
                .is_some()
        }
        Effect::Teleport { actor, destination } => {
            if !walkable(owner, runtime, destination)
                || !runtime.native_summon_cell_free(destination)
            {
                return false;
            }
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return true;
            };
            let committed =
                if let Some((_, session)) = players.iter().find(|(target, _)| *target == actor) {
                    runtime
                        .relocate_arena_player(actor, *session, expected, destination)
                        .is_ok()
                } else {
                    runtime
                        .relocate_tibiadrome_creature(actor, expected, destination)
                        .is_ok()
                };
            if committed {
                // The existing effect queue is observation only. A committed
                // translation must never be replayed because cue publication failed.
                teleport_cues(owner, runtime, expected.position(), destination);
            }
            committed
        }
        Effect::Status {
            target,
            kind,
            duration_ms,
            incoming_percent,
        } => {
            let Some(states) = states else {
                return false;
            };
            let Some((_, session)) = players.iter().find(|(actor, _)| *actor == target) else {
                return true;
            };
            let Some(source) = opponents
                .iter()
                .copied()
                .find(|source| runtime.companion_snapshot(*source).is_ok())
            else {
                return false;
            };
            let Some(definition) = crate::foundation::ConditionDefinition::new(
                "oteryn:condition.drome.modifier",
                1,
                crate::foundation::ConditionValues::TimedStatus { kind, duration_ms },
            ) else {
                return false;
            };
            let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                runtime.content_pin().server_artifact_digest(),
            );
            let mut occurrence = source.placement_identity();
            occurrence[..8].copy_from_slice(&ordinal.to_be_bytes());
            let facts = crate::foundation::ApplicationFacts {
                now: now.get(),
                base_speed: 0,
                mana_shield_capacity: 0,
                target_reentry_protected: runtime
                    .current_player_reentry_protection(target, *session, now.get())
                    .unwrap_or(true),
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    occurrence,
                ),
            };
            let Ok(stamp) = runtime.issue_owner_work() else {
                return false;
            };
            if states
                .native_composite_conditions(
                    runtime,
                    target,
                    *session,
                    &format!("drome-modifier:{ordinal}"),
                    source,
                    &[definition],
                    &facts,
                    &[],
                    None,
                    crate::player_lethal::PlayerHitKind::Mitigated,
                    None,
                    stamp,
                )
                .is_none()
            {
                return false;
            }
            let modifier = match incoming_percent {
                110 => DromeModifier::TankedUp,
                115 => DromeModifier::SownSorrow,
                125 => DromeModifier::BadRoots,
                _ => return false,
            };
            runtime
                .apply_tibiadrome_incoming_modifier(
                    target,
                    modifier,
                    now.get().saturating_add(u64::from(duration_ms) * 1000),
                )
                .is_ok()
        }
        // Native mark/seed projection needs its source-qualified visual carrier.
        // Retain it rather than pretending an unqualified effect ID is a native mark.
        Effect::Mark { .. } | Effect::Seed { .. } => false,
    }
}

/// Current scoped map, owner-qualified wire coordinate frame, same outbox.
fn teleport_cues(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) {
    use crate::map::boot::BundleCollisionIndexExt;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        owner.movement_cells().index()
    else {
        return;
    };
    if index.map().overlay.runtime_scope() != runtime.binding().runtime_scope() {
        return;
    }
    for at in [from, to] {
        let Some(observation) = owner.wire_observation(runtime, at) else {
            continue;
        };
        let at = observation.actor_position;
        let (Ok(x), Ok(y), Ok(floor)) = (at.x.try_into(), at.y.try_into(), at.floor.try_into())
        else {
            continue;
        };
        let _published = index
            .map()
            .facts
            .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 11);
    }
}
