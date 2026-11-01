//! Live adapter for D.1.3. The planner owns timing/formulas; the existing
//! compositor owns payment, magnitude, presentation and retained timer installation.
use super::*;
use crate::spell::delayed_execution::{SavedStrike, SpellTimerOccurrence};
use crate::spell::native_combat::{CombatPlan, MagnitudePlan};
use crate::spell::native_delayed::{
    CasterIdentity, CasterSnapshot, Element, Event, NativeDelayedFacts, NativeDelayedPlan,
    StrikePlan, TargetSnapshot,
};

pub(super) fn held_target(
    runtime: &ChannelRuntimeV1,
    held: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
) -> Option<(ExactActorRef, TilePosition)> {
    let held = held?;
    let actor = runtime
        .visible_entities()
        .creatures
        .iter()
        .find(|c| c.actor.placement_identity() == held.identity && c.generation == held.generation)
        .map(|c| c.actor)?;
    if runtime.live_creature_health(actor)? == 0 {
        return None;
    }
    Some((
        actor,
        tile(runtime.read_actor_position(actor).ok()?.position()),
    ))
}

#[allow(
    clippy::too_many_arguments,
    reason = "owner snapshots from the shared spatial phase"
)]
pub(super) fn facts(
    runtime: &ChannelRuntimeV1,
    owned: &OwnedCastFacts,
    before: &PlayerSpellState,
    caster: &crate::spell::CasterState,
    spell: &SpellDefinition,
    f: &NativeCombatFacts,
    tiles: &BTreeMap<TilePosition, TileFlags>,
    explicit_target: Option<u64>,
    saved: Option<&SavedStrike>,
) -> Result<NativeDelayedFacts, SpellCastDisposition> {
    // Planner identities are data, never authority. Full exact actor, session, UUIDs and
    // lease fences remain in CastBinding and the sealed SQL due evidence.
    let atom = |bytes: &[u8; 16]| {
        let mut tail = [0; 8];
        tail.copy_from_slice(&bytes[8..]);
        u64::from_be_bytes(tail).max(1)
    };
    let stance = if matches!(&spell.execution, Execution::NativeProfile(p)
        if p.spell()["execution"]["native_behavior"]["parameters"]["elemental_stance_snapshot"].is_array())
    {
        owned.elemental_stance(f.now_ms * 1000)
    } else {
        None
    };
    let current = CasterSnapshot {
        identity: CasterIdentity {
            actor: f.caster,
            character: atom(&owned.binding().character),
            session_generation: owned.binding().connection_generation,
            world: atom(runtime.binding().world_id().as_bytes()),
            channel: atom(runtime.binding().channel_id().as_bytes()),
        },
        position: f.caster_position,
        inputs: f.inputs,
        harmony: before.monk.as_ref().map_or(0, |m| m.harmony()),
        harmony_multiplier: caster.harmony_multiplier.numerator() as f64
            / crate::spell::harmony::MULTIPLIER_DENOMINATOR as f64,
        stage: f.stage,
        stance: stance
            .filter(|s| *s != ElementalStance::None)
            .map(|s| {
                owned
                    .magnitude()
                    .ok_or(SpellCastDisposition::Rejected)?
                    .delayed_stance_snapshot(
                        match s {
                            ElementalStance::Thunder => Element::Energy,
                            ElementalStance::Flames => Element::Fire,
                            _ => Element::Death,
                        },
                        owned
                            .access
                            .elemental_stance
                            .as_ref()
                            .ok_or(SpellCastDisposition::Rejected)?
                            .authority_revision,
                        spell
                            .authored
                            .as_ref()
                            .and_then(|p| p.header.reference_spell_id)
                            .ok_or(SpellCastDisposition::Rejected)?,
                    )
                    .map_err(|_| SpellCastDisposition::Rejected)
            })
            .transpose()?,
    };
    let cast = saved
        .map(|s| {
            s.binding
                .cast_snapshot
                .clone()
                .ok_or(SpellCastDisposition::Rejected)
        })
        .transpose()?
        .unwrap_or_else(|| current.clone());
    let origin = f.area_origin;
    let at = tiles.get(&origin).ok_or(SpellCastDisposition::Rejected)?;
    let caster_tile = tiles
        .get(&f.caster_position)
        .ok_or(SpellCastDisposition::Rejected)?;
    Ok(NativeDelayedFacts {
        event: if saved.is_some() {
            Event::DelayedOccurrence
        } else {
            Event::Cast
        },
        now_ms: f.now_ms,
        cast_ms: saved.map_or(f.now_ms, |s| s.binding.cast_at.get() / 1000),
        cast,
        current: Some(current),
        cast_position: origin,
        cast_position_exists: at.present,
        cast_position_legal: at.present,
        caster_in_protection_zone: caster_tile.protection,
        cast_position_in_protection_zone: at.protection,
        immediate_succeeded: true,
        explicit_target: saved.map_or(explicit_target, |s| s.scheduled.explicit_target),
        targets: f
            .targets
            .iter()
            .map(|t| TargetSnapshot {
                actor: t.id,
                position: t.position,
                legal: t.legal,
                npc: t.kind == TargetKind::Other,
                protection_zone: tiles.get(&t.position).is_none_or(|t| t.protection),
            })
            .collect(),
        clear_sight: f.clear_sight.clone(),
        field_tiles: Vec::new(),
        field_items: Vec::new(),
        created_field_instances: BTreeSet::new(),
        captured_raw_bounds: saved.and_then(|s| s.scheduled.captured_raw_bounds),
    })
}

