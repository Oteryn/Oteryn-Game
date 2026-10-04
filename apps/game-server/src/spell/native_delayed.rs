//! Read-only occurrence plans for the four source-qualified delayed candidates.
//! Canonical profile validation happens in the authoring reader before this helper.
//! Snapshots and scheduled intents never authorize a world, Item or actor mutation.

use super::chain::TilePosition;
use super::formula::{Formula, FormulaInputs};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Error(pub(crate) String);
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    Cast,
    DelayedOccurrence,
    FieldEvaluate,
    FieldExpiry,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Element {
    Physical,
    Death,
    Fire,
    Energy,
    Holy,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CasterIdentity {
    pub(crate) actor: u64,
    pub(crate) character: u64,
    pub(crate) session_generation: u64,
    pub(crate) world: u64,
    pub(crate) channel: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StanceSnapshot {
    pub(crate) intrinsic: Element,
    pub(crate) resolved: Element,
    pub(crate) state_revision: u64,
    pub(crate) spell_id: u32,
    pub(crate) damage_multiplier_permyriad: i32,
    pub(crate) critical_chance_permyriad: u32,
    pub(crate) critical_damage_permyriad: u32,
    pub(crate) converted: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CasterSnapshot {
    pub(crate) identity: CasterIdentity,
    pub(crate) position: TilePosition,
    pub(crate) inputs: FormulaInputs,
    pub(crate) harmony: u8,
    /// Resolved by the Harmony owner; this planner does not own the resource.
    pub(crate) harmony_multiplier: f64,
    pub(crate) stage: u8,
    pub(crate) stance: Option<StanceSnapshot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TargetSnapshot {
    pub(crate) actor: u64,
    pub(crate) position: TilePosition,
    pub(crate) legal: bool,
    pub(crate) npc: bool,
    pub(crate) protection_zone: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldTileSnapshot {
    pub(crate) position: TilePosition,
    pub(crate) exists: bool,
    pub(crate) immovable_block_solid: bool,
    pub(crate) floor_change: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldItemSnapshot {
    pub(crate) instance: u64,
    pub(crate) item_id: u32,
    pub(crate) owner_character: u64,
    pub(crate) position: TilePosition,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeDelayedFacts {
    pub(crate) event: Event,
    pub(crate) now_ms: u64,
    pub(crate) cast_ms: u64,
    pub(crate) cast: CasterSnapshot,
    /// Fresh read supplied by the actor owner; None means the caster is offline.
    pub(crate) current: Option<CasterSnapshot>,
    pub(crate) cast_position: TilePosition,
    pub(crate) cast_position_exists: bool,
    pub(crate) cast_position_legal: bool,
    pub(crate) caster_in_protection_zone: bool,
    pub(crate) cast_position_in_protection_zone: bool,
    pub(crate) immediate_succeeded: bool,
    pub(crate) explicit_target: Option<u64>,
    /// Preserve qualified spectator traversal order for source tie behavior.
    pub(crate) targets: Vec<TargetSnapshot>,
    pub(crate) clear_sight: BTreeSet<(TilePosition, TilePosition)>,
    pub(crate) field_tiles: Vec<FieldTileSnapshot>,
    pub(crate) field_items: Vec<FieldItemSnapshot>,
    /// Exact item instances created by this cast; never a type-only cleanup list.
    pub(crate) created_field_instances: BTreeSet<u64>,
    /// Grenade's formula bounds captured at cast, supplied from its ScheduledStrike.
    pub(crate) captured_raw_bounds: Option<(i64, i64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueTiming {
    RecomputeAtOccurrence,
    RawBoundsAtCast,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModifierTiming {
    CurrentAtOccurrence,
    CastStanceSnapshot,
    RawCastAndExpiryModifiers,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScheduledStrike {
    pub(crate) due_ms: u64,
    pub(crate) expected_caster: CasterIdentity,
    pub(crate) position: TilePosition,
    pub(crate) explicit_target: Option<u64>,
    pub(crate) value_timing: ValueTiming,
    pub(crate) modifier_timing: ModifierTiming,
    pub(crate) captured_raw_bounds: Option<(i64, i64)>,
    pub(crate) stance: Option<StanceSnapshot>,
    pub(crate) requires_caster_online: bool,
    pub(crate) ignore_caster_floor: bool,
    pub(crate) suppress_charms: bool,
    pub(crate) requires_full_harmony_at_cast: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkerPlan {
    pub(crate) position: TilePosition,
    pub(crate) asset_binding: String,
    pub(crate) remove_at_ms: u64,
    pub(crate) removal_requires_caster_online: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DamageHit {
    pub(crate) actor: u64,
    pub(crate) magnitude: i64,
    pub(crate) minimum: i64,
    pub(crate) maximum: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StrikePlan {
    pub(crate) element: Element,
    pub(crate) origin: TilePosition,
    pub(crate) area_tiles: BTreeSet<TilePosition>,
    pub(crate) chain_targets: Vec<u64>,
    pub(crate) hits: Vec<DamageHit>,
    pub(crate) suppress_charms: bool,
    pub(crate) ignore_caster_floor: bool,
    pub(crate) effect_asset_binding: String,
    pub(crate) modifier_timing: ModifierTiming,
    pub(crate) stance: Option<StanceSnapshot>,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NativeDelayedPlan {
    Cast {
        immediate: Option<StrikePlan>,
        scheduled: Option<Box<ScheduledStrike>>,
        marker: Option<MarkerPlan>,
        cooldown_ms: Option<u64>,
        harmony_consumed: bool,
        /// Source Outburst schedules before observing immediate combat's result.
        scheduled_before_immediate_result: bool,
    },
    Occurrence {
        strike: Option<StrikePlan>,
        marker_removed: bool,
        dropped: bool,
    },
    CreateField {
        positions: Vec<TilePosition>,
        item_key: String,
        item_revision: String,
        owner: CasterIdentity,
        expires_ms: u64,
        cooldown_ms: u64,
        bonus_percent: u32,
        evaluation_interval_ms: u64,
        cleanup_only_created_instances: bool,
    },
    FieldBonus {
        bonus_percent: u32,
        next_evaluation_ms: u64,
    },
    RemoveField {
        instances: Vec<u64>,
    },
}

fn text<'a>(p: &'a Value, key: &str) -> Result<&'a str, Error> {
    p.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error(format!("missing string {key}")))
}
fn number(p: &Value, key: &str) -> Result<u64, Error> {
    p.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error(format!("missing integer {key}")))
}
fn stage_number(p: &Value, key: &str, stage: u8) -> Result<u64, Error> {
    if !(1..=3).contains(&stage) {
        return Err(Error("wheel stage outside 1..3".into()));
    }
    p.get(key)
        .and_then(Value::as_array)
        .and_then(|a| a.get(usize::from(stage - 1)))
        .and_then(Value::as_u64)
        .ok_or_else(|| Error(format!("missing stage integer {key}")))
}
fn deadline(base: u64, delay: u64) -> Result<u64, Error> {
    base.checked_add(delay)
        .ok_or_else(|| Error("occurrence deadline overflow".into()))
}
fn validate(f: &NativeDelayedFacts) -> Result<(), Error> {
    if f.now_ms < f.cast_ms {
        return Err(Error("current time precedes cast".into()));
    }
    if f.cast.harmony > 5
        || f.cast.stage > 3
        || f.targets.len() > 1024
        || f.field_items.len() > 1024
        || f.field_tiles.len() > 9
        || f.created_field_instances.len() > 9
    {
        return Err(Error("bounded snapshot exceeded".into()));
    }
    for actor in std::iter::once(&f.cast).chain(f.current.iter()) {
        if actor.harmony > 5
            || actor.stage > 3
            || !actor.harmony_multiplier.is_finite()
            || actor.harmony_multiplier < 1.0
            || actor.harmony_multiplier > 100.0
        {
            return Err(Error("invalid actor resource snapshot".into()));
        }
        if actor.identity.actor == 0 || actor.identity.character == 0 {
            return Err(Error("zero actor identity".into()));
        }
    }
    let ids: BTreeSet<_> = f.targets.iter().map(|t| t.actor).collect();
    if ids.len() != f.targets.len() || ids.contains(&0) {
        return Err(Error("duplicate or zero target identity".into()));
    }
    let tile_positions: BTreeSet<_> = f.field_tiles.iter().map(|t| t.position).collect();
    if tile_positions.len() != f.field_tiles.len() {
        return Err(Error("duplicate field tile".into()));
    }
    let item_ids: BTreeSet<_> = f.field_items.iter().map(|i| i.instance).collect();
    if item_ids.len() != f.field_items.len() || item_ids.contains(&0) {
        return Err(Error("duplicate or zero field instance".into()));
    }
    Ok(())
}
fn actor_current(f: &NativeDelayedFacts) -> Result<Option<&CasterSnapshot>, Error> {
    let Some(current) = f.current.as_ref() else {
        return Ok(None);
    };
    if current.identity != f.cast.identity {
        return Err(Error(
            "caster identity/session/world/channel changed".into(),
        ));
    }
    Ok(Some(current))
}
fn within(a: TilePosition, b: TilePosition, radius: u32) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x) <= radius && a.y.abs_diff(b.y) <= radius
}
fn sight(f: &NativeDelayedFacts, a: TilePosition, b: TilePosition) -> bool {
    a.floor == b.floor && (a == b || f.clear_sight.contains(&(a, b)))
}
fn formula_bounds(p: &Value, actor: &CasterSnapshot) -> Result<(f64, f64), Error> {
    let fp = p
        .get("formula")
        .ok_or_else(|| Error("missing formula".into()))?;
    let formula = Formula {
        minimum: super::authoring::expression(&fp["minimum"], 1)
            .map_err(|e| Error(e.to_string()))?,
        maximum: super::authoring::expression(&fp["maximum"], 1)
            .map_err(|e| Error(e.to_string()))?,
    };
    let mut inputs = actor.inputs;
    inputs.base_power = Some(
        i64::try_from(number(p, "base_power")?).map_err(|_| Error("base power overflow".into()))?,
    );
    Ok((
        formula
            .minimum
            .evaluate(&inputs)
            .map_err(|e| Error(e.to_string()))?,
        formula
            .maximum
            .evaluate(&inputs)
            .map_err(|e| Error(e.to_string()))?,
    ))
}
fn bounds(raw: (f64, f64), multiplier: f64) -> Result<(i64, i64), Error> {
    let (low, high) = (raw.0 * multiplier, raw.1 * multiplier);
    if !low.is_finite()
        || !high.is_finite()
        || low < 0.0
        || high < low
        || high > f64::from(i32::MAX)
    {
        return Err(Error("invalid damage bounds".into()));
    }
    Ok((low.trunc() as i64, high.trunc() as i64))
}
fn draw_hit(
    actor: u64,
    b: (i64, i64),
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<DamageHit, Error> {
    let value = draw(b.0, b.1);
    if value < b.0 || value > b.1 {
        return Err(Error("damage draw outside bounds".into()));
    }
    Ok(DamageHit {
        actor,
        magnitude: value,
        minimum: b.0,
        maximum: b.1,
    })
}
fn area(p: &Value, origin: TilePosition) -> Result<BTreeSet<TilePosition>, Error> {
    let rows = p
        .get("area")
        .and_then(Value::as_array)
        .ok_or_else(|| Error("missing area".into()))?;
    if rows.len() != 5 {
        return Err(Error("area must have five rows".into()));
    }
    let mut result = BTreeSet::new();
    for (y, row) in rows.iter().enumerate() {
        let cells = row
            .as_array()
            .filter(|r| r.len() == 5)
            .ok_or_else(|| Error("area must have five columns".into()))?;
        for (x, cell) in cells.iter().enumerate() {
            let value = cell
                .as_u64()
                .ok_or_else(|| Error("noninteger area cell".into()))?;
            if value > 3 {
                return Err(Error("invalid area cell".into()));
            }
            if value == 0 {
                continue;
            }
            result.insert(TilePosition {
                x: origin
                    .x
                    .checked_add(x as i32 - 2)
                    .ok_or_else(|| Error("area x overflow".into()))?,
                y: origin
                    .y
                    .checked_add(y as i32 - 2)
                    .ok_or_else(|| Error("area y overflow".into()))?,
                floor: origin.floor,
            });
        }
    }
    Ok(result)
}
fn chain_targets<'a>(
    f: &'a NativeDelayedFacts,
    actor: &CasterSnapshot,
) -> Result<Vec<&'a TargetSnapshot>, Error> {
    let eligible = |t: &&TargetSnapshot| {
        t.legal && !t.npc && !t.protection_zone && t.actor != actor.identity.actor
    };
    let first = f
        .explicit_target
        .and_then(|id| f.targets.iter().find(|t| t.actor == id))
        .filter(|t| {
            eligible(t)
                && within(actor.position, t.position, 4)
                && sight(f, actor.position, t.position)
        });
    let mut selected = Vec::new();
    let mut visited = BTreeSet::from([actor.identity.actor]);
    let mut origin = actor.position;
    if let Some(first) = first {
        selected.push(first);
        visited.insert(first.actor);
        origin = first.position;
    } else if f.explicit_target.is_some() {
        return Err(Error("explicit chain target is illegal".into()));
    }
    while selected.len() < 8 {
        let mut nearest = None;
        let mut distance = u128::MAX;
        for target in f.targets.iter().filter(eligible) {
            if visited.contains(&target.actor)
                || !within(origin, target.position, 4)
                || !sight(f, origin, target.position)
            {
                continue;
            }
            let dx = u128::from(origin.x.abs_diff(target.position.x));
            let dy = u128::from(origin.y.abs_diff(target.position.y));
            let candidate = dx * dx + dy * dy;
            if candidate < distance {
                nearest = Some(target);
                distance = candidate;
            }
        }
        let Some(next) = nearest else {
            break;
        };
        selected.push(next);
        visited.insert(next.actor);
        origin = next.position;
    }
    Ok(selected)
}
fn strike(
    p: &Value,
    f: &NativeDelayedFacts,
    actor: &CasterSnapshot,
    delayed: bool,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<StrikePlan, Error> {
    let name = text(p, "spell_name")?;
    let (element, raw, multiplier) = match name {
        "death echo" => (
            f.cast
                .stance
                .as_ref()
                .map_or(Element::Death, |s| s.resolved),
            formula_bounds(p, actor)?,
            if delayed { 0.5 } else { 1.0 },
        ),
        "divine grenade" => {
            let captured = f
                .captured_raw_bounds
                .ok_or_else(|| Error("missing grenade cast bounds".into()))?;
            if captured.0 < 0 || captured.1 < captured.0 || captured.1 > i64::from(i32::MAX) {
                return Err(Error("invalid captured grenade bounds".into()));
            }
            (Element::Holy, (captured.0 as f64, captured.1 as f64), 1.0)
        }
        "spiritual outburst" => {
            let percent = if delayed {
                match actor.stage {
                    1 => 0.375,
                    2 => 0.5,
                    3 => 0.625,
                    _ => return Err(Error("outburst stage outside 1..3".into())),
                }
            } else {
                1.0
            };
            (
                Element::Physical,
                {
                    let raw = formula_bounds(p, actor)?;
                    (raw.0 * percent, raw.1 * percent)
                },
                actor.harmony_multiplier,
            )
        }
        _ => return Err(Error("unknown delayed spell".into())),
    };
    let b = bounds(raw, multiplier)?;
    let chain = name == "spiritual outburst";
    let tiles = if chain {
        BTreeSet::new()
    } else {
        area(p, f.cast_position)?
    };
    let selected = if chain {
        chain_targets(f, actor)?
    } else {
        f.targets
            .iter()
            .filter(|t| {
                t.legal
                    && !t.npc
                    && !t.protection_zone
                    && t.actor != actor.identity.actor
                    && tiles.contains(&t.position)
                    && sight(f, f.cast_position, t.position)
            })
            .collect()
    };
    if chain && selected.is_empty() {
        return Err(Error("no legal chain target".into()));
    }
    let chain_ids = if chain {
        selected.iter().map(|t| t.actor).collect()
    } else {
        vec![]
    };
    let hits = selected
        .into_iter()
        .map(|t| draw_hit(t.actor, b, draw))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(StrikePlan {
        element,
        origin: f.cast_position,
        area_tiles: tiles,
        chain_targets: chain_ids,
        hits,
        suppress_charms: delayed && name == "death echo",
        ignore_caster_floor: delayed && name == "death echo",
        effect_asset_binding: text(
            p,
            if chain {
                "chain_effect_asset_binding"
            } else {
                "effect_asset_binding"
            },
        )?
        .to_owned(),
        modifier_timing: match name {
            "death echo" => ModifierTiming::CastStanceSnapshot,
            "divine grenade" => ModifierTiming::RawCastAndExpiryModifiers,
            _ => ModifierTiming::CurrentAtOccurrence,
        },
        stance: if name == "death echo" {
            f.cast.stance.clone()
        } else {
            None
        },
    })
}

pub(crate) fn plan(
    key: &str,
    p: &Value,
    f: &NativeDelayedFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeDelayedPlan, Error> {
    validate(f)?;
    match (key, text(p, "spell_name")?) {
        ("owned_field_buff", "divine empowerment") => field(p, f),
        ("delayed_strike", name @ ("death echo" | "divine grenade" | "spiritual outburst")) => {
            let due = deadline(f.cast_ms, number(p, "delay_ms")?)?;
            if f.cast.stage == 0 && name != "death echo" {
                return Err(Error("wheel stage zero refuses cast".into()));
            }
            match f.event {
                Event::Cast => {
                    if f.now_ms != f.cast_ms {
                        return Err(Error("cast snapshot timestamp mismatch".into()));
                    }
                    if !f.cast_position_exists || !f.cast_position_legal {
                        return Err(Error("illegal cast position".into()));
                    }
                    if name == "divine grenade"
                        && f.cast_position_in_protection_zone
                        && !f.caster_in_protection_zone
                    {
                        return Err(Error("grenade protection-zone placement refused".into()));
                    }
                    let full = f.cast.harmony == 5;
                    let schedules = match name {
                        "death echo" => f.immediate_succeeded,
                        "divine grenade" => true,
                        _ => full,
                    };
                    let raw = if name == "divine grenade" {
                        let multiplier = 1.0
                            + stage_number(p, "base_damage_bonus_percent_by_stage", f.cast.stage)?
                                as f64
                                / 100.0;
                        Some(bounds(formula_bounds(p, &f.cast)?, multiplier)?)
                    } else {
                        None
                    };
                    let scheduled = if schedules {
                        Some(Box::new(ScheduledStrike {
                            due_ms: due,
                            expected_caster: f.cast.identity.clone(),
                            position: f.cast_position,
                            explicit_target: f.explicit_target,
                            value_timing: if raw.is_some() {
                                ValueTiming::RawBoundsAtCast
                            } else {
                                ValueTiming::RecomputeAtOccurrence
                            },
                            modifier_timing: match name {
                                "death echo" => ModifierTiming::CastStanceSnapshot,
                                "divine grenade" => ModifierTiming::RawCastAndExpiryModifiers,
                                _ => ModifierTiming::CurrentAtOccurrence,
                            },
                            captured_raw_bounds: raw,
                            stance: f.cast.stance.clone(),
                            requires_caster_online: true,
                            ignore_caster_floor: name == "death echo",
                            suppress_charms: name == "death echo",
                            requires_full_harmony_at_cast: name == "spiritual outburst",
                        }))
                    } else {
                        None
                    };
                    let immediate = if name != "divine grenade" && f.immediate_succeeded {
                        Some(strike(p, f, &f.cast, false, draw)?)
                    } else {
                        None
                    };
                    let marker = if name == "divine grenade" {
                        Some(MarkerPlan {
                            position: f.cast_position,
                            asset_binding: text(p, "marker_asset_binding")?.to_owned(),
                            remove_at_ms: due,
                            removal_requires_caster_online: false,
                        })
                    } else {
                        None
                    };
                    let cooldown = if name == "death echo" {
                        None
                    } else {
                        Some(stage_number(p, "cooldown_ms_by_stage", f.cast.stage)?)
                    };
                    Ok(NativeDelayedPlan::Cast {
                        immediate,
                        scheduled,
                        marker,
                        cooldown_ms: cooldown,
                        harmony_consumed: name == "spiritual outburst" && f.immediate_succeeded,
                        scheduled_before_immediate_result: name == "spiritual outburst" && full,
                    })
                }
                Event::DelayedOccurrence => {
                    if f.now_ms < due {
                        return Err(Error("occurrence fired before deadline".into()));
                    }
                    let marker_removed = name == "divine grenade";
                    let scheduled = name != "death echo" || f.immediate_succeeded;
                    let scheduled =
                        scheduled && (name != "spiritual outburst" || f.cast.harmony == 5);
                    let current = actor_current(f)?;
                    let Some(current) = current.filter(|_| scheduled && f.cast_position_exists)
                    else {
                        return Ok(NativeDelayedPlan::Occurrence {
                            strike: None,
                            marker_removed,
                            dropped: true,
                        });
                    };
                    if name != "death echo" && current.position.floor != f.cast_position.floor {
                        return Ok(NativeDelayedPlan::Occurrence {
                            strike: None,
                            marker_removed,
                            dropped: true,
                        });
                    }
                    Ok(NativeDelayedPlan::Occurrence {
                        strike: Some(strike(p, f, current, true, draw)?),
                        marker_removed,
                        dropped: false,
                    })
                }
                _ => Err(Error("event does not belong to delayed strike".into())),
            }
        }
        _ => Err(Error("unknown delayed native key/name".into())),
    }
}

fn field(p: &Value, f: &NativeDelayedFacts) -> Result<NativeDelayedPlan, Error> {
    if !(1..=3).contains(&f.cast.stage) {
        return Err(Error("field requires wheel stage 1..3".into()));
    }
    let expiry = deadline(f.cast_ms, number(p, "duration_ms")?)?;
    let owner = f.cast.identity.character;
    let eligible_item = |i: &&FieldItemSnapshot| {
        i.item_id == 40450 && i.owner_character == owner && within(f.cast.position, i.position, 1)
    };
    match f.event {
        Event::Cast => {
            if f.now_ms != f.cast_ms {
                return Err(Error("field cast time mismatch".into()));
            }
            let positions = f
                .field_tiles
                .iter()
                .filter(|t| {
                    t.exists
                        && !t.immovable_block_solid
                        && !t.floor_change
                        && within(f.cast.position, t.position, 1)
                })
                .map(|t| t.position)
                .collect();
            Ok(NativeDelayedPlan::CreateField {
                positions,
                item_key: text(&p["field_item"], "key")?.to_owned(),
                item_revision: text(&p["field_item"], "revision")?.to_owned(),
                owner: f.cast.identity.clone(),
                expires_ms: expiry,
                cooldown_ms: stage_number(p, "cooldown_ms_by_stage", f.cast.stage)?,
                bonus_percent: u32::try_from(stage_number(
                    p,
                    "bonus_damage_percent_by_stage",
                    f.cast.stage,
                )?)
                .map_err(|_| Error("bonus overflow".into()))?,
                evaluation_interval_ms: number(p, "evaluation_interval_ms")?,
                cleanup_only_created_instances: true,
            })
        }
        Event::FieldEvaluate => {
            let current = actor_current(f)?;
            let own_tile = current.is_some_and(|c| {
                f.now_ms < expiry
                    && f.field_items
                        .iter()
                        .filter(eligible_item)
                        .any(|i| i.position == c.position)
            });
            let stage = current.map_or(f.cast.stage, |c| c.stage);
            let bonus = if own_tile {
                u32::try_from(stage_number(p, "bonus_damage_percent_by_stage", stage)?)
                    .map_err(|_| Error("bonus overflow".into()))?
            } else {
                0
            };
            Ok(NativeDelayedPlan::FieldBonus {
                bonus_percent: bonus,
                next_evaluation_ms: deadline(f.now_ms, number(p, "evaluation_interval_ms")?)?,
            })
        }
        Event::FieldExpiry => {
            if f.now_ms < expiry {
                return Err(Error("field expires before deadline".into()));
            }
            let instances = f
                .field_items
                .iter()
                .filter(eligible_item)
                .filter(|i| f.created_field_instances.contains(&i.instance))
                .map(|i| i.instance)
                .collect();
            Ok(NativeDelayedPlan::RemoveField { instances })
        }
        _ => Err(Error(
            "strike occurrence does not belong to owned field".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;
    fn position(x: i32, y: i32) -> TilePosition {
        TilePosition { x, y, floor: 7 }
    }
    fn actor() -> CasterSnapshot {
        CasterSnapshot {
            identity: CasterIdentity {
                actor: 1,
                character: 10,
                session_generation: 2,
                world: 3,
                channel: 4,
            },
            position: position(10, 10),
            inputs: FormulaInputs {
                level: 1200,
                magic_level: 100,
                base_power: None,
                attack_skill: 100,
                attack_value: 50,
                attack_factor: 1.0,
                shielding_skill: 0,
                shield_defense: None,
            },
            harmony: 5,
            harmony_multiplier: 1.5,
            stage: 2,
            stance: None,
        }
    }
    fn facts() -> NativeDelayedFacts {
        NativeDelayedFacts {
            event: Event::Cast,
            now_ms: 100,
            cast_ms: 100,
            cast: actor(),
            current: Some(actor()),
            cast_position: position(10, 10),
            cast_position_exists: true,
            cast_position_legal: true,
            caster_in_protection_zone: false,
            cast_position_in_protection_zone: false,
            immediate_succeeded: true,
            explicit_target: None,
            targets: vec![TargetSnapshot {
                actor: 2,
                position: position(10, 10),
                legal: true,
                npc: false,
                protection_zone: false,
            }],
            clear_sight: BTreeSet::new(),
            field_tiles: vec![],
            field_items: vec![],
            created_field_instances: BTreeSet::new(),
            captured_raw_bounds: None,
        }
    }
    fn strike_parameters(name: &str) -> Value {
        // Minimal expression parser fixture isolates world curve and occurrence timing;
        // canonical-profile qualification is the caller's separately tested boundary.
        let curve = json!({"op":"add","args":[{"fn":"level_base_damage_healing","args":[{"var":"level"}]},{"var":"magic_level"}]});
        json!({"spell_name":name,"delay_ms":if name=="divine grenade"{3000}else{1000},"base_power":75,
            "formula":{"minimum":curve.clone(),"maximum":curve},
            "area":[[0,1,1,1,0],[1,1,1,1,1],[1,1,3,1,1],[1,1,1,1,1],[0,1,1,1,0]],
            "effect_asset_binding":"canary.appearance:effect/death_echo",
            "chain_effect_asset_binding":"canary.appearance:effect/blow_white",
            "marker_asset_binding":"canary.appearance:effect/divine_grenade",
            "cooldown_ms_by_stage":[26000,20000,14000],"base_damage_bonus_percent_by_stage":[0,16,32]})
    }
    fn field_parameters() -> Value {
        json!({"spell_name":"divine empowerment","duration_ms":5000,"evaluation_interval_ms":1000,
            "field_item":{"key":"candidate:item/40450","revision":"spell-p2-r21"},
            "cooldown_ms_by_stage":[32000,28000,24000],"bonus_damage_percent_by_stage":[8,10,12]})
    }
    #[test]
    fn all_four_canonical_profiles_reach_their_typed_cast_plans() {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .unwrap();
        let profiles = document["profiles"].as_array().unwrap();
        for name in [
            "death echo",
            "divine empowerment",
            "divine grenade",
            "spiritual outburst",
        ] {
            let profile = profiles
                .iter()
                .find(|p| {
                    p["name"]
                        .as_str()
                        .is_some_and(|v| v.eq_ignore_ascii_case(name))
                        && p["carrier"] == "instant"
                })
                .unwrap_or_else(|| panic!("missing canonical profile: {name}"));
            let bundle = json!({"spell": profile["spell"].clone()});
            let compiled =
                super::super::native::spell_from_bundle(&bundle, &profile["dependencies"])
                    .unwrap_or_else(|e| panic!("profile reader refused {name}: {e:?}"));
            let mut f = facts();
            f.field_tiles.push(FieldTileSnapshot {
                position: position(10, 10),
                exists: true,
                immovable_block_solid: false,
                floor_change: false,
            });
            let result = compiled
                .plan(super::super::native::Facts::Delayed(&f), &mut |low, _| low)
                .unwrap_or_else(|e| panic!("canonical cast refused {name}: {e:?}"));
            let super::super::native::Plan::Delayed(result) = result else {
                panic!("wrong dispatch: {name}")
            };
            match (name, *result) {
                (
                    "divine empowerment",
                    NativeDelayedPlan::CreateField {
                        positions,
                        item_key,
                        item_revision,
                        expires_ms,
                        ..
                    },
                ) => {
                    assert_eq!(positions, vec![position(10, 10)]);
                    assert_eq!(item_key, "candidate:item/40450");
                    assert_eq!(item_revision, "spell-p2-r21");
                    assert_eq!(expires_ms, 5100);
                }
                (
                    "divine grenade",
                    NativeDelayedPlan::Cast {
                        immediate: None,
                        scheduled: Some(schedule),
                        marker: Some(marker),
                        ..
                    },
                ) => {
                    assert_eq!(schedule.due_ms, 3100);
                    assert!(schedule.captured_raw_bounds.is_some());
                    assert_eq!(marker.remove_at_ms, 3100);
                }
                (
                    "death echo" | "spiritual outburst",
                    NativeDelayedPlan::Cast {
                        immediate: Some(strike),
                        scheduled: Some(schedule),
                        ..
                    },
                ) => {
                    assert_eq!(schedule.due_ms, 1100);
                    assert_eq!(strike.hits.len(), 1);
                    assert_eq!(strike.hits[0].actor, 2);
                    assert!(strike.hits[0].magnitude > 0);
                }
                _ => panic!("wrong canonical plan: {name}"),
            }
        }
    }

    #[test]
    fn death_echo_recomputes_stats_keeps_tiles_and_drops_offline() {
        let mut f = facts();
        let p = strike_parameters("death echo");
        let NativeDelayedPlan::Cast {
            immediate: Some(hit),
            scheduled: Some(schedule),
            ..
        } = plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap()
        else {
            panic!("cast plan")
        };
        assert_eq!(hit.hits[0].magnitude, 314); // independent S5 curve at 1200 is 214
        assert_eq!(schedule.due_ms, 1100);
        assert_eq!(hit.area_tiles.len(), 21);
        f.event = Event::DelayedOccurrence;
        f.now_ms = 1100;
        f.current.as_mut().unwrap().inputs.magic_level = 200;
        f.current.as_mut().unwrap().position.floor = 8;
        let NativeDelayedPlan::Occurrence {
            strike: Some(hit),
            dropped: false,
            ..
        } = plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap()
        else {
            panic!("echo")
        };
        assert_eq!(hit.hits[0].magnitude, 207);
        assert!(hit.suppress_charms);
        assert!(hit.ignore_caster_floor);
        f.current = None;
        assert!(matches!(
            plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap(),
            NativeDelayedPlan::Occurrence {
                dropped: true,
                strike: None,
                ..
            }
        ));
    }
    #[test]
    fn grenade_snapshots_bounds_marker_cleanup_survives_logout() {
        let mut f = facts();
        let p = strike_parameters("divine grenade");
        let NativeDelayedPlan::Cast {
            immediate: None,
            scheduled: Some(schedule),
            marker: Some(marker),
            cooldown_ms: Some(20000),
            ..
        } = plan("delayed_strike", &p, &f, &mut |_, _| {
            panic!("no cast damage")
        })
        .unwrap()
        else {
            panic!("grenade")
        };
        assert_eq!(schedule.captured_raw_bounds, Some((364, 364)));
        assert_eq!(marker.remove_at_ms, 3100);
        f.event = Event::DelayedOccurrence;
        f.now_ms = 3100;
        f.captured_raw_bounds = schedule.captured_raw_bounds;
        f.current.as_mut().unwrap().inputs.magic_level = 999;
        let NativeDelayedPlan::Occurrence {
            strike: Some(hit), ..
        } = plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap()
        else {
            panic!("explosion")
        };
        assert_eq!(hit.hits[0].magnitude, 364);
        f.current = None;
        assert_eq!(
            plan("delayed_strike", &p, &f, &mut |_, _| panic!("offline draw")).unwrap(),
            NativeDelayedPlan::Occurrence {
                strike: None,
                marker_removed: true,
                dropped: true
            }
        );
    }
    #[test]
    fn outburst_requires_cast_full_harmony_and_rechecks_current_bonus() {
        let mut f = facts();
        let p = strike_parameters("spiritual outburst");
        f.cast.harmony = 4;
        assert!(matches!(
            plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap(),
            NativeDelayedPlan::Cast {
                scheduled: None,
                ..
            }
        ));
        f.cast.harmony = 5;
        let NativeDelayedPlan::Cast {
            scheduled: Some(schedule),
            harmony_consumed: true,
            ..
        } = plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap()
        else {
            panic!("outburst cast")
        };
        assert!(schedule.requires_full_harmony_at_cast);
        f.event = Event::DelayedOccurrence;
        f.now_ms = 1100;
        f.current.as_mut().unwrap().harmony = 0;
        f.current.as_mut().unwrap().harmony_multiplier = 1.0;
        let NativeDelayedPlan::Occurrence {
            strike: Some(hit), ..
        } = plan("delayed_strike", &p, &f, &mut |a, _| a).unwrap()
        else {
            panic!("outburst repeat")
        };
        assert_eq!(hit.hits[0].magnitude, 157);
        assert_eq!(hit.chain_targets, vec![2]);
    }
    #[test]
    fn owned_field_creation_filters_tiles_and_exact_cleanup_preserves_other_owner() {
        let mut f = facts();
        let p = field_parameters();
        f.field_tiles = vec![
            FieldTileSnapshot {
                position: position(10, 10),
                exists: true,
                immovable_block_solid: false,
                floor_change: false,
            },
            FieldTileSnapshot {
                position: position(11, 10),
                exists: true,
                immovable_block_solid: false,
                floor_change: true,
            },
        ];
        let NativeDelayedPlan::CreateField {
            positions,
            expires_ms: 5100,
            bonus_percent: 10,
            ..
        } = plan("owned_field_buff", &p, &f, &mut |_, _| panic!("field draw")).unwrap()
        else {
            panic!("field")
        };
        assert_eq!(positions, vec![position(10, 10)]);
        f.field_items = vec![
            FieldItemSnapshot {
                instance: 1,
                item_id: 40450,
                owner_character: 10,
                position: position(10, 10),
            },
            FieldItemSnapshot {
                instance: 2,
                item_id: 40450,
                owner_character: 20,
                position: position(10, 10),
            },
            FieldItemSnapshot {
                instance: 3,
                item_id: 40450,
                owner_character: 10,
                position: position(10, 10),
            },
        ];
        f.created_field_instances = BTreeSet::from([1, 2]);
        f.event = Event::FieldEvaluate;
        f.now_ms = 200;
        assert_eq!(
            plan("owned_field_buff", &p, &f, &mut |_, _| panic!("field draw")).unwrap(),
            NativeDelayedPlan::FieldBonus {
                bonus_percent: 10,
                next_evaluation_ms: 1200
            }
        );
        f.event = Event::FieldExpiry;
        f.now_ms = 5100;
        f.current = None;
        assert_eq!(
            plan("owned_field_buff", &p, &f, &mut |_, _| panic!("field draw")).unwrap(),
            NativeDelayedPlan::RemoveField { instances: vec![1] }
        );
    }
    #[test]
    fn wrong_time_identity_draw_and_unbounded_facts_are_rejected() {
        let mut f = facts();
        let p = strike_parameters("death echo");
        assert!(plan("delayed_strike", &p, &f, &mut |_, b| b + 1).is_err());
        f.event = Event::DelayedOccurrence;
        f.now_ms = 1099;
        assert!(plan("delayed_strike", &p, &f, &mut |a, _| a).is_err());
        f.now_ms = 1100;
        f.current.as_mut().unwrap().identity.channel += 1;
        assert!(plan("delayed_strike", &p, &f, &mut |a, _| a).is_err());
        f.current = Some(actor());
        f.targets.push(f.targets[0].clone());
        assert!(plan("delayed_strike", &p, &f, &mut |a, _| a).is_err());
        let mut f = facts();
        f.cast_ms = u64::MAX;
        f.now_ms = u64::MAX;
        assert!(plan("delayed_strike", &p, &f, &mut |a, _| a).is_err());
    }
}
