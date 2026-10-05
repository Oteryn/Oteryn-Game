//! Pure plans for qualified wheel combat, avatar, monster AI, healing and shield candidates.
//! A plan is read-only intent. The native profile reader must qualify the exact authoring
//! snapshot first; this module neither grants runtime admission nor mutates actors/worlds.

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
pub(crate) enum Direction {
    North,
    East,
    South,
    West,
    NorthWest,
    NorthEast,
    SouthEast,
    SouthWest,
}
impl Direction {
    fn key(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::East => "east",
            Self::South => "south",
            Self::West => "west",
            Self::NorthWest => "northwest",
            Self::NorthEast => "northeast",
            Self::SouthEast => "southeast",
            Self::SouthWest => "southwest",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Player,
    PlayerSummon,
    MasterlessMonster,
    MonsterSummon,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeTargetFact {
    pub(crate) id: u64,
    pub(crate) position: TilePosition,
    pub(crate) kind: TargetKind,
    pub(crate) name: String,
    pub(crate) reward_boss: bool,
    pub(crate) preferred_distance: u32,
    pub(crate) health: u32,
    pub(crate) maximum_health: u32,
    /// Includes tile and actor legality, alive/removed checks and the world's permissions.
    pub(crate) legal: bool,
}
#[derive(Debug, Clone)]
pub(crate) struct NativeCombatFacts {
    pub(crate) now_ms: u64,
    pub(crate) stage: u8,
    pub(crate) inputs: FormulaInputs,
    pub(crate) maximum_mana: u32,
    pub(crate) caster: u64,
    pub(crate) caster_position: TilePosition,
    /// World-resolved combat matrix origin (source toPos), not inferred from actor state.
    pub(crate) area_origin: TilePosition,
    /// Qualified source getNextPosition(direction_to_caster, toPos) sight origin.
    pub(crate) area_sight_origin: TilePosition,
    pub(crate) direction: Direction,
    pub(crate) explicit_target: Option<u64>,
    pub(crate) attacked_target: Option<u64>,
    /// Canonical source spectator order; equal-distance ties preserve this order.
    pub(crate) targets: Vec<NativeTargetFact>,
    /// Ordered world-checked visibility pairs; absence means blocked, not unknown/allowed.
    pub(crate) clear_sight: BTreeSet<(TilePosition, TilePosition)>,
    pub(crate) floor_change_tiles: BTreeSet<TilePosition>,
    pub(crate) elemental_stance: ElementalStance,
    /// Optional incoming hit for the avatar reduction demonstration; no health mutation.
    pub(crate) incoming_hit: Option<i64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ElementalStance {
    None,
    Thunder,
    Flames,
    Decay,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Element {
    Physical,
    Energy,
    Fire,
    Death,
    Ice,
    Earth,
    Healing,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MagnitudePlan {
    pub(crate) target: u64,
    pub(crate) magnitude: i64,
    pub(crate) bonus_percent: u32,
    pub(crate) side_percent: Option<u32>,
    pub(crate) delay_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AreaPlan {
    pub(crate) origin: TilePosition,
    pub(crate) central_tiles: BTreeSet<TilePosition>,
    /// Unfiltered active matrix positions, used by independent side-beam geometry.
    pub(crate) central_geometry_tiles: BTreeSet<TilePosition>,
    pub(crate) side_tiles: BTreeSet<TilePosition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CombatPlan {
    pub(crate) element: Element,
    pub(crate) area: Option<AreaPlan>,
    pub(crate) hits: Vec<MagnitudePlan>,
    pub(crate) cooldown_ms: Option<u64>,
    pub(crate) shared_cooldown: Option<(String, u64)>,
    pub(crate) reduce_all_spell_cooldowns_ms: u64,
    pub(crate) block_armor: bool,
    pub(crate) use_weapon_charges: bool,
    pub(crate) weapon_missile: bool,
    pub(crate) dispel_paralysis_targets: Vec<u64>,
    pub(crate) resolve_critical_and_fatal_once: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AvatarPlan {
    pub(crate) expires_ms: u64,
    pub(crate) outfit_look_type: u32,
    pub(crate) incoming_reduction_percent: u32,
    pub(crate) incoming_after_reduction: Option<i64>,
    pub(crate) critical_chance_percent: u32,
    pub(crate) critical_extra_percentage_points: u32,
    pub(crate) cooldown_ms: u64,
    pub(crate) refresh_stats_on_apply_and_expiry: bool,
    pub(crate) replace_deadline: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AiOverridePlan {
    pub(crate) target: u64,
    pub(crate) delay_ms: u64,
    pub(crate) forced_distance: Option<u32>,
    pub(crate) forced_distance_expires_ms: Option<u64>,
    pub(crate) challenged_to: Option<u64>,
    pub(crate) challenge_expires_ms: Option<u64>,
    pub(crate) replace_deadline: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeCombatPlan {
    Combat(CombatPlan),
    Avatar(AvatarPlan),
    MonsterAi {
        overrides: Vec<AiOverridePlan>,
        cooldown_ms: Option<u64>,
    },
    MassSpiritMend {
        heals: Vec<MagnitudePlan>,
        dispel_paralysis_targets: Vec<u64>,
        area: AreaPlan,
        cooldown_ms: u64,
        harmony_spender: bool,
    },
    ManaShield {
        capacity: u32,
        expires_ms: u64,
        replace_capacity_and_deadline: bool,
        apply_before_presentation: bool,
    },
}

fn fail<T>(s: impl Into<String>) -> Result<T, Error> {
    Err(Error(s.into()))
}
fn field<'a>(v: &'a Value, k: &str) -> Result<&'a Value, Error> {
    v.get(k).ok_or_else(|| Error(format!("missing {k}")))
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str, Error> {
    field(v, k)?
        .as_str()
        .ok_or_else(|| Error(format!("invalid {k}")))
}
fn stage_value(v: &Value, stage: u8) -> Result<&Value, Error> {
    if v.is_object() {
        v.get(stage.to_string())
            .ok_or_else(|| Error("stage outside authored table".into()))
    } else {
        Ok(v)
    }
}
fn number(v: &Value, stage: u8) -> Result<u64, Error> {
    stage_value(v, stage)?
        .as_u64()
        .ok_or_else(|| Error("invalid nonnegative integer".into()))
}
fn n(v: &Value, k: &str, stage: u8) -> Result<u64, Error> {
    number(field(v, k)?, stage)
}
fn u32n(v: &Value, k: &str, stage: u8) -> Result<u32, Error> {
    u32::try_from(n(v, k, stage)?).map_err(|_| Error("integer overflow".into()))
}
fn deadline(f: &NativeCombatFacts, duration: u64) -> Result<u64, Error> {
    f.now_ms
        .checked_add(duration)
        .ok_or_else(|| Error("deadline overflow".into()))
}
fn gate(p: &Value, f: &NativeCombatFacts) -> Result<(), Error> {
    if f.stage > 3 {
        return fail("stage outside domain");
    }
    if let Some(wheel) = p.get("wheel") {
        if u64::from(f.stage) > n(wheel, "stage_max", 0)? {
            return fail("stage outside authored wheel domain");
        }
        if f.stage == 0 && text(wheel, "stage_zero")? == "reject_before_costs" {
            return fail("wheel stage locked");
        }
    }
    let mut ids = BTreeSet::new();
    for target in &f.targets {
        if !ids.insert(target.id) {
            return fail("duplicate world target identity");
        }
        if target.maximum_health == 0 || target.health > target.maximum_health {
            return fail("health exceeds maximum");
        }
    }
    if f.area_origin.floor != f.caster_position.floor
        || f.area_sight_origin.floor != f.caster_position.floor
    {
        return fail("cross-floor area origin");
    }
    Ok(())
}
fn sight(f: &NativeCombatFacts, a: TilePosition, b: TilePosition) -> bool {
    a == b || f.clear_sight.contains(&(a, b))
}
fn within(a: TilePosition, b: TilePosition, r: u64) -> bool {
    a.floor == b.floor && u64::from(a.x.abs_diff(b.x)) <= r && u64::from(a.y.abs_diff(b.y)) <= r
}
fn distance(a: TilePosition, b: TilePosition) -> u128 {
    let dx = u128::from(a.x.abs_diff(b.x));
    let dy = u128::from(a.y.abs_diff(b.y));
    dx * dx + dy * dy
}
fn formula(v: &Value, f: &NativeCombatFacts) -> Result<(i64, i64), Error> {
    let expression =
        |k| super::authoring::expression(field(v, k)?, 1).map_err(|e| Error(e.to_string()));
    Formula {
        minimum: expression("minimum")?,
        maximum: expression("maximum")?,
    }
    .bounds(&f.inputs)
    .map_err(|e| Error(e.to_string()))
}
fn roll(bounds: (i64, i64), draw: &mut dyn FnMut(i64, i64) -> i64) -> Result<i64, Error> {
    let value = draw(bounds.0, bounds.1);
    if value < bounds.0 || value > bounds.1 {
        fail("damage draw outside qualified bounds")
    } else {
        Ok(value)
    }
}
fn rotate(dx: i32, dy: i32, turn: u8) -> (i32, i32) {
    match turn {
        0 => (dx, dy),
        1 => (-dy, dx),
        2 => (-dx, -dy),
        _ => (dy, -dx),
    }
}
fn tile(origin: TilePosition, dx: i32, dy: i32) -> Result<TilePosition, Error> {
    Ok(TilePosition {
        x: origin
            .x
            .checked_add(dx)
            .ok_or_else(|| Error("tile overflow".into()))?,
        y: origin
            .y
            .checked_add(dy)
            .ok_or_else(|| Error("tile overflow".into()))?,
        floor: origin.floor,
    })
}
fn area(a: &Value, f: &NativeCombatFacts) -> Result<AreaPlan, Error> {
    let directional = field(a, "directional")?
        .as_bool()
        .ok_or_else(|| Error("invalid direction flag".into()))?;
    let diagonal = a.get("diagonal").filter(|v| v.is_array());
    let (matrix, turn) = if !directional {
        (field(a, "orthogonal")?, 0)
    } else {
        match (f.direction, diagonal) {
            (Direction::NorthWest, Some(m)) => (m, 0),
            (Direction::NorthEast, Some(m)) => (m, 1),
            (Direction::SouthEast, Some(m)) => (m, 2),
            (Direction::SouthWest, Some(m)) => (m, 3),
            (Direction::North, _) => (field(a, "orthogonal")?, 0),
            (Direction::South, _) => (field(a, "orthogonal")?, 2),
            (Direction::West | Direction::NorthWest | Direction::SouthWest, _) => {
                (field(a, "orthogonal")?, 3)
            }
            _ => (field(a, "orthogonal")?, 1),
        }
    };
    let rows = matrix
        .as_array()
        .ok_or_else(|| Error("invalid area matrix".into()))?;
    if rows.is_empty() || rows.len() > 64 {
        return fail("area rows outside bound");
    }
    let mut origin = None;
    let mut active = Vec::new();
    let mut width = None;
    for (y, row) in rows.iter().enumerate() {
        let cells = row
            .as_array()
            .ok_or_else(|| Error("invalid area row".into()))?;
        if cells.is_empty() || cells.len() > 64 || width.is_some_and(|w| w != cells.len()) {
            return fail("ragged/oversized area");
        }
        width = Some(cells.len());
        for (x, cell) in cells.iter().enumerate() {
            let value = cell
                .as_u64()
                .ok_or_else(|| Error("invalid area cell".into()))?;
            if value > 3 {
                return fail("unknown area encoding");
            }
            if value >= 2 && origin.replace((x as i32, y as i32)).is_some() {
                return fail("multiple area origins");
            }
            if value == 1 || value == 3 {
                active.push((x as i32, y as i32));
            }
        }
    }
    let (cx, cy) = origin.ok_or_else(|| Error("missing area origin".into()))?;
    let mut central = BTreeSet::new();
    let mut geometry = BTreeSet::new();
    for (x, y) in active {
        let (dx, dy) = rotate(x - cx, y - cy, turn);
        let position = tile(f.area_origin, dx, dy)?;
        geometry.insert(position);
        if !f.floor_change_tiles.contains(&position) && sight(f, f.area_sight_origin, position) {
            central.insert(position);
        }
    }
    Ok(AreaPlan {
        origin: f.area_origin,
        central_tiles: central,
        central_geometry_tiles: geometry,
        side_tiles: BTreeSet::new(),
    })
}
fn boss_allowed(p: &Value, target: &NativeTargetFact) -> Result<bool, Error> {
    if matches!(target.kind, TargetKind::Player | TargetKind::PlayerSummon) {
        return Ok(true);
    }
    if !matches!(
        target.kind,
        TargetKind::MasterlessMonster | TargetKind::MonsterSummon
    ) {
        return Ok(false);
    }
    let names = field(p, "monster_name_casefold_allowlist")?
        .as_array()
        .ok_or_else(|| Error("invalid boss allowlist".into()))?;
    Ok(names.iter().any(|name| {
        name.as_str()
            .is_some_and(|name| name == target.name.to_lowercase())
    }))
}
fn chain<'a>(
    spec: &Value,
    p: &Value,
    f: &'a NativeCombatFacts,
    eligible: impl Fn(&NativeTargetFact) -> bool,
) -> Result<Vec<&'a NativeTargetFact>, Error> {
    let r = n(spec, "jump_radius", f.stage)?;
    let first_range = p
        .get("first_target_range")
        .map(|v| number(v, f.stage))
        .transpose()?
        .unwrap_or(r);
    let valid = |t: &&NativeTargetFact| {
        t.id != f.caster
            && t.legal
            && eligible(t)
            && within(f.caster_position, t.position, first_range)
            && sight(f, f.caster_position, t.position)
    };
    let first_mode = text(spec, "first_target")?;
    let first = if first_mode == "explicit_target" {
        f.targets
            .iter()
            .filter(valid)
            .find(|t| Some(t.id) == f.explicit_target)
    } else {
        let attacked = f
            .targets
            .iter()
            .filter(valid)
            .find(|t| Some(t.id) == f.attacked_target);
        attacked.or_else(|| {
            f.targets
                .iter()
                .filter(valid)
                .min_by_key(|t| distance(f.caster_position, t.position))
        })
    };
    let Some(first) = first else {
        return fail("no first chain target");
    };
    let count = n(spec, "further_targets", f.stage)?;
    if count > 16 {
        return fail("chain length outside bound");
    }
    let mut selected = vec![first];
    let mut ids = BTreeSet::from([f.caster, first.id]);
    for _ in 0..count {
        let last = selected
            .last()
            .ok_or_else(|| Error("missing previous chain target".into()))?;
        let next = f
            .targets
            .iter()
            .filter(|t| {
                !ids.contains(&t.id)
                    && t.legal
                    && eligible(t)
                    && within(last.position, t.position, r)
                    && sight(f, last.position, t.position)
            })
            .min_by_key(|t| distance(last.position, t.position));
        if let Some(next) = next {
            ids.insert(next.id);
            selected.push(next);
        } else {
            break;
        }
    }
    Ok(selected)
}
fn health_bonus(p: &Value, f: &NativeCombatFacts, t: &NativeTargetFact) -> Result<u32, Error> {
    let Some(bonus) = p.get("health_bonus") else {
        return Ok(0);
    };
    if t.maximum_health == 0 {
        return fail("zero maximum health for threshold");
    }
    let percent = (100.0 * f64::from(t.health) / f64::from(t.maximum_health)).round() as u64;
    let threshold = n(bonus, "threshold", f.stage)?;
    let enabled = match text(bonus, "comparison")? {
        "greater_than" => percent > threshold,
        "less_than_or_equal" => percent <= threshold,
        _ => return fail("unknown health threshold"),
    };
    if enabled {
        u32n(bonus, "bonus_percent", f.stage)
    } else {
        Ok(0)
    }
}
fn element(p: &Value, f: &NativeCombatFacts) -> Result<Element, Error> {
    if p.get("elemental_stance").is_some() {
        return Ok(match f.elemental_stance {
            ElementalStance::Flames => Element::Fire,
            ElementalStance::Decay => Element::Death,
            _ => Element::Energy,
        });
    }
    Ok(match text(p, "element")? {
        "physical" => Element::Physical,
        "energy" => Element::Energy,
        "ice" => Element::Ice,
        "earth" => Element::Earth,
        "healing" => Element::Healing,
        _ => return fail("unknown combat element"),
    })
}

pub(crate) fn plan(
    key: &str,
    p: &Value,
    f: &NativeCombatFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeCombatPlan, Error> {
    gate(p, f)?;
    match key {
        "avatar_state" => {
            let reduction = u32n(p, "incoming_damage_reduction_percent", f.stage)?;
            let incoming = f
                .incoming_hit
                .map(|hit| {
                    if !(0..=i64::from(i32::MAX)).contains(&hit) {
                        return fail("incoming hit outside source integer domain");
                    }
                    let removed = ((hit as f64) * f64::from(reduction) / 100.0).ceil();
                    if !removed.is_finite() || removed > (i64::MAX as f64) {
                        return fail("reduction overflow");
                    }
                    Ok(hit - (removed as i64))
                })
                .transpose()?;
            Ok(NativeCombatPlan::Avatar(AvatarPlan {
                expires_ms: deadline(f, n(p, "duration_ms", f.stage)?)?,
                outfit_look_type: u32n(p, "outfit_look_type", f.stage)?,
                incoming_reduction_percent: reduction,
                incoming_after_reduction: incoming,
                critical_chance_percent: u32n(p, "critical_chance_percent", f.stage)?,
                critical_extra_percentage_points: u32n(
                    p,
                    "critical_extra_damage_percentage_points",
                    f.stage,
                )?,
                cooldown_ms: n(p, "cooldown_ms", f.stage)?,
                refresh_stats_on_apply_and_expiry: true,
                replace_deadline: true,
            }))
        }
        "mana_shield_capacity" => {
            // maximum_mana is State-owned, not a new Formula input. Substitute
            // that single typed snapshot fact, then use the existing evaluator.
            fn replace(v: &mut Value, mana: u32) {
                if v.get("var").and_then(Value::as_str) == Some("maximum_mana") {
                    *v = serde_json::json!({"const":mana.to_string()});
                } else if let Some(args) = v.get_mut("args").and_then(Value::as_array_mut) {
                    for arg in args {
                        replace(arg, mana);
                    }
                }
            }
            let mut expr = field(p, "capacity")?.clone();
            replace(&mut expr, f.maximum_mana);
            let raw = super::authoring::expression(&expr, 1)
                .map_err(|e| Error(e.to_string()))?
                .evaluate(&f.inputs)
                .map_err(|e| Error(e.to_string()))?;
            if !raw.is_finite()
                || raw < 0.0
                || raw > f64::from(f.maximum_mana)
                || raw.fract() != 0.0
            {
                return fail("invalid shield capacity");
            }
            Ok(NativeCombatPlan::ManaShield {
                capacity: raw as u32,
                expires_ms: deadline(f, n(p, "duration_ms", f.stage)?)?,
                replace_capacity_and_deadline: true,
                apply_before_presentation: true,
            })
        }
        "mass_spirit_mend" => {
            let a = area(field(p, "area")?, f)?;
            let routing = field(p, "target_routing")?;
            let bonus = u32n(p, "healing_bonus_percent", f.stage)?;
            let mut heals = Vec::new();
            let mut dispels = Vec::new();
            for target in &f.targets {
                if !target.legal || !a.central_tiles.contains(&target.position) {
                    continue;
                }
                dispels.push(target.id);
                if !boss_allowed(routing, target)? {
                    continue;
                }
                let formula_key = if target.id == f.caster {
                    "caster_formula"
                } else {
                    "others_formula"
                };
                heals.push(MagnitudePlan {
                    target: target.id,
                    magnitude: roll(formula(field(p, formula_key)?, f)?, draw)?,
                    bonus_percent: bonus,
                    side_percent: None,
                    delay_ms: 0,
                });
            }
            Ok(NativeCombatPlan::MassSpiritMend {
                heals,
                dispel_paralysis_targets: dispels,
                area: a,
                cooldown_ms: n(p, "cooldown_ms", f.stage)?,
                harmony_spender: false,
            })
        }
        "monster_ai_override" => {
            if let Some(refusal) = p.get("reward_boss_cast_refusal").filter(|v| !v.is_null()) {
                let radius = n(refusal, "dx_max", f.stage)?;
                if f.targets.iter().any(|t| {
                    t.reward_boss
                        && matches!(
                            t.kind,
                            TargetKind::MasterlessMonster
                                | TargetKind::MonsterSummon
                                | TargetKind::PlayerSummon
                        )
                        && within(f.caster_position, t.position, radius)
                }) {
                    return fail("nearby reward boss refuses cast");
                }
            }
            let filter = text(p, "target_filter")?;
            let eligible = |t: &NativeTargetFact| {
                matches!(t.kind, TargetKind::MasterlessMonster)
                    && (filter == "masterless_monsters" || !t.reward_boss)
                    && (filter != "masterless_ranged_non_reward_monsters"
                        || t.preferred_distance > 1)
            };
            let selected = if let Some(spec) = p.get("chain") {
                chain(spec, p, f, eligible)?
            } else {
                let a = area(field(p, "area")?, f)?;
                f.targets
                    .iter()
                    .filter(|t| t.legal && eligible(t) && a.central_tiles.contains(&t.position))
                    .collect()
            };
            let mut overrides = Vec::new();
            for (index, target) in selected.into_iter().enumerate() {
                let delay = if p.get("chain").is_some() {
                    index as u64 * 50
                } else {
                    0
                };
                let start = f
                    .now_ms
                    .checked_add(delay)
                    .ok_or_else(|| Error("chain delay overflow".into()))?;
                let expire = |key| {
                    p.get(key)
                        .map(|value| {
                            number(value, f.stage).and_then(|duration| {
                                start
                                    .checked_add(duration)
                                    .ok_or_else(|| Error("expiry overflow".into()))
                            })
                        })
                        .transpose()
                };
                overrides.push(AiOverridePlan {
                    target: target.id,
                    delay_ms: delay,
                    forced_distance: p
                        .get("force_distance")
                        .map(|v| {
                            number(v, f.stage).and_then(|v| {
                                u32::try_from(v).map_err(|_| Error("distance overflow".into()))
                            })
                        })
                        .transpose()?,
                    forced_distance_expires_ms: expire("forced_melee_ms")?,
                    challenged_to: p.get("challenge_target").map(|_| f.caster),
                    challenge_expires_ms: expire("challenge_ms")?,
                    replace_deadline: true,
                });
            }
            Ok(NativeCombatPlan::MonsterAi {
                overrides,
                cooldown_ms: p
                    .get("cooldown_ms")
                    .map(|v| number(v, f.stage))
                    .transpose()?,
            })
        }
        "wheel_combat" => combat(p, f, draw),
        _ => fail("unknown native combat key"),
    }
}

fn combat(
    p: &Value,
    f: &NativeCombatFacts,
    draw: &mut dyn FnMut(i64, i64) -> i64,
) -> Result<NativeCombatPlan, Error> {
    let healing = element(p, f)? == Element::Healing;
    let mut area_plan = None;
    let selected;
    if let Some(spec) = p.get("chain") {
        selected = chain(spec, p, f, |_| true)?;
    } else {
        let enhanced = u64::from(f.stage) >= n(p, "enhanced_area_from_stage", f.stage)?;
        let mut a = area(
            field(
                p,
                if enhanced {
                    "enhanced_area"
                } else {
                    "base_area"
                },
            )?,
            f,
        )?;
        if let Some(beam) = p.get("beam_mastery").filter(|_| f.stage > 0) {
            let direction = if field(
                p,
                if enhanced {
                    "enhanced_area"
                } else {
                    "base_area"
                },
            )?
            .get("diagonal")
            .is_some_and(Value::is_array)
            {
                f.direction
            } else {
                match f.direction {
                    Direction::NorthWest | Direction::SouthWest => Direction::West,
                    Direction::NorthEast | Direction::SouthEast => Direction::East,
                    d => d,
                }
            };
            let offsets = field(field(beam, "side_offsets_by_direction")?, direction.key())?
                .as_array()
                .ok_or_else(|| Error("invalid beam offsets".into()))?;
            for position in &a.central_geometry_tiles {
                for offset in offsets {
                    let pair = offset
                        .as_array()
                        .filter(|a| a.len() == 2)
                        .ok_or_else(|| Error("invalid side offset".into()))?;
                    let dx = pair[0]
                        .as_i64()
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or_else(|| Error("invalid side dx".into()))?;
                    let dy = pair[1]
                        .as_i64()
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or_else(|| Error("invalid side dy".into()))?;
                    let side = tile(*position, dx, dy)?;
                    if side != f.caster_position
                        && !a.central_geometry_tiles.contains(&side)
                        && !f.floor_change_tiles.contains(&side)
                        && sight(f, f.area_sight_origin, side)
                    {
                        a.side_tiles.insert(side);
                    }
                }
            }
        }
        selected = f
            .targets
            .iter()
            .filter(|t| {
                t.legal
                    && (healing || t.id != f.caster)
                    && (a.central_tiles.contains(&t.position) || a.side_tiles.contains(&t.position))
            })
            .collect();
        area_plan = Some(a);
    }
    let routing = p.get("target_routing");
    let bounds = formula(field(p, "formula")?, f)?;
    // Area direct damage rolls once for one combat execution. Chain targets
    // run independent doCombat/getCombatDamage calls. Callback-routed Mass Healing
    // independently draws per accepted target as its Lua callback does.
    let shared_roll = if !healing && p.get("chain").is_none() {
        Some(roll(bounds, draw)?)
    } else {
        None
    };
    let mut dispels = Vec::new();
    let mut hits = Vec::new();
    let central_count = area_plan
        .as_ref()
        .map(|a| {
            selected
                .iter()
                .filter(|t| a.central_tiles.contains(&t.position))
                .count()
        })
        .unwrap_or(0);
    let count = central_count.min(3) as u32;
    let beam = p.get("beam_mastery").filter(|_| f.stage > 0);
    let beam_bonus = beam
        .map(|v| u32n(v, "central_target_bonus_percent", f.stage))
        .transpose()?
        .unwrap_or(0)
        .checked_mul(count)
        .ok_or_else(|| Error("beam bonus overflow".into()))?;
    let ordinary_bonus = p
        .get("damage_or_heal_bonus_percent")
        .map(|v| number(v, f.stage))
        .transpose()?
        .unwrap_or(0);
    let ordinary_bonus =
        u32::try_from(ordinary_bonus).map_err(|_| Error("combat bonus overflow".into()))?;
    for (index, target) in selected.iter().enumerate() {
        if let Some(routing) = routing {
            dispels.push(target.id);
            if !boss_allowed(routing, target)? {
                continue;
            }
        }
        let side = area_plan
            .as_ref()
            .is_some_and(|a| a.side_tiles.contains(&target.position));
        let side_percent = if side {
            beam.map(|v| u32n(v, "side_damage_percent", f.stage))
                .transpose()?
        } else {
            None
        };
        let magnitude = if let Some(value) = shared_roll {
            value
        } else {
            roll(bounds, draw)?
        };
        let bonus = ordinary_bonus
            .checked_add(health_bonus(p, f, target)?)
            .and_then(|v| v.checked_add(if side { 0 } else { beam_bonus }))
            .ok_or_else(|| Error("target bonus overflow".into()))?;
        hits.push(MagnitudePlan {
            target: target.id,
            magnitude,
            bonus_percent: bonus,
            side_percent,
            delay_ms: if p.get("chain").is_some() {
                index as u64 * 50
            } else {
                0
            },
        });
    }
    let shared_cooldown = p
        .get("shared_spell_cooldown")
        .map(|value| {
            Ok((
                text(value, "peer")?.to_owned(),
                n(value, "duration_ms", f.stage)?,
            ))
        })
        .transpose()?;
    let reduced = beam
        .map(|v| {
            n(
                v,
                "reduce_all_spell_cooldowns_ms_per_counted_target",
                f.stage,
            )
        })
        .transpose()?
        .unwrap_or(0)
        .checked_mul(u64::from(count))
        .ok_or_else(|| Error("cooldown reduction overflow".into()))?;
    Ok(NativeCombatPlan::Combat(CombatPlan {
        element: element(p, f)?,
        area: area_plan,
        hits,
        cooldown_ms: p
            .get("cooldown_ms")
            .map(|v| number(v, f.stage))
            .transpose()?,
        shared_cooldown,
        reduce_all_spell_cooldowns_ms: reduced,
        block_armor: p
            .get("block_armor")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        use_weapon_charges: p
            .get("use_weapon_charges")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        weapon_missile: p.get("missile_selection").and_then(Value::as_str)
            == Some("equipped_weapon_type"),
        dispel_paralysis_targets: dispels,
        resolve_critical_and_fatal_once: beam.is_some(),
    }))
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;
    fn facts() -> NativeCombatFacts {
        NativeCombatFacts {
            now_ms: 100,
            stage: 1,
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
            maximum_mana: 1000,
            caster: 1,
            caster_position: TilePosition {
                x: 10,
                y: 10,
                floor: 7,
            },
            area_origin: TilePosition {
                x: 10,
                y: 10,
                floor: 7,
            },
            area_sight_origin: TilePosition {
                x: 10,
                y: 10,
                floor: 7,
            },
            direction: Direction::North,
            explicit_target: None,
            attacked_target: None,
            targets: vec![],
            clear_sight: BTreeSet::new(),
            floor_change_tiles: BTreeSet::new(),
            elemental_stance: ElementalStance::None,
            incoming_hit: Some(101),
        }
    }
    fn target(id: u64, x: i32, y: i32) -> NativeTargetFact {
        NativeTargetFact {
            id,
            position: TilePosition { x, y, floor: 7 },
            kind: TargetKind::MasterlessMonster,
            name: "ordinary monster".into(),
            reward_boss: false,
            preferred_distance: 4,
            health: 300,
            maximum_health: 1000,
            legal: true,
        }
    }
    const PROFILE_PARAMETERS: &str = include_str!(
        "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    );
    fn profile(name: &str) -> (String, Value) {
        let export: Value = serde_json::from_str(PROFILE_PARAMETERS).expect("canonical profiles");
        let profile = export["profiles"]
            .as_array()
            .expect("profile array")
            .iter()
            .find(|p| {
                p["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    && p["carrier"].as_str() == Some("instant")
            })
            .expect("qualified instant profile");
        let behavior = &profile["execution"]["native_behavior"];
        (
            behavior["key"].as_str().expect("native key").to_owned(),
            behavior["parameters"].clone(),
        )
    }
    fn visible_facts() -> NativeCombatFacts {
        let mut f = facts();
        f.targets = vec![target(2, 10, 9), target(3, 11, 9), target(4, 12, 9)];
        f.explicit_target = Some(2);
        f.attacked_target = Some(2);
        for y in -8..29 {
            for x in -8..29 {
                f.clear_sight
                    .insert((f.area_sight_origin, TilePosition { x, y, floor: 7 }));
                for target in &f.targets {
                    f.clear_sight
                        .insert((target.position, TilePosition { x, y, floor: 7 }));
                }
            }
        }
        f
    }
    #[test]
    fn all_twenty_canonical_profiles_plan_at_stages_zero_and_one() {
        let names = [
            "energy beam",
            "energy wave",
            "executioner's throw",
            "front sweep",
            "great energy beam",
            "ice burst",
            "mass healing",
            "strong ice wave",
            "terra burst",
            "avatar of balance",
            "avatar of light",
            "avatar of nature",
            "avatar of steel",
            "avatar of storm",
            "balanced brawl",
            "challenge",
            "chivalrous challenge",
            "divine dazzle",
            "mass spirit mend",
            "magic shield",
        ];
        let mut count = 0;
        for name in names {
            let (key, p) = profile(name);
            for stage in [0, 1] {
                let mut f = visible_facts();
                f.stage = stage;
                let before = f.targets.clone();
                let mut draws = 0;
                let result = plan(&key, &p, &f, &mut |low, _| {
                    draws += 1;
                    low
                });
                let locked = stage == 0
                    && (name.starts_with("avatar")
                        || matches!(name, "executioner's throw" | "ice burst" | "terra burst"));
                if locked {
                    assert!(result.is_err(), "{name}");
                    assert_eq!(draws, 0, "locked cast drew RNG: {name}");
                } else {
                    assert!(result.is_ok(), "{name}: {result:?}");
                }
                assert_eq!(f.targets, before, "planner mutated source facts");
            }
            count += 1;
        }
        assert_eq!(count, 20);
    }
    #[test]
    fn canonical_mass_mend_routes_self_players_summons_and_named_monsters_separately() {
        let (key, p) = profile("mass spirit mend");
        let mut f = facts();
        f.stage = 0;
        let mut caster = target(1, 10, 10);
        caster.kind = TargetKind::Player;
        let mut summon = target(2, 10, 10);
        summon.kind = TargetKind::PlayerSummon;
        let mut boss = target(3, 10, 10);
        boss.name = "Leiden".into();
        boss.reward_boss = true;
        let excluded = target(4, 10, 10);
        f.targets = vec![caster, summon, boss, excluded];
        let NativeCombatPlan::MassSpiritMend {
            heals,
            dispel_paralysis_targets,
            cooldown_ms,
            harmony_spender,
            ..
        } = plan(&key, &p, &f, &mut |low, _| low).expect("canonical mend")
        else {
            panic!("wrong family")
        };
        assert_eq!(
            heals
                .iter()
                .map(|p| (p.target, p.magnitude, p.bonus_percent))
                .collect::<Vec<_>>(),
            [(1, 980, 0), (2, 3252, 0), (3, 3252, 0)]
        );
        assert_eq!(dispel_paralysis_targets, [1, 2, 3, 4]);
        assert_eq!(cooldown_ms, 12000);
        assert!(!harmony_spender);
    }
    #[test]
    fn malformed_owned_target_facts_reject_before_consuming_rng() {
        let (key, p) = profile("executioner's throw");
        let mut f = visible_facts();
        f.targets[0].maximum_health = 0;
        f.targets[0].health = 0;
        let mut draws = 0;
        assert!(
            plan(&key, &p, &f, &mut |low, _| {
                draws += 1;
                low
            })
            .is_err()
        );
        assert_eq!(draws, 0);
        f = visible_facts();
        f.targets.push(f.targets[0].clone());
        assert!(
            plan(&key, &p, &f, &mut |low, _| {
                draws += 1;
                low
            })
            .is_err()
        );
        assert_eq!(draws, 0);
    }
    #[test]
    fn canonical_executioner_chain_draws_independently_per_bounce() {
        let (key, p) = profile("executioner's throw");
        let f = visible_facts();
        let mut draws = 0;
        let NativeCombatPlan::Combat(plan) = plan(&key, &p, &f, &mut |low, high| {
            let value = low + draws;
            draws += 1;
            assert!(value <= high);
            value
        })
        .expect("canonical executioner") else {
            panic!("wrong family")
        };
        assert_eq!(draws, 3);
        assert_eq!(
            plan.hits
                .iter()
                .map(|hit| hit.magnitude)
                .collect::<Vec<_>>(),
            [503, 504, 505]
        );
        assert_eq!(
            plan.hits.iter().map(|hit| hit.delay_ms).collect::<Vec<_>>(),
            [0, 50, 100]
        );
    }
    #[test]
    fn canonical_beam_stages_preserve_side_damage_and_cooldown_count_cap() {
        let (key, p) = profile("energy beam");
        let mut f = visible_facts();
        // Three central creatures and a fourth target on an independent side tile.
        f.targets = vec![
            target(2, 10, 9),
            target(3, 10, 8),
            target(4, 10, 7),
            target(5, 9, 9),
        ];
        for stage in [1, 2, 3] {
            f.stage = stage;
            let NativeCombatPlan::Combat(plan) =
                plan(&key, &p, &f, &mut |low, _| low).expect("canonical beam")
            else {
                panic!("wrong family")
            };
            assert_eq!(plan.reduce_all_spell_cooldowns_ms, 3000);
            assert_eq!(
                plan.hits
                    .iter()
                    .find(|hit| hit.target == 5)
                    .expect("side target")
                    .side_percent,
                Some(match stage {
                    1 => 25,
                    2 => 40,
                    _ => 70,
                })
            );
            assert_eq!(
                plan.hits
                    .iter()
                    .find(|hit| hit.target == 2)
                    .expect("central target")
                    .bonus_percent,
                match stage {
                    1 => 30,
                    2 => 36,
                    _ => 42,
                }
            );
        }
    }
    #[test]
    fn avatar_ceil_reduction_and_deadline_no_mutation() {
        let f = facts();
        let p = json!({"duration_ms":15000,"outfit_look_type":1593,
            "incoming_damage_reduction_percent":{"1":5},"critical_chance_percent":100,
            "critical_extra_damage_percentage_points":{"1":5},"cooldown_ms":{"1":7200000}});
        let plan = plan("avatar_state", &p, &f, &mut |a, _| a).unwrap();
        let NativeCombatPlan::Avatar(p) = plan else {
            panic!("wrong plan")
        };
        assert_eq!(p.expires_ms, 15100);
        assert_eq!(p.incoming_after_reduction, Some(95));
        assert_eq!(f.incoming_hit, Some(101));
    }
    #[test]
    fn chain_attack_priority_source_ties_and_no_repeats() {
        let mut f = facts();
        f.targets = vec![target(2, 11, 10), target(3, 12, 10), target(4, 11, 11)];
        f.attacked_target = Some(3);
        for from in [
            f.caster_position,
            f.targets[0].position,
            f.targets[1].position,
            f.targets[2].position,
        ] {
            for t in &f.targets {
                f.clear_sight.insert((from, t.position));
            }
        }
        let p = json!({"first_target":"attacked_valid_target_else_nearest_valid_target","jump_radius":7,"further_targets":2});
        let selected = chain(&p, &json!({}), &f, |_| true).unwrap();
        assert_eq!(selected.iter().map(|t| t.id).collect::<Vec<_>>(), [3, 2, 4]);
        f.attacked_target = None;
        assert_eq!(chain(&p, &json!({}), &f, |_| true).unwrap()[0].id, 2);
    }
    #[test]
    fn rounded_health_threshold_is_attributed_source_behavior() {
        let f = facts();
        let p = json!({"health_bonus":{"comparison":"less_than_or_equal","threshold":30,"bonus_percent":{"1":100}}});
        let mut t = target(2, 10, 9);
        t.health = 304;
        assert_eq!(health_bonus(&p, &f, &t).unwrap(), 100);
        t.health = 305;
        assert_eq!(health_bonus(&p, &f, &t).unwrap(), 0);
        t.maximum_health = 0;
        assert!(health_bonus(&p, &f, &t).is_err());
    }
    #[test]
    fn shield_state_maximum_mana_and_world_curve_are_independent_inputs() {
        let f = facts();
        let p = json!({"capacity":{"op":"min","args":[{"var":"maximum_mana"},{"const":"10300"}]},"duration_ms":180000});
        assert_eq!(
            plan("mana_shield_capacity", &p, &f, &mut |a, _| a).unwrap(),
            NativeCombatPlan::ManaShield {
                capacity: 1000,
                expires_ms: 180100,
                replace_capacity_and_deadline: true,
                apply_before_presentation: true
            }
        );
        assert_eq!(super::super::formula::level_base_damage_healing(1200), 214);
        assert!(roll((10, 20), &mut |_, _| 21).is_err());
    }
    #[test]
    fn nearby_reward_boss_cast_gate_counts_monsters_even_if_player_owned() {
        let mut f = facts();
        let mut boss = target(2, 11, 10);
        boss.kind = TargetKind::PlayerSummon;
        boss.reward_boss = true;
        f.targets.push(boss);
        let p = json!({"reward_boss_cast_refusal":{"dx_max":11}});
        assert_eq!(
            plan("monster_ai_override", &p, &f, &mut |a, _| a)
                .unwrap_err()
                .0,
            "nearby reward boss refuses cast"
        );
    }
    #[test]
    fn area_origin_two_excludes_caster_and_visibility_fails_closed() {
        let mut f = facts();
        let a = json!({"directional":false,"orthogonal":[[1],[2]],"diagonal":null});
        assert!(area(&a, &f).unwrap().central_tiles.is_empty());
        let hit = TilePosition {
            x: 10,
            y: 9,
            floor: 7,
        };
        f.clear_sight.insert((f.caster_position, hit));
        assert_eq!(area(&a, &f).unwrap().central_tiles, BTreeSet::from([hit]));
        f.floor_change_tiles.insert(hit);
        assert!(area(&a, &f).unwrap().central_tiles.is_empty());
    }
}