pub(in crate::gameplay_transport) fn combat(
    strike: Option<&StrikePlan>,
    cooldown_ms: Option<u64>,
) -> NativeCombatPlan {
    let element = match strike.map(|s| s.element).unwrap_or(Element::Holy) {
        Element::Physical => crate::spell::native_combat::Element::Physical,
        Element::Death => crate::spell::native_combat::Element::Death,
        Element::Fire => crate::spell::native_combat::Element::Fire,
        Element::Energy => crate::spell::native_combat::Element::Energy,
        Element::Holy => crate::spell::native_combat::Element::Holy,
    };
    NativeCombatPlan::Combat(CombatPlan {
        element,
        area: None,
        hits: strike
            .into_iter()
            .flat_map(|s| &s.hits)
            .map(|h| MagnitudePlan {
                target: h.actor,
                magnitude: h.magnitude,
                bonus_percent: 0,
                side_percent: None,
                delay_ms: 0,
            })
            .collect(),
        cooldown_ms,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor: false,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: Vec::new(),
        resolve_critical_and_fatal_once: true,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "same paid owner admission for both native plan families"
)]
pub(super) fn paid(
    before: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &OperationalCastFacts,
    facts: &NativeCombatFacts,
    delayed: Option<&NativeDelayedFacts>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    caster: &crate::spell::CasterState,
) -> Result<
    (
        crate::spell::cast::PaidNativeCast,
        Option<NativeDelayedPlan>,
    ),
    SpellCastDisposition,
> {
    let mut paid = prepare_native_owner_cast_with_caster(
        before,
        spell,
        operational,
        delayed.map_or(Facts::Combat(facts), Facts::Delayed),
        now,
        draw,
        caster,
    )?;
    if !matches!(paid.plan, Plan::Delayed(_)) {
        return Ok((paid, None));
    }
    let Plan::Delayed(plan) = paid.plan else {
        return reject();
    };
    let NativeDelayedPlan::Cast {
        immediate,
        cooldown_ms,
        ..
    } = plan.as_ref()
    else {
        return reject();
    };
    paid.plan = Plan::Combat(Box::new(combat(immediate.as_ref(), *cooldown_ms)));
    Ok((paid, Some(*plan)))
}

#[allow(
    clippy::too_many_arguments,
    reason = "same timer reservation as the parent combat phase"
)]
pub(super) fn requests(
    runtime: &ChannelRuntimeV1,
    lowered: &crate::spell::combat_batch::LoweredNativeCombat,
    plan: Option<&NativeDelayedPlan>,
    facts: Option<&NativeDelayedFacts>,
    profile: &crate::spell::native::CompiledNativeSpell,
    occurrence: &AbilityOccurrence,
    position: TilePosition,
    now: SemanticTimeMicros,
    owned: &OwnedCastFacts,
    before: &PlayerSpellState,
) -> Result<Vec<ScheduleRequest>, SpellCastDisposition> {
    let Some(NativeDelayedPlan::Cast { scheduled, .. }) = plan else {
        return timer_requests(lowered, profile, occurrence, position, now);
    };
    let Some(scheduled) = scheduled else {
        return Ok(Vec::new());
    };
    let batch = &lowered.batch;
    let attributes = owned.magnitude().ok_or(SpellCastDisposition::Rejected)?;
    let canonical_hex = runtime
        .soulpit_hex_delta(attributes.binding.actor, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let cast_modifiers = crate::spell::magnitude_owner::StrikeCastModifiers::capture(
        attributes,
        before,
        profile.spell()["name"]
            .as_str()
            .ok_or(SpellCastDisposition::Rejected)?,
        now.get() / 1000,
        canonical_hex,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    // Preserve the source raw formula snapshot. Crystal computes current Wheel
    // flat/gem/Focus bonuses in the new due Combat, never inside planted bounds.
    let scheduled = scheduled.clone();
    Ok(vec![ScheduleRequest {
        occurrence: SpellTimerOccurrence {
            command: batch.command,
            phase: 0,
        },
        due: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
            scheduled
                .due_ms
                .checked_mul(1000)
                .ok_or(SpellCastDisposition::Rejected)?,
        ),
        payload: TimerPayload::Strike(SavedStrike {
            binding: CastBinding {
                spell: profile.clone(),
                caster: batch.caster,
                attacker: batch.attacker,
                command: batch.command,
                occurrence: occurrence.clone(),
                parent_binding: batch.binding.clone(),
                cast_at: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                cast_position: scheduled.position,
                cast_snapshot: Some(facts.ok_or(SpellCastDisposition::Rejected)?.cast.clone()),
            },
            scheduled,
            resolved: None,
            cast_modifiers,
            first_sub_ordinal: lowered.next_sub_ordinal,
        }),
    }])
}

pub(super) fn cues(
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    combat: &NativeCombatPlan,
    plan: Option<&NativeDelayedPlan>,
    profile: &crate::spell::native::CompiledNativeSpell,
    bindings: &[LiveActorBinding],
    caster: ExactActorRef,
) -> Result<Vec<super::super::super::spell_presentations::LocatedCueRequest>, SpellCastDisposition>
{
    let Some(NativeDelayedPlan::Cast {
        immediate, marker, ..
    }) = plan
    else {
        return presentation_requests(room, runtime, combat, profile, bindings, caster);
    };
    let mut cues = strike_cues(room, runtime, immediate.as_ref(), bindings)?;
    if let Some(marker) = marker {
        use crate::gameplay_transport::spell_presentations::{
            CueTarget, LocatedCueRequest, QualifiedCueTile,
        };
        cues.push(LocatedCueRequest {
            binding: marker.asset_binding.clone(),
            target: CueTarget::Tile(
                QualifiedCueTile::from_owners(room, runtime, cell(marker.position))
                    .map_err(|_| SpellCastDisposition::Rejected)?,
            ),
        });
    }
    Ok(cues)
}

pub(in crate::gameplay_transport) fn strike_cues(
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    strike: Option<&StrikePlan>,
    bindings: &[LiveActorBinding],
) -> Result<
    Vec<crate::gameplay_transport::spell_presentations::LocatedCueRequest>,
    SpellCastDisposition,
> {
    use crate::gameplay_transport::spell_presentations::{
        CueTarget, LocatedCueRequest, QualifiedCueTile,
    };
    let Some(strike) = strike else {
        return Ok(Vec::new());
    };
    let mut cues = Vec::new();
    for p in &strike.area_tiles {
        cues.push(LocatedCueRequest {
            binding: strike.effect_asset_binding.clone(),
            target: CueTarget::Tile(
                QualifiedCueTile::from_owners(room, runtime, cell(*p))
                    .map_err(|_| SpellCastDisposition::Rejected)?,
            ),
        });
    }
    for id in &strike.chain_targets {
        cues.push(LocatedCueRequest {
            binding: strike.effect_asset_binding.clone(),
            target: CueTarget::Actor(
                bindings
                    .iter()
                    .find(|b| b.source_id == *id)
                    .ok_or(SpellCastDisposition::Rejected)?
                    .actor,
            ),
        });
    }
    Ok(cues)
}
