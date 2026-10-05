//! Ordinary authored abilities use the same retained, fenced source transaction
//! as native combat. Actor identities, map flags and target predecessors come
//! from the actual owners; the client supplies only a spatial proposal.
use super::*;
use crate::gameplay_transport::spell_presentations::{
    CueTarget, LocatedCueRequest, QualifiedCueTile,
};
use crate::spell::ResolvedEffect;
use crate::spell::cast::{PaidOrdinaryCast, prepare_ordinary_owner_cast_with_caster};
use crate::spell::chain::{ChainCreature, ChainStart, ChainWorld};
use crate::spell::combat_batch::{OwnerCombatChange, OwnerCombatEffect, TimedCombatEffect};
use crate::spell::executable_catalog::{AbilityProfile, DefinitionRef};
use crate::spell::native_combat::{CombatPlan, Element, MagnitudePlan};
use sha2::Digest;

#[derive(Debug, Clone)]
pub(super) struct OrdinaryItemCreation {
    pub(super) position: TilePosition,
    pub(super) item: DefinitionRef,
}

pub(super) fn applicable(spell: &SpellDefinition) -> bool {
    matches!(
        spell.execution,
        Execution::Effects(_) | Execution::AbilityVariants(_) | Execution::PartyBuff(_)
    )
}
pub(super) fn has_world_geometry(spell: &SpellDefinition) -> bool {
    spell.chain.is_some()
        || spell.authored.as_ref().is_some_and(|p| {
            p.dependencies
                .abilities
                .iter()
                .any(|a| a.area.is_some() || a.chain.is_some())
        })
}
fn ability<'a>(
    spell: &'a SpellDefinition,
    variant: Option<&DefinitionRef>,
) -> Result<&'a AbilityProfile, SpellCastDisposition> {
    let p = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let key = variant
        .or(p.header.execution.ability.as_ref())
        .ok_or(SpellCastDisposition::Rejected)?;
    p.dependencies
        .abilities
        .iter()
        .find(|a| a.identity.key == key.key && a.identity.revision == key.revision)
        .ok_or(SpellCastDisposition::Rejected)
}
fn geometry(
    a: &AbilityProfile,
    origin: TilePosition,
    facing: Direction,
) -> Result<BTreeSet<TilePosition>, SpellCastDisposition> {
    let Some(area) = &a.area else {
        return Ok(BTreeSet::from([origin]));
    };
    let matrix = |rows: &[String]| -> Result<Value, SpellCastDisposition> {
        rows.iter()
            .map(|r| {
                r.bytes()
                    .map(|c| match c {
                        b'.' => Ok(json!(0)),
                        b'x' => Ok(json!(1)),
                        b'c' => Ok(json!(2)),
                        b'C' => Ok(json!(3)),
                        _ => reject(),
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    };
    let mut area_json =
        json!({"directional":a.needs_direction,"orthogonal":matrix(&area.matrix.north)?});
    if let Some(rows) = &area.matrix.diagonal {
        area_json["diagonal"] = matrix(rows)?;
    }
    area_candidates(&json!({"area":area_json}), origin, facing)
}
fn facing_to(from: TilePosition, to: TilePosition) -> Direction {
    match (
        (i64::from(to.x) - i64::from(from.x)).signum(),
        (i64::from(to.y) - i64::from(from.y)).signum(),
    ) {
        (0, -1) => Direction::North,
        (1, -1) => Direction::NorthEast,
        (1, 0) => Direction::East,
        (1, 1) => Direction::SouthEast,
        (0, 1) => Direction::South,
        (-1, 1) => Direction::SouthWest,
        (-1, 0) => Direction::West,
        _ => Direction::NorthWest,
    }
}
/// SPELL-TARGET-1: the geometry origin and facing of a held-target cast. A directional or aimed
/// spell keeps the caster as origin (the tile in front of it) and takes only its facing from the
/// target; a single-target spell is centred on the target tile.
fn attack_target_origin(
    caster: TilePosition,
    target: TilePosition,
    directional: bool,
) -> Result<(TilePosition, Option<Direction>), SpellCastDisposition> {
    if directional && target != caster {
        let facing = facing_to(caster, target);
        let (dx, dy) = delta(facing);
        Ok((offset(caster, dx, dy)?, Some(facing)))
    } else {
        Ok((target, None))
    }
}
#[derive(Debug)]
struct World {
    origin: TilePosition,
    /// The position every populated sight path starts from: the caster for a directional
    /// cast, otherwise the aimed origin. Area visibility must look up the same key.
    sight_origin: TilePosition,
    direction: Direction,
    caster: ChainCreature,
    creatures: Vec<ChainCreature>,
    legal: BTreeSet<u64>,
    sight: BTreeSet<(TilePosition, TilePosition)>,
    tiles: BTreeMap<TilePosition, TileFlags>,
}
impl World {
    /// Area targets, created items and tile cues are visible from the sight origin.
    fn area_sight_clear(&self, p: TilePosition) -> bool {
        self.sight_clear(self.sight_origin, p)
    }
}
impl ChainWorld for World {
    fn caster(&self) -> &ChainCreature {
        &self.caster
    }
    fn creatures(&self) -> &[ChainCreature] {
        &self.creatures
    }
    fn may_hit(&self, c: &ChainCreature) -> bool {
        self.legal.contains(&c.id)
    }
    fn sight_clear(&self, a: TilePosition, b: TilePosition) -> bool {
        a == b || self.sight.contains(&(a, b))
    }
    fn path(&self, from: TilePosition, to: TilePosition) -> Option<Vec<TilePosition>> {
        use std::collections::VecDeque;
        if from.floor != to.floor {
            return None;
        }
        let mut queue = VecDeque::from([from]);
        let mut seen = BTreeMap::<TilePosition, Option<TilePosition>>::from([(from, None)]);
        while let Some(p) = queue.pop_front() {
            if p.x.abs_diff(to.x) <= 1 && p.y.abs_diff(to.y) <= 1 {
                let mut path = Vec::new();
                let mut cur = p;
                while let Some(Some(previous)) = seen.get(&cur) {
                    path.push(cur);
                    cur = *previous;
                }
                path.reverse();
                return Some(path);
            }
            for (dx, dy) in [
                (0, -1),
                (1, 0),
                (0, 1),
                (-1, 0),
                (-1, -1),
                (1, -1),
                (1, 1),
                (-1, 1),
            ] {
                let Ok(next) = offset(p, dx, dy) else {
                    continue;
                };
                if next.x.abs_diff(from.x) > crate::spell::chain::CHAIN_PATH_SEARCH_TILES
                    || next.y.abs_diff(from.y) > crate::spell::chain::CHAIN_PATH_SEARCH_TILES
                    || seen.contains_key(&next)
                    || !self
                        .tiles
                        .get(&next)
                        .is_some_and(|t| t.present && !t.solid && !t.floor_change)
                {
                    continue;
                }
                if dx != 0
                    && dy != 0
                    && [offset(p, dx, 0).ok(), offset(p, 0, dy).ok()]
                        .into_iter()
                        .any(|side| {
                            side.and_then(|s| self.tiles.get(&s))
                                .is_none_or(|t| !t.present || t.solid)
                        })
                {
                    continue;
                }
                seen.insert(next, Some(p));
                queue.push_back(next);
            }
        }
        None
    }
}
fn element(key: &str) -> Result<Element, SpellCastDisposition> {
    Ok(match key {
        "physical" => Element::Physical,
        "energy" => Element::Energy,
        "fire" => Element::Fire,
        "death" => Element::Death,
        "ice" => Element::Ice,
        "earth" => Element::Earth,
        "healing" => Element::Healing,
        _ => return reject(),
    })
}
fn heals_health(effect: &ResolvedEffect) -> bool {
    match effect {
        ResolvedEffect::Heal { .. } => true,
        ResolvedEffect::ResolvedOther { profile, .. } => profile
            .condition
            .as_ref()
            .and_then(|c| c.regeneration.as_ref())
            .is_some_and(|r| r.health_gain.is_some_and(|gain| gain > 0)),
        _ => false,
    }
}
fn condition_kind(
    key: &str,
) -> Result<crate::ability::condition::ConditionType, SpellCastDisposition> {
    use crate::ability::condition::{ConditionType as T, DotElement as E};
    Ok(match key {
        "paralyze" => T::Paralysis,
        "manashield" => T::ManaShield,
        "poison" => T::DamageOverTime(E::Poison),
        "fire" => T::DamageOverTime(E::Fire),
        "energy" => T::DamageOverTime(E::Energy),
        "bleeding" => T::DamageOverTime(E::Bleeding),
        "cursed" => T::DamageOverTime(E::Cursed),
        "dazzled" => T::DamageOverTime(E::Dazzled),
        "invisible" => T::Invisible,
        _ => return reject(),
    })
}
fn modify_conditions<S: Clone>(
    store: &mut crate::ability::condition::ConditionStore<S>,
    effects: &[ResolvedEffect],
    source: Option<S>,
    immunities: &[crate::ability::condition::ConditionType],
    facts: &crate::ability::condition::ApplicationFacts<'_>,
) -> Result<(), SpellCastDisposition> {
    use crate::ability::condition::{ConditionRefusal, ConditionSourceKind};
    for effect in effects {
        match effect {
            ResolvedEffect::Damage { .. } | ResolvedEffect::Heal { .. } => {}
            ResolvedEffect::RemoveCondition { condition } => {
                store.remove_type(condition_kind(condition)?);
            }
            ResolvedEffect::ResolvedOther { profile, .. }
                if crate::spell::actor_conditions::is_presentation_only(profile) => {}
            ResolvedEffect::ResolvedOther {
                profile,
                dependencies,
            } if profile.operation == "condition" => {
                let definition =
                    crate::spell::actor_conditions::condition_definition(profile, dependencies)?;
                let source_kind =
                    if definition.condition_type().is_negative() && !facts.target_is_player {
                        ConditionSourceKind::Player
                    } else {
                        ConditionSourceKind::SelfUse
                    };
                match store.apply(&definition, source.clone(), source_kind, immunities, facts) {
                    Ok(_) | Err(ConditionRefusal::KeptCurrent) | Err(ConditionRefusal::Immune) => {}
                    Err(_) => return reject(),
                }
            }
            ResolvedEffect::ResolvedOther { profile, .. } if profile.operation == "create_item" => {
            }
            _ => return reject(),
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent>,
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    prepare_inner(
        tx,
        root,
        authority,
        runtime,
        states,
        room,
        objects,
        content,
        owned,
        book,
        spell,
        intent,
        command,
        occurrence,
        training_occurrence,
        now,
        rune,
        None,
        attack_target,
        draw,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_named(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    parameter: &oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent,
    proposal: &super::super::parameter_cast::QualifiedNamedPlayer,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    proposal
        .validate_in_transaction(tx, runtime, authority)
        .await
        .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    let named_target = proposal.actor();
    if parameter.intent != *intent
        || intent.target != SpellTarget::None
        || intent.aim_at_target
        || spell.aggressive
        || spell
            .authored
            .as_ref()
            .is_none_or(|p| p.header.targeting.parameter != "player_name")
        || !states
            .actors
            .iter()
            .any(|(a, s, _)| *a == named_target && states.get(runtime, *a, *s).is_some())
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let p = runtime
        .read_actor_position(named_target)
        .map_err(|_| SpellCastDisposition::TargetIllegal)?
        .position();
    let resolved = SpellCastIntent {
        target: SpellTarget::Position(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        }),
        ..*intent
    };
    prepare_inner(
        tx,
        root,
        authority,
        runtime,
        states,
        room,
        objects,
        content,
        owned,
        book,
        spell,
        &resolved,
        command,
        occurrence,
        training_occurrence,
        now,
        None,
        Some((named_target, parameter)),
        None,
        draw,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn prepare_inner(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent>,
    named: Option<(
        ExactActorRef,
        &oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent,
    )>,
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    if !applicable(spell)
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || (matches!(spell.carrier, crate::spell::Carrier::Rune { .. }) && rune.is_none())
    {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=bindings");
    let b = owned.binding();
    let actual = runtime.binding();
    if b.session != command.game_session_id()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
        || authority.scope_generation() != actual.scope_generation().get()
    {
        return reject();
    }
    let actor = b.actor;
    let session = b.session;
    let source_protected = runtime
        .current_player_reentry_protection(actor, session, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if spell.aggressive && source_protected {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let before = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let caster = owned
        .caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=caster_qualified");
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let party = if matches!(spell.execution, Execution::PartyBuff(_))
        || matches!(
            before.source_party_vocation(),
            crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
        ) {
        Some(
            crate::gameplay_transport::party_spell_owner::read_source_party_world_in_transaction(
                tx, root, authority, runtime, states, actor, session,
            )
            .await
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let creatures = roster
        .iter()
        .filter(|(_, _, s)| s.is_none())
        .map(|(a, _, _)| {
            runtime
                .companion_snapshot(*a)
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let players = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .cloned()
        .collect::<Vec<_>>();
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let caster_position = tile(position.position());
    // SPELL-TARGET-1: the held ATTACK-0 §4 target. No target is `TargetRequired`; one that is no
    // longer a visible creature is `TargetIllegal`.
    let attack_target_actor = match intent.target {
        SpellTarget::AttackTarget => {
            let held = attack_target.ok_or(SpellCastDisposition::TargetRequired)?;
            let creature = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| {
                    c.actor.placement_identity() == held.identity && c.generation == held.generation
                })
                .map(|c| c.actor)
                .ok_or(SpellCastDisposition::TargetIllegal)?;
            let seen = runtime
                .read_actor_position(creature)
                .map_err(|_| SpellCastDisposition::TargetIllegal)?;
            if !crate::gameplay_transport::attack::sees(position.position(), seen.position()) {
                return Err(SpellCastDisposition::TargetIllegal);
            }
            Some((creature, tile(seen.position())))
        }
        _ => None,
    };
    let attack_target_tile = attack_target_actor.map(|(_, t)| t);
    let attack_target_actor = attack_target_actor.map(|(a, _)| a);
    // A directional or aimed spell keeps the caster as geometry origin; the target only picks
    // the facing (`aim_at_target`). A single-target spell is centred on the target.
    let directional = profile.header.targeting.needs_direction || spell.target_or_direction;
    let origin = match intent.target {
        SpellTarget::Position(p) => TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        SpellTarget::None
            if profile.header.targeting.needs_direction || spell.target_or_direction =>
        {
            let d = position
                .facing()
                .map(direction)
                .ok_or(SpellCastDisposition::TargetRequired)?;
            let (dx, dy) = delta(d);
            offset(caster_position, dx, dy)?
        }
        SpellTarget::None => caster_position,
        SpellTarget::AttackTarget => {
            let target = attack_target_tile.ok_or(SpellCastDisposition::TargetRequired)?;
            attack_target_origin(caster_position, target, directional)?.0
        }
    };
    if origin.floor != caster_position.floor {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let facing = if let Some(target) = attack_target_tile {
        attack_target_origin(caster_position, target, directional)?.1
    } else if intent.aim_at_target && origin != caster_position {
        Some(facing_to(caster_position, origin))
    } else {
        position.facing().map(direction)
    };
    let facing = facing.unwrap_or(Direction::North);
    let mut candidates = BTreeSet::from([caster_position, origin]);
    for a in &profile.dependencies.abilities {
        candidates.extend(geometry(a, origin, facing)?);
    }
    let mut bindings = Vec::new();
    let mut chain_creatures = Vec::new();
    for (a, p, _) in &roster {
        let p = tile(p.position());
        if p.floor != caster_position.floor
            || p.x.abs_diff(caster_position.x) > MAX_SIGHT_DISTANCE
            || p.y.abs_diff(caster_position.y) > MAX_SIGHT_DISTANCE
        {
            continue;
        }
        let atom = if runtime.contains_live_creature(*a) {
            runtime
                .creature_spell_target_atom(*a)
                .map_err(|_| SpellCastDisposition::Rejected)?
        } else {
            crate::spell::combat_execution::actor_atom(*a)
        };
        let id = u64::from(a.actor_local_id());
        bindings.push(LiveActorBinding {
            source_id: id,
            actor: *a,
            target_atom: atom.clone(),
        });
        chain_creatures.push(ChainCreature {
            id,
            actor: atom,
            position: p,
        });
        candidates.insert(p);
        if spell.chain.is_some() {
            for dx in -9..=9 {
                for dy in -9..=9 {
                    candidates.insert(offset(p, dx, dy)?);
                }
            }
        }
    }
    if bindings.len() > crate::spell::combat_batch::MAX_EFFECTS {
        return reject();
    }
    let mut paths = BTreeMap::new();
    let sight_origin = if profile.header.targeting.needs_direction {
        caster_position
    } else {
        origin
    };
    for p in candidates.clone() {
        paths.insert((sight_origin, p), sight_steps(sight_origin, p)?);
    }
    paths.insert(
        (caster_position, origin),
        sight_steps(caster_position, origin)?,
    );
    if let Some(target) = attack_target_tile {
        paths.insert(
            (caster_position, target),
            sight_steps(caster_position, target)?,
        );
        candidates.insert(target);
    }
    if spell.chain.is_some() {
        for a in chain_creatures
            .iter()
            .chain(std::iter::once(&ChainCreature {
                id: u64::from(actor.actor_local_id()),
                actor: String::new(),
                position: caster_position,
            }))
        {
            for c in &chain_creatures {
                if a.position.x.abs_diff(c.position.x) <= 9
                    && a.position.y.abs_diff(c.position.y) <= 9
                {
                    paths.insert(
                        (a.position, c.position),
                        sight_steps(a.position, c.position)?,
                    );
                }
            }
        }
    }
    for steps in paths.values() {
        candidates.extend(steps.iter().map(|(p, _)| *p));
    }
    if candidates.len() > MAX_WORLD_TILES {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=tile_reads");
    let mut tiles = BTreeMap::new();
    for p in candidates {
        tiles.insert(
            p,
            read_tile(tx, authority, room, runtime, objects, p).await?,
        );
    }
    let caster_tile = tiles
        .get(&caster_position)
        .ok_or(SpellCastDisposition::Rejected)?;
    if !caster_tile.present {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=tiles_qualified");
    let sight = paths
        .into_iter()
        .filter_map(|(pair, steps)| {
            steps
                .iter()
                .all(|(p, exempt)| *exempt || tiles.get(p).is_some_and(|t| !t.projectile))
                .then_some(pair)
        })
        .collect::<BTreeSet<_>>();
    let legal = chain_creatures
        .iter()
        .filter(|c| {
            let Some(t) = tiles.get(&c.position) else {
                return false;
            };
            if !t.present || t.floor_change || t.solid {
                return false;
            }
            if spell.aggressive {
                creatures
                    .iter()
                    .find(|m| u64::from(m.actor.actor_local_id()) == c.id)
                    .is_some_and(|m| {
                        m.health > 0
                            && m.state.master.is_none()
                            && m.state.policy.flags.attackable
                            && !t.protection
                            && !caster_tile.protection
                    })
            } else {
                true
            }
        })
        .map(|c| c.id)
        .collect::<BTreeSet<_>>();
    let selected = if matches!(
        intent.target,
        SpellTarget::Position(_) | SpellTarget::AttackTarget
    ) {
        chain_creatures
            .iter()
            .filter(|c| {
                attack_target_actor.is_none_or(|t| u64::from(t.actor_local_id()) == c.id)
                    && c.position == attack_target_tile.unwrap_or(origin)
                    && legal.contains(&c.id)
                    && named.is_none_or(|(a, _)| u64::from(a.actor_local_id()) == c.id)
            })
            .min_by_key(|c| c.id)
    } else {
        None
    };
    let selected_actor = selected.and_then(|c| bindings.iter().find(|b| b.source_id == c.id));
    if let (SpellTarget::AttackTarget, Some(target)) = (intent.target, attack_target_tile) {
        let distance = target
            .x
            .abs_diff(caster_position.x)
            .max(target.y.abs_diff(caster_position.y));
        if selected_actor.is_none()
            || !(caster_position == target || sight.contains(&(caster_position, target)))
            || profile
                .header
                .targeting
                .range_tiles
                .is_some_and(|range| distance > range)
        {
            return Err(SpellCastDisposition::TargetIllegal);
        }
    }
    if named.is_some_and(|(a, _)| selected_actor.is_none_or(|b| b.actor != a)) {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    if selected_actor.is_some_and(|t| t.actor == actor) && !profile.header.targeting.allow_on_self {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    // Name-directed healing uses player owners; rune self/own-summon rules
    // remain explicitly owned by allowed_targets and the actual master binding.
    if profile.header.targeting.parameter == "player_name"
        && selected_actor.is_some_and(|t| runtime.contains_live_creature(t.actor))
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let target = selected.map(|c| crate::spell::target::CastTarget {
        caster: u64::from(actor.actor_local_id()),
        creature: c.id,
        actor: c.actor.clone(),
        master: creatures
            .iter()
            .find(|m| u64::from(m.actor.actor_local_id()) == c.id)
            .and_then(|m| m.state.master.map(|m| u64::from(m.actor.actor_local_id()))),
    });
    let operational = OperationalCastFacts {
        caster_position,
        target_position: matches!(
            intent.target,
            SpellTarget::Position(_) | SpellTarget::AttackTarget
        )
        .then_some(origin),
        target,
        line_of_sight_clear: Some(
            caster_position == origin || sight.contains(&(caster_position, origin)),
        ),
        direction_available: position.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: caster_tile.protection,
        target_tile_solid: tiles.get(&origin).map(|t| t.solid),
        target_tile_creature: Some(chain_creatures.iter().any(|c| c.position == origin)),
    };
    let world = World {
        origin,
        sight_origin,
        direction: facing,
        caster: ChainCreature {
            id: u64::from(actor.actor_local_id()),
            actor: crate::spell::combat_execution::actor_atom(actor),
            position: caster_position,
        },
        creatures: chain_creatures,
        legal,
        sight,
        tiles,
    };
    let chain_start = ChainStart {
        target: selected_actor.map(|b| b.source_id),
        attacked: None,
    };
    let prepare_paid = |draw: &mut dyn FnMut(i64, i64) -> i64| {
        prepare_ordinary_owner_cast_with_caster(
            book,
            &before,
            spell,
            &operational,
            now,
            &caster,
            party
                .as_ref()
                .map(|p| p as &dyn crate::spell::party::PartyWorld),
            spell
                .chain
                .as_ref()
                .map(|_| (&world as &dyn ChainWorld, chain_start)),
            draw,
        )
    };
    // Entire footprint, magnitude prerequisites, owner stores and outbox capacity
    // are qualified using a non-executed preview before consuming the live stream.
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_preview operational_target={} position_target={} direction={} protection={} needs_target={} target_or_direction={}",
        operational.target.is_some(),
        operational.target_position.is_some(),
        operational.direction_available,
        operational.in_protection_zone,
        spell.needs_target,
        spell.target_or_direction
    );
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_preview caster_state pending_training={} level={} owned_magic={:?} qualified_level={} qualified_magic={} vocation_matches={}",
        before.pending_training_checkpoint().is_some(),
        before.character_facts().level,
        before.owned_effective_magic_level(now.get()),
        caster.level,
        caster.magic_level,
        caster.vocation == before.character_facts().vocation
    );
    let preview = prepare_paid(&mut |minimum, _| minimum).inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!(
            "SEAM_EVIDENCE ordinary_combat_prepare stage=paid_preview_refused reason={_reason:?}"
        );
    })?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=paid_preview_qualified");
    let preview_output = lower(
        runtime,
        states,
        room,
        content,
        owned,
        spell,
        intent,
        rune,
        named,
        &before,
        &preview,
        &caster,
        &operational,
        &bindings,
        &world,
        &party,
        command,
        &occurrence,
        now,
        &mut |minimum, _| minimum,
    )
    .inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!(
            "SEAM_EVIDENCE ordinary_combat_prepare stage=lower_preview_refused reason={_reason:?}"
        );
    })?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=lower_preview_qualified");
    let formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::Rejected)?;
    before
        .prepare_paid_training(
            &preview.next,
            &preview.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_reason| {
            #[cfg(test)]
            eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=training_or_runtime_preview_refused reason={_reason:?}");
            SpellCastDisposition::Rejected
        })?;
    stage_player_batch(
        runtime,
        states,
        &preview_output.batch,
        Some(preview.next.clone()),
    ).inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=player_batch_preview_refused reason={_reason:?}");
    })?;
    runtime
        .stage_spell_batch(&preview_output.batch)
        .map_err(|_reason| {
            #[cfg(test)]
            eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=training_or_runtime_preview_refused reason={_reason:?}");
            SpellCastDisposition::Rejected
        })?;
    let preview_requests = crate::spell::ordinary_timer::ordinary_timer_requests(
        runtime,
        spell,
        &preview_output.batch,
        &occurrence,
        caster_position,
        &preview_output.delayed,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let stamp = if preview_requests.is_empty() {
        None
    } else {
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        timer_preflight(
            runtime,
            states,
            &preview_output.batch,
            preview_requests,
            stamp,
        )?;
        Some(stamp)
    };
    let paid = prepare_paid(draw)?;
    let output = lower(
        runtime,
        states,
        room,
        content,
        owned,
        spell,
        intent,
        rune,
        named,
        &before,
        &paid,
        &caster,
        &operational,
        &bindings,
        &world,
        &party,
        command,
        &occurrence,
        now,
        draw,
    )?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(runtime, states, &output.batch, Some(paid.next.clone()))?;
    runtime
        .stage_spell_batch(&output.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let timers = if output.delayed.is_empty() {
        None
    } else {
        let requests = crate::spell::ordinary_timer::ordinary_timer_requests(
            runtime,
            spell,
            &output.batch,
            &occurrence,
            caster_position,
            &output.delayed,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
        Some(timer_preflight(
            runtime,
            states,
            &output.batch,
            requests,
            stamp.ok_or(SpellCastDisposition::Rejected)?,
        )?)
    };
    Ok(PreparedNativeCombatCast {
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        party,
        before,
        paid: paid.next,
        batch: output.batch,
        tile_facts: world.tiles,
        roster,
        training,
        timers,
        magnitude: output.magnitude,
        facts_binding: b.clone(),
        creatures,
        players,
        equipment: owned.equipment().clone(),
        presentation: Some(output.presentation),
        installation: None,
        item_creations: output.items,
    })
}

struct Output {
    batch: OwnerCombatBatch,
    magnitude: Option<PreparedMagnitudeOwner>,
    delayed: Vec<TimedCombatEffect>,
    presentation: crate::gameplay_transport::spell_presentations::PreparedPresentation,
    items: Vec<OrdinaryItemCreation>,
}
#[allow(clippy::too_many_arguments)]
fn lower(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    rune: Option<&oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent>,
    named: Option<(
        ExactActorRef,
        &oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent,
    )>,
    before: &PlayerSpellState,
    paid: &PaidOrdinaryCast,
    _caster: &crate::spell::CasterState,
    operational: &OperationalCastFacts,
    bindings: &[LiveActorBinding],
    world: &World,
    party: &Option<crate::gameplay_transport::party_spell_owner::SourcePartyWorld>,
    command: CommandRef,
    occurrence: &AbilityOccurrence,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<Output, SpellCastDisposition> {
    let mut applications: Vec<(u64, u64, Vec<ResolvedEffect>)> = Vec::new();
    let selected_ability = if matches!(spell.execution, Execution::PartyBuff(_)) {
        None
    } else {
        Some(ability(spell, paid.resolution.ability_variant.as_ref())?)
    };
    let geometry = selected_ability
        .map(|a| geometry(a, world.origin, world.direction))
        .transpose()?
        .unwrap_or_default();
    if !paid.resolution.chain.is_empty() {
        for hit in &paid.resolution.chain {
            applications.push((
                hit.hit.creature,
                hit.hit.delay_micros / 1000,
                hit.effects.clone(),
            ));
        }
    } else if !paid.resolution.party.is_empty() {
        for member in &paid.resolution.party {
            applications.push((member.creature, 0, member.effects.clone()));
        }
    } else {
        let targets: Vec<u64> = if selected_ability.is_some_and(|a| a.area.is_some()) {
            world
                .creatures
                .iter()
                .filter(|c| {
                    geometry.contains(&c.position)
                        && world.legal.contains(&c.id)
                        && world.area_sight_clear(c.position)
                })
                .map(|c| c.id)
                .collect()
        } else if let Some(t) = &operational.target {
            vec![t.creature]
        } else if spell.self_target {
            vec![world.caster.id]
        } else if spell.target_or_direction {
            let facing = runtime
                .read_actor_position(owned.binding().actor)
                .map_err(|_| SpellCastDisposition::Rejected)?
                .facing()
                .map(direction)
                .ok_or(SpellCastDisposition::TargetRequired)?;
            let (dx, dy) = delta(facing);
            let front = offset(world.caster.position, dx, dy)?;
            world
                .creatures
                .iter()
                .filter(|c| c.position == front && world.legal.contains(&c.id))
                .map(|c| c.id)
                .collect()
        } else {
            Vec::new()
        };
        // Source area combat computes one base roll before visiting its tiles.
        // Target mitigation remains independently resolved for each actual owner.
        for id in targets {
            applications.push((id, 0, paid.resolution.effects.clone()));
        }
    }
    if applications.len() > crate::spell::combat_batch::MAX_EFFECTS {
        return reject();
    }
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_lower stage=applications_qualified applications={} geometry={}",
        applications.len(),
        geometry.len()
    );
    let source_protected = runtime
        .current_player_reentry_protection(
            owned.binding().actor,
            command.game_session_id(),
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if source_protected
        && applications.iter().any(|(id, _, effects)| {
            *id != world.caster.id
                && bindings
                    .iter()
                    .find(|b| b.source_id == *id)
                    .is_some_and(|b| !runtime.contains_live_creature(b.actor))
                && effects.iter().any(heals_health)
        })
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let mut hits = Vec::new();
    let mut healing = None;
    let mut damage_element = None;

    for (id, delay, effects) in &applications {
        for effect in effects {
            match effect {
                ResolvedEffect::Damage {
                    damage_type,
                    magnitude,
                } => {
                    let e = element(damage_type)?;
                    if healing == Some(true) || damage_element.is_some_and(|old| old != e) {
                        return reject();
                    }
                    healing = Some(false);
                    damage_element = Some(e);
                    if *magnitude > 0 {
                        hits.push(MagnitudePlan {
                            target: *id,
                            magnitude: *magnitude,
                            bonus_percent: 0,
                            side_percent: None,
                            delay_ms: *delay,
                        });
                    }
                }
                ResolvedEffect::Heal { magnitude } => {
                    if healing == Some(false) {
                        return reject();
                    }
                    healing = Some(true);
                    damage_element = Some(Element::Healing);
                    if *magnitude > 0 {
                        hits.push(MagnitudePlan {
                            target: *id,
                            magnitude: *magnitude,
                            bonus_percent: 0,
                            side_percent: None,
                            delay_ms: *delay,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    let block_armor = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?
        .dependencies
        .effects
        .iter()
        .any(|e| {
            e.mitigated_by
                .as_ref()
                .is_some_and(|m| m.iter().any(|m| m == "armor"))
        });
    let source_delays = hits
        .iter()
        .map(|h| (h.target, h.delay_ms))
        .collect::<Vec<_>>();
    // The numerical owner qualifies original cast-time snapshots. The timer
    // retains the source delay independently and consumes no second payment.
    for hit in &mut hits {
        hit.delay_ms = 0;
    }
    let plan = NativeCombatPlan::Combat(CombatPlan {
        element: damage_element.unwrap_or(Element::Healing),
        area: None,
        hits,
        cooldown_ms: None,
        shared_cooldown: None,
        reduce_all_spell_cooldowns_ms: 0,
        block_armor,
        use_weapon_charges: false,
        weapon_missile: false,
        dispel_paralysis_targets: Vec::new(),
        resolve_critical_and_fatal_once: selected_ability.is_some_and(|a| a.area.is_some()),
    });
    let players = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .map(|(a, _, p)| (*a, p))
        .collect::<Vec<_>>();
    let has_magnitude = paid.resolution.chain.is_empty()
        && matches!(&plan,NativeCombatPlan::Combat(p) if !p.hits.is_empty());
    let mut magnitude = if has_magnitude {
        Some(
            PreparedMagnitudeOwner::qualify(
                runtime,
                before,
                owned.binding().actor,
                command.game_session_id(),
                owned.magnitude(),
                &plan,
                bindings,
                &players,
                now.get() / 1000,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let mut batch=OwnerCombatBatch{caster:owned.binding().actor,attacker:CharacterId::decode(&owned.binding().character).map_err(|_|SpellCastDisposition::Rejected)?,
        current_lease_generation:owned.binding().lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,
        effects:Vec::new(),deferred:None,binding:serde_json::to_vec(&json!({"intent":match named {Some((_,parameter))=>source_parameter_intent(&parameter.intent,parameter)?,None=>source_cast_intent(intent,rune)},"source_named_target":named.map(|(a,_)|format!("{a:?}")),"parameter_result":if named.is_some(){Some(oteryn_protocol_oteryn::actor_spell_v2::encode_parameter_spell_cast_result(&oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastResult{disposition:SpellCastDisposition::Cast,feedback:None,editor:None}).map_err(|_|SpellCastDisposition::Rejected)?)}else{None},"source_definition":format!("{spell:?}"),
            "capture_format":"ordinary-owner-r21","source_world_capture_digest":sha2::Sha256::digest(format!("{world:?}").as_bytes()).to_vec(),"source_hit_delays":source_delays,"source_chain_origins":paid.resolution.chain.iter().map(|hit|json!([hit.hit.creature,hit.hit.step,hit.hit.from.x,hit.hit.from.y,hit.hit.from.floor])).collect::<Vec<_>>(),"equipment_facts":format!("{:?}",owned.equipment()),
            "magnitude_facts":format!("{:?}",owned.magnitude()),"selected_variant":format!("{:?}",paid.resolution.ability_variant),
            "source_party":party.as_ref().map(|p|format!("{:?}",p.source()))})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut delayed = Vec::new();
    let mut ordinal = 0u16;
    if has_magnitude {
        let lowered = lower_native(
            runtime,
            batch,
            world.caster.id,
            &plan,
            bindings,
            ordinal,
            &mut |p, hit, target| {
                magnitude
                    .as_mut()
                    .ok_or(Error::InvalidBatch)?
                    .finish(p, hit, target, draw)
            },
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
        batch = lowered.batch;
        delayed = lowered.delayed;
        ordinal = lowered.next_sub_ordinal;
        let mut immediate = Vec::new();
        for effect in batch.effects.drain(..) {
            let id = bindings
                .iter()
                .find(|b| b.actor == effect.target)
                .ok_or(SpellCastDisposition::Rejected)?
                .source_id;
            let delay_ms = source_delays
                .iter()
                .find(|(target, _)| *target == id)
                .map(|(_, delay)| *delay)
                .unwrap_or(0);
            if delay_ms == 0 {
                immediate.push(effect)
            } else {
                delayed.push(TimedCombatEffect { delay_ms, effect })
            }
        }
        batch.effects = immediate;
    }
    if !paid.resolution.chain.is_empty() {
        // Only the selected exact recipients and source phases are captured at
        // cast time. The actual timer resolves numerical damage at its deadline.
        for (id, delay, effects) in &applications {
            let [ResolvedEffect::Damage { .. }] = effects.as_slice() else {
                return reject();
            };
            let target = bindings
                .iter()
                .find(|b| b.source_id == *id)
                .ok_or(SpellCastDisposition::Rejected)?;
            delayed.push(TimedCombatEffect {
                delay_ms: *delay,
                effect: OwnerCombatEffect {
                    target: target.actor,
                    sub_ordinal: ordinal,
                    change: OwnerCombatChange::Damage {
                        target_atom: target.target_atom.clone(),
                        magnitude: 0,
                    },
                },
            });
            ordinal = ordinal
                .checked_add(1)
                .ok_or(SpellCastDisposition::Rejected)?;
        }
    }
    let decision = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(nonce(
        b"oteryn:ordinary-condition-occurrence:v1",
        batch.caster,
        command.game_session_id(),
        command.command_id().get(),
        &[],
    ));
    let stream =
        oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest());
    let application_targets = applications
        .iter()
        .map(|(id, _, _)| *id)
        .collect::<BTreeSet<_>>();
    let mut cues = Vec::new();
    let mut items = Vec::new();
    for (id, delay, effects) in applications {
        let target = bindings
            .iter()
            .find(|b| b.source_id == id)
            .ok_or(SpellCastDisposition::Rejected)?;
        let mut change = None;
        if runtime.contains_live_creature(target.actor) {
            let expected = runtime
                .companion_snapshot(target.actor)
                .map_err(|_| SpellCastDisposition::Rejected)?;
            let mut next = expected.state.clone();
            let facts = crate::ability::condition::ApplicationFacts {
                now: now.get(),
                base_speed: u16::try_from(next.policy.base_speed)
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: source_protected,
                target_is_player: false,
                decision_root: &stream,
                occurrence: decision,
            };
            modify_conditions(
                &mut next.conditions,
                &effects,
                Some(batch.caster),
                &next.policy.condition_immunities,
                &facts,
            )?;
            if next.conditions != expected.state.conditions {
                change = Some(OwnerCombatChange::CompanionConditions(Box::new(
                    crate::foundation::runtime_actor_spell_types::CompanionConditionUpdate {
                        expected,
                        next,
                    },
                )));
            }
        } else {
            let (_, session, state) = states
                .actors
                .iter()
                .find(|(a, _, _)| *a == target.actor)
                .ok_or(SpellCastDisposition::Rejected)?;
            let actual = states
                .get(runtime, target.actor, *session)
                .ok_or(SpellCastDisposition::Rejected)?;
            let expected = actual.owned_conditions().clone();
            let mut next = expected.clone();
            let facts = crate::ability::condition::ApplicationFacts {
                now: now.get(),
                base_speed: u16::try_from(state.owned_base_speed())
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                mana_shield_capacity: 0,
                target_reentry_protected: runtime
                    .current_player_reentry_protection(target.actor, *session, now.get())
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                source_reentry_protected: source_protected,
                target_is_player: true,
                decision_root: &stream,
                occurrence: decision,
            };
            modify_conditions(
                &mut next,
                &effects,
                Some(crate::spell::combat_execution::actor_atom(batch.caster)),
                &[],
                &facts,
            )?;
            if next != expected {
                change = Some(OwnerCombatChange::PlayerConditions {
                    expected: Box::new(expected),
                    next: Box::new(next),
                });
            }
        }
        if let Some(change) = change {
            let effect = OwnerCombatEffect {
                target: target.actor,
                sub_ordinal: ordinal,
                change,
            };
            ordinal = ordinal
                .checked_add(1)
                .ok_or(SpellCastDisposition::Rejected)?;
            if delay == 0 {
                batch.effects.push(effect)
            } else {
                delayed.push(TimedCombatEffect {
                    delay_ms: delay,
                    effect,
                })
            }
        }
    }
    let source = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(p) = &source.header.presentation {
        if let Some(binding) = &p.cast_cue {
            cues.push(LocatedCueRequest {
                binding: binding.clone(),
                target: CueTarget::Actor(batch.caster),
            });
        }
        if let Some(binding) = &p.impact_cue {
            for target in batch
                .effects
                .iter()
                .map(|e| (e.target.placement_identity(), e.target))
                .collect::<BTreeMap<_, _>>()
                .into_values()
            {
                cues.push(LocatedCueRequest {
                    binding: binding.clone(),
                    target: CueTarget::Actor(target),
                });
            }
        }
    }
    let selected_effects = if let Some(a) = selected_ability {
        a.effects
            .iter()
            .map(|reference| {
                source
                    .dependencies
                    .effects
                    .iter()
                    .find(|e| {
                        e.identity.key == reference.key && e.identity.revision == reference.revision
                    })
                    .ok_or(SpellCastDisposition::Rejected)
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        source.dependencies.effects.iter().collect::<Vec<_>>()
    };
    let is_area = selected_ability.is_some_and(|a| a.area.is_some());
    let tile_target =
        matches!(intent.target, SpellTarget::Position(_)) && operational.target.is_none();
    for profile in selected_effects {
        if spell.chain.is_some() {
            continue; // Source chain combat presentations belong to its due turn.
        }
        if profile.operation == "create_item" {
            let item = profile
                .created_item
                .as_ref()
                .ok_or(SpellCastDisposition::Rejected)?;
            for p in &geometry {
                if world
                    .tiles
                    .get(p)
                    .is_some_and(|t| t.present && !t.solid && !t.floor_change && !t.protection)
                    && world.area_sight_clear(*p)
                {
                    items.push(OrdinaryItemCreation {
                        position: *p,
                        item: item.clone(),
                    });
                }
            }
        }
        if let Some(p) = &profile.presentation {
            if let Some(binding) = &p.caster_effect_asset_binding {
                cues.push(LocatedCueRequest {
                    binding: binding.clone(),
                    target: CueTarget::Actor(batch.caster),
                });
            }
            if let Some(binding) = &p.impact_asset_binding {
                if is_area || tile_target {
                    for tile in &geometry {
                        if world
                            .tiles
                            .get(tile)
                            .is_some_and(|t| t.present && !t.solid && !t.floor_change)
                            && world.area_sight_clear(*tile)
                        {
                            cues.push(LocatedCueRequest {
                                binding: binding.clone(),
                                target: CueTarget::Tile(
                                    QualifiedCueTile::from_owners(room, runtime, cell(*tile)).map_err(|_reason| {
                    #[cfg(test)]
                    eprintln!("SEAM_EVIDENCE ordinary_combat_lower stage=tile_cue_owner_refused reason={_reason:?}");
                    SpellCastDisposition::Rejected
                })?,
                                ),
                            });
                        }
                    }
                } else {
                    for id in &application_targets {
                        let target = bindings
                            .iter()
                            .find(|b| b.source_id == *id)
                            .ok_or(SpellCastDisposition::Rejected)?;
                        cues.push(LocatedCueRequest {
                            binding: binding.clone(),
                            target: CueTarget::Actor(target.actor),
                        });
                    }
                }
            }
            if let Some(binding) = &p.projectile_asset_binding {
                if is_area || tile_target {
                    if world.tiles.get(&world.origin).is_some_and(|t| t.present) {
                        cues.push(LocatedCueRequest {
                            binding: binding.clone(),
                            target: CueTarget::Tile(
                                QualifiedCueTile::from_owners(room, runtime, cell(world.origin)).map_err(|_reason| {
                    #[cfg(test)]
                    eprintln!("SEAM_EVIDENCE ordinary_combat_lower stage=tile_cue_owner_refused reason={_reason:?}");
                    SpellCastDisposition::Rejected
                })?,
                            ),
                        });
                    }
                } else {
                    for id in &application_targets {
                        let target = bindings
                            .iter()
                            .find(|b| b.source_id == *id)
                            .ok_or(SpellCastDisposition::Rejected)?;
                        cues.push(LocatedCueRequest {
                            binding: binding.clone(),
                            target: CueTarget::Actor(target.actor),
                        });
                    }
                }
            }
        }
    }
    if !items.is_empty() {
        let mut capture: Value =
            serde_json::from_slice(&batch.binding).map_err(|_| SpellCastDisposition::Rejected)?;
        capture["ordinary_item_creations"] = json!(format!("{items:?}"));
        batch.binding = serde_json::to_vec(&capture).map_err(|_| SpellCastDisposition::Rejected)?;
    }
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_lower stage=presentation_prepare cues={} applications={} magnitude_hits={}",
        cues.len(),
        application_targets.len(),
        source_delays.len()
    );
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, cues)
        .map_err(|_reason| {
            #[cfg(test)]
            eprintln!(
                "SEAM_EVIDENCE ordinary_combat_lower stage=presentation_refused reason={_reason:?}"
            );
            SpellCastDisposition::Rejected
        })?;
    Ok(Output {
        batch,
        magnitude,
        delayed,
        presentation,
        items,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::items_after_test_module)]
    #![allow(clippy::panic)]
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    fn source(key: &str) -> SpellDefinition {
        let catalog: Value = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        ))
        .unwrap();
        let row = catalog["bundles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["bundle"]["spell"]["identity"]["key"] == key)
            .unwrap();
        crate::spell::authoring::spell_from_bundle(&row["bundle"], &row["dependencies"]).unwrap()
    }
    #[test]
    fn genuine_lightning_selects_and_pays_once_without_cast_time_draw_then_uses_due_stats() {
        use crate::spell::cast::CharacterCastFacts;
        use crate::spell::{CasterState, Vocation};
        let spell = source("candidate:spell/lightning");
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Sorcerer,
                level: 100,
                magic_level: 50,
                max_health: 500,
                max_mana: 5000,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        state
            .apply_owner_premium_transition(true, Some(100_000_000))
            .unwrap();
        let mut caster = CasterState {
            harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::ONE,
            vocation: Vocation::Sorcerer,
            level: 100,
            magic_level: 50,
            premium: true,
            mana: 5000,
            max_mana: 5000,
            soul: 100,
            learned: BTreeSet::new(),
            attack_skill: 10,
            attack_value: 7,
            attack_factor: 1.0,
            shielding_skill: 10,
            melee_weapon: false,
            shield_defense: None,
        };
        let at = |x| TilePosition { x, y: 0, floor: 7 };
        let world = World {
            origin: at(1),
            sight_origin: at(0),
            direction: Direction::East,
            caster: ChainCreature {
                id: 1,
                actor: "actor:caster".into(),
                position: at(0),
            },
            creatures: [2, 3]
                .map(|id| ChainCreature {
                    id,
                    actor: format!("actor:{id}"),
                    position: at(i32::try_from(id - 1).unwrap()),
                })
                .to_vec(),
            legal: BTreeSet::from([2, 3]),
            sight: BTreeSet::from([(at(0), at(1)), (at(0), at(2)), (at(1), at(2))]),
            tiles: (0..=2)
                .map(|x| {
                    (
                        at(x),
                        TileFlags {
                            present: true,
                            protection: false,
                            solid: false,
                            floor_change: false,
                            projectile: false,
                        },
                    )
                })
                .collect(),
        };
        let facts = OperationalCastFacts {
            caster_position: at(0),
            target_position: Some(at(1)),
            target: Some(crate::spell::target::CastTarget {
                caster: 1,
                creature: 2,
                actor: "actor:2".into(),
                master: None,
            }),
            line_of_sight_clear: Some(true),
            direction_available: true,
            wheel_unlocked: None,
            in_protection_zone: false,
            target_tile_solid: Some(false),
            target_tile_creature: Some(true),
        };
        let before = state.clone();
        let mut cast_draws = 0;
        let book = crate::spell::SpellBook::canonical(vec![spell.clone()]).unwrap();
        let paid = prepare_ordinary_owner_cast_with_caster(
            &book,
            &state,
            &spell,
            &facts,
            SemanticTimeMicros::from_micros(1000),
            &caster,
            None,
            Some((
                &world,
                ChainStart {
                    target: Some(2),
                    attacked: None,
                },
            )),
            &mut |minimum, _| {
                cast_draws += 1;
                minimum
            },
        )
        .unwrap();
        assert_eq!(cast_draws, 0);
        assert_eq!(state, before);
        assert_eq!(
            paid.anchor.paid_mana,
            crate::spell::mana_cost(&spell, &caster)
        );
        assert_eq!(paid.next.vitals().mana, 5000 - paid.anchor.paid_mana);
        assert_eq!(paid.resolution.chain.len(), 2);
        assert_eq!(paid.resolution.chain[0].hit.delay_micros, 0);
        assert_eq!(paid.resolution.chain[1].hit.delay_micros, 50_000);
        let mut due_draws = 0;
        let mut due = |_, maximum| {
            due_draws += 1;
            maximum
        };
        let first =
            crate::spell::cast::resolve_ordinary_due_effects(&spell, &caster, 0, &mut due).unwrap();
        caster.magic_level = 100;
        caster.mana = 0; // A due callback never repeats initial resource eligibility.
        let second =
            crate::spell::cast::resolve_ordinary_due_effects(&spell, &caster, 1, &mut due).unwrap();
        assert_eq!(due_draws, 2);
        let amount = |effects: &[ResolvedEffect]| match &effects[0] {
            ResolvedEffect::Damage { magnitude, .. } => *magnitude,
            _ => panic!("source damage"),
        };
        assert!(amount(&second) > amount(&first));
        assert_eq!(paid.next.vitals().mana, 5000 - paid.anchor.paid_mana);
    }
    #[test]
    fn directional_held_target_keeps_the_caster_as_geometry_origin() {
        let tile = |x, y| TilePosition { x, y, floor: 7 };
        let caster = tile(10, 10);
        let target = tile(14, 10);
        // Fire Wave / Energy Beam: origin is the tile in front of the caster, facing the target.
        assert_eq!(
            attack_target_origin(caster, target, true).unwrap(),
            (tile(11, 10), Some(Direction::East))
        );
        assert_eq!(
            attack_target_origin(caster, tile(8, 8), true).unwrap(),
            (tile(9, 9), Some(Direction::NorthWest))
        );
        // A single-target spell is centred on the target.
        assert_eq!(
            attack_target_origin(caster, target, false).unwrap(),
            (target, None)
        );
    }

    #[test]
    fn tile_aimed_area_visibility_uses_the_populated_sight_origin() {
        // A non-directional rune aimed at a tile populates (aimed tile, candidate) paths.
        let at = |x| TilePosition { x, y: 0, floor: 7 };
        let world = World {
            origin: at(4),
            sight_origin: at(4),
            direction: Direction::East,
            caster: ChainCreature {
                id: 1,
                actor: "actor:caster".into(),
                position: at(0),
            },
            creatures: Vec::new(),
            legal: BTreeSet::new(),
            sight: BTreeSet::from([(at(0), at(4)), (at(4), at(5))]),
            tiles: BTreeMap::new(),
        };
        // The neighbour of the aimed tile is visible from the aimed tile, which is the
        // key the area paths were populated with; the caster has no path to it.
        assert!(world.area_sight_clear(at(5)));
        assert!(world.area_sight_clear(at(4)));
        assert!(!world.sight_clear(world.caster.position, at(5)));
        assert!(!world.area_sight_clear(at(6)));
    }

    #[test]
    fn genuine_berserk_and_cancel_invisibility_are_world_areas_not_self_mutations() {
        let center = TilePosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        for key in [
            "candidate:spell/berserk",
            "candidate:spell/cancel_invisibility",
        ] {
            let spell = source(key);
            assert!(applicable(&spell));
            assert!(has_world_geometry(&spell));
            assert!(!super::super::is_source_self(&spell));
            let cells = geometry(ability(&spell, None).unwrap(), center, Direction::North).unwrap();
            assert!(cells.contains(&center));
            assert!(cells.contains(&TilePosition {
                x: 101,
                y: 100,
                floor: 7
            }));
        }
    }
    #[test]
    fn genuine_fire_wave_keeps_diagonal_matrix_and_rotates_owner_direction() {
        let spell = source("candidate:spell/fire_wave");
        let a = ability(&spell, None).unwrap();
        let center = TilePosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let north = geometry(a, center, Direction::North).unwrap();
        let east = geometry(a, center, Direction::East).unwrap();
        // Canary AREA_WAVE4 uses 3 (active origin), not 2 (origin only).
        assert!(north.contains(&center));
        assert_eq!(north.len(), 12);
        assert!(north.contains(&TilePosition {
            x: 100,
            y: 99,
            floor: 7
        }));
        assert_eq!(north.len(), east.len());
        assert!(east.contains(&TilePosition {
            x: 101,
            y: 100,
            floor: 7
        }));
        let northwest = geometry(a, center, Direction::NorthWest).unwrap();
        assert!(northwest.contains(&center));
        assert_eq!(northwest.len(), 16);
        assert!(northwest.contains(&TilePosition {
            x: 99,
            y: 99,
            floor: 7
        }));
        assert!(northwest.len() > north.len());
        assert!(!super::super::is_source_self(&spell));
    }
    #[test]
    fn source_presentation_only_has_no_state_effect_and_substituted_condition_is_refused() {
        let spell = source("candidate:spell/aura_of_exposed_weakness");
        let mut effect = spell.authored.as_ref().unwrap().dependencies.effects[0].clone();
        assert!(crate::spell::actor_conditions::is_presentation_only(
            &effect
        ));
        effect.duration_ms = Some(1000);
        assert!(!crate::spell::actor_conditions::is_presentation_only(
            &effect
        ));
    }
}

/// Source chain recipient selection is retained, while getCombatDamage and
/// target mitigation run in its due owner turn (combat.cpp 1469,1497).
/// The timer retains this resulting typed effect before a potentially committing
/// retry; it never requests another resource debit or cooldown successor.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn prepare_due(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    target: ExactActorRef,
    step: u32,
    command: CommandRef,
    _occurrence: &AbilityOccurrence,
    now: SemanticTimeMicros,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<(OwnerCombatEffect, PreparedMagnitudeOwner), SpellCastDisposition> {
    let b = owned.binding();
    let binding = runtime.binding();
    if b.session != command.game_session_id()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || authority.scope_generation() != binding.scope_generation().get()
        || !spell.aggressive
        || spell.chain.is_none()
        || target == b.actor
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?;
    let caster = owned
        .numerical_caster(
            before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let creature = runtime
        .companion_snapshot(target)
        .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    if creature.health <= 0
        || creature.state.master.is_some()
        || !creature.state.policy.flags.attackable
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let caster_position = tile(
        runtime
            .read_actor_position(b.actor)
            .map_err(|_| SpellCastDisposition::Rejected)?
            .position(),
    );
    let target_position = tile(
        runtime
            .read_actor_position(target)
            .map_err(|_| SpellCastDisposition::Rejected)?
            .position(),
    );
    for position in [caster_position, target_position] {
        let tile = read_tile(tx, authority, room, runtime, objects, position).await?;
        if !tile.present || tile.protection || tile.floor_change || tile.solid {
            return Err(SpellCastDisposition::TargetIllegal);
        }
    }
    let target_id = u64::from(target.actor_local_id());
    let bindings = vec![
        LiveActorBinding {
            source_id: u64::from(b.actor.actor_local_id()),
            actor: b.actor,
            target_atom: crate::spell::combat_execution::actor_atom(b.actor),
        },
        LiveActorBinding {
            source_id: target_id,
            actor: target,
            target_atom: runtime
                .creature_spell_target_atom(target)
                .map_err(|_| SpellCastDisposition::Rejected)?,
        },
    ];
    let numerical_plan =
        |effects: Vec<ResolvedEffect>| -> Result<NativeCombatPlan, SpellCastDisposition> {
            let [
                ResolvedEffect::Damage {
                    damage_type,
                    magnitude,
                },
            ] = effects.as_slice()
            else {
                return reject();
            };
            Ok(NativeCombatPlan::Combat(CombatPlan {
                element: element(damage_type)?,
                area: None,
                hits: vec![MagnitudePlan {
                    target: target_id,
                    magnitude: *magnitude,
                    bonus_percent: 0,
                    side_percent: None,
                    delay_ms: 0,
                }],
                cooldown_ms: None,
                shared_cooldown: None,
                reduce_all_spell_cooldowns_ms: 0,
                block_armor: spell
                    .authored
                    .as_ref()
                    .ok_or(SpellCastDisposition::Rejected)?
                    .dependencies
                    .effects
                    .iter()
                    .any(|e| {
                        e.mitigated_by
                            .as_ref()
                            .is_some_and(|v| v.iter().any(|s| s == "armor"))
                    }),
                use_weapon_charges: false,
                weapon_missile: false,
                dispel_paralysis_targets: Vec::new(),
                resolve_critical_and_fatal_once: false,
            }))
        };
    let preview = numerical_plan(crate::spell::cast::resolve_ordinary_due_effects(
        spell,
        &caster,
        step,
        &mut |minimum, _| minimum,
    )?)?;
    let players = [(b.actor, before)];
    let qualify = |plan: &NativeCombatPlan| {
        PreparedMagnitudeOwner::qualify(
            runtime,
            before,
            b.actor,
            b.session,
            owned.magnitude(),
            plan,
            &bindings,
            &players,
            now.get() / 1000,
        )
        .map_err(|_| SpellCastDisposition::Rejected)
    };
    let mut preview_owner = qualify(&preview)?;
    let NativeCombatPlan::Combat(preview_combat) = &preview else {
        return reject();
    };
    preview_owner
        .finish(
            &preview,
            &preview_combat.hits[0],
            &bindings[1],
            &mut |minimum, _| minimum,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let plan = numerical_plan(crate::spell::cast::resolve_ordinary_due_effects(
        spell, &caster, step, draw,
    )?)?;
    let mut magnitude = qualify(&plan)?;
    let NativeCombatPlan::Combat(combat) = &plan else {
        return reject();
    };
    let amount = magnitude
        .finish(&plan, &combat.hits[0], &bindings[1], draw)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok((
        OwnerCombatEffect {
            target,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: bindings[1].target_atom.clone(),
                magnitude: amount,
            },
        },
        magnitude,
    ))
}

/// Resolve chain walking effects against the actual due grid and target position.
/// Selection preserves its original `from`; the source reads the recipient's
/// current position at callback, then emits the target effect even without a path.
#[allow(clippy::too_many_arguments)]
pub(in crate::gameplay_transport) async fn due_chain_presentations(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    spell: &SpellDefinition,
    from: TilePosition,
    target: ExactActorRef,
) -> Result<Vec<LocatedCueRequest>, SpellCastDisposition> {
    let spec = spell.chain.as_ref().ok_or(SpellCastDisposition::Rejected)?;
    let to = tile(
        runtime
            .read_actor_position(target)
            .map_err(|_| SpellCastDisposition::TargetIllegal)?
            .position(),
    );
    let mut cues = Vec::new();
    if let Some(binding) = &spec.asset_binding {
        let mut tiles = BTreeMap::new();
        if from.floor == to.floor {
            let radius = i32::try_from(crate::spell::chain::CHAIN_PATH_SEARCH_TILES)
                .map_err(|_| SpellCastDisposition::Rejected)?;
            for y in -radius..=radius {
                for x in -radius..=radius {
                    let p = offset(from, x, y)?;
                    tiles.insert(
                        p,
                        read_tile(tx, authority, room, runtime, objects, p).await?,
                    );
                }
            }
        }
        let world = World {
            origin: from,
            sight_origin: from,
            direction: Direction::North,
            caster: ChainCreature {
                id: 0,
                actor: String::new(),
                position: from,
            },
            creatures: Vec::new(),
            legal: BTreeSet::new(),
            sight: BTreeSet::new(),
            tiles,
        };
        let mut path = world.path(from, to).unwrap_or_default();
        path.push(to);
        for position in path {
            // The recipient may have moved outside the original selector footprint.
            if !read_tile(tx, authority, room, runtime, objects, position)
                .await?
                .present
            {
                return Err(SpellCastDisposition::TargetIllegal);
            }
            cues.push(LocatedCueRequest {
                binding: binding.clone(),
                target: CueTarget::Tile(
                    QualifiedCueTile::from_owners(room, runtime, cell(position))
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                ),
            });
        }
    }
    let source = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    if let Some(p) = &source.header.presentation
        && let Some(binding) = &p.impact_cue
    {
        cues.push(LocatedCueRequest {
            binding: binding.clone(),
            target: CueTarget::Actor(target),
        });
    }
    let selected = ability(spell, None)?;
    for reference in &selected.effects {
        let effect = source
            .dependencies
            .effects
            .iter()
            .find(|e| e.identity.key == reference.key && e.identity.revision == reference.revision)
            .ok_or(SpellCastDisposition::Rejected)?;
        if let Some(p) = &effect.presentation {
            for binding in [&p.impact_asset_binding, &p.projectile_asset_binding]
                .into_iter()
                .flatten()
            {
                cues.push(LocatedCueRequest {
                    binding: binding.clone(),
                    target: CueTarget::Actor(target),
                });
            }
        }
    }
    Ok(cues)
}
