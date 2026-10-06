//! Pure item-operation plans for qualified native spell parameters.
//!
//! Snapshots contain Game-owned item identities and protection flags. These
//! helpers neither mutate a world nor authorize a commit. The caller resolves
//! the returned exact references and commits inventory, tile and outfit changes
//! atomically with the cast's resource/charge debit.

use std::collections::BTreeSet;
use std::fmt;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NativeItemRef {
    pub(crate) key: String,
    pub(crate) revision: String,
}

impl NativeItemRef {
    /// Recognize the two explicit Item identity forms; no identity is allocated.
    pub(crate) fn numeric_id(&self) -> Result<u32, Error> {
        let suffix = self
            .key
            .strip_prefix("candidate:item/")
            .or_else(|| self.key.strip_prefix("oteryn:item.tibia.i"))
            .ok_or(Error::InvalidReference)?;
        if suffix.is_empty()
            || !suffix.bytes().all(|c| c.is_ascii_digit())
            || suffix.starts_with('0')
            || self.revision.is_empty()
        {
            return Err(Error::InvalidReference);
        }
        suffix
            .parse::<u32>()
            .ok()
            .filter(|id| *id > 0)
            .ok_or(Error::InvalidReference)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectPosition {
    Caster,
    TargetTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemPresentation {
    pub(crate) asset_binding: String,
    pub(crate) position: EffectPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemRefusal {
    MissingTarget,
    CreatureTarget,
    ImmovableItem,
    ProtectionZone,
    NoListedField,
    MissingTile,
    FloorChange,
    CreatureOnTile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidParameter(&'static str),
    InvalidReference,
    RandomOutOfRange,
    Refused {
        reason: ItemRefusal,
        message: Option<String>,
        presentation: Option<ItemPresentation>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemGrantOverflow {
    DropOnCasterTile,
}

pub(crate) fn random_item_grant_overflow(parameters: &Value) -> Result<ItemGrantOverflow, Error> {
    expect_string(parameters, "overflow", "drop_on_caster_tile")?;
    Ok(ItemGrantOverflow::DropOnCasterTile)
}

/// Draw bounds are inclusive and preserve the authored source order: the 0/1
/// extra roll, optional pool selection, then the guaranteed pool selection.
pub(crate) fn plan_random_item_grant(
    parameters: &Value,
    draw: &mut dyn FnMut(u32, u32) -> u32,
) -> Result<Vec<NativeItemRef>, Error> {
    expect_u32(parameters, "guaranteed", 1)?;
    expect_u32(parameters, "extra", 1)?;
    expect_u32(parameters, "extra_chance_percent", 50)?;
    expect_string(parameters, "selection", "uniform_independent")?;
    expect_string(
        parameters,
        "draw_order",
        "extra_chance_then_optional_then_guaranteed",
    )?;
    random_item_grant_overflow(parameters)?;
    expect_bool(parameters, "always_succeeds", true)?;
    string(parameters, "effect_asset_binding")?;
    let pool = item_refs(parameters, "pool")?;
    if pool.len() != 7 {
        return Err(Error::InvalidParameter("pool"));
    }
    let extra = checked_draw(draw, 0, 1)? == 1;
    let mut items = Vec::with_capacity(if extra { 2 } else { 1 });
    if extra {
        let index = checked_draw(draw, 1, 7)? as usize - 1;
        items.push(pool[index].clone());
    }
    let index = checked_draw(draw, 1, 7)? as usize - 1;
    items.push(pool[index].clone());
    Ok(items)
}

fn checked_draw(draw: &mut dyn FnMut(u32, u32) -> u32, low: u32, high: u32) -> Result<u32, Error> {
    let value = draw(low, high);
    if value < low || value > high {
        Err(Error::RandomOutOfRange)
    } else {
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorldItemFacts {
    pub(crate) instance_key: String,
    pub(crate) item: NativeItemRef,
    pub(crate) movable: bool,
    pub(crate) script_tagged: bool,
    pub(crate) action_tagged: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemTarget {
    #[default]
    Tile,
    /// The owner resolves the client slot address. An existing container takes
    /// precedence even when its slot is empty; equipment is the fallback only
    /// when that container does not exist.
    ContainerOrEquipment,
    Creature,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ItemWorldSnapshot {
    pub(crate) caster_in_pz: bool,
    pub(crate) tile_exists: bool,
    pub(crate) target: ItemTarget,
    pub(crate) top_item: Option<WorldItemFacts>,
    /// Source/world item iteration order, including protected items.
    pub(crate) tile_items: Vec<WorldItemFacts>,
    /// First magic field, selected before membership is checked. A later listed
    /// field cannot replace an unlisted first field.
    pub(crate) first_magic_field: Option<WorldItemFacts>,
    pub(crate) container_exists: bool,
    pub(crate) container_slot_item: Option<WorldItemFacts>,
    pub(crate) equipment_slot_item: Option<WorldItemFacts>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedItemSource {
    TileTopItem,
    ContainerSlot,
    EquipmentSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemOperation {
    MimicItem {
        item: NativeItemRef,
        duration_ms: u32,
        selected_from: SelectedItemSource,
    },
    Disintegrate {
        remove_instances: Vec<String>,
        visited_items: u32,
    },
    RemoveField {
        remove_instance: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ItemOperationPlan {
    pub(crate) operation: ItemOperation,
    pub(crate) success_presentation: ItemPresentation,
    pub(crate) consume_charge: bool,
}

pub(crate) fn resolve_tile_item_operation(
    parameters: &Value,
    snapshot: &ItemWorldSnapshot,
) -> Result<ItemOperationPlan, Error> {
    match string(parameters, "operation")? {
        "mimic_item" => mimic_item(parameters, snapshot),
        "disintegrate" => disintegrate(parameters, snapshot),
        "remove_field" => remove_field(parameters, snapshot),
        _ => Err(Error::InvalidParameter("operation")),
    }
}

fn mimic_item(p: &Value, snapshot: &ItemWorldSnapshot) -> Result<ItemOperationPlan, Error> {
    expect_sources(p, &["tile_top_item", "container_slot", "equipment_slot"])?;
    expect_bool(p, "require_movable", true)?;
    expect_bool(p, "reject_creature_target", true)?;
    let duration_ms = positive_u32(p, "duration_ms")?;
    let success = presentation(p, "success_effect_asset_binding", EffectPosition::Caster)?;
    let failure = presentation(p, "failure_effect_asset_binding", EffectPosition::Caster)?;
    let message = string(p, "failure_message")?.to_owned();
    let refusal = |reason| Error::Refused {
        reason,
        message: Some(message.clone()),
        presentation: Some(failure.clone()),
    };
    let (item, selected_from) = match snapshot.target {
        ItemTarget::Creature => return Err(refusal(ItemRefusal::CreatureTarget)),
        ItemTarget::Tile => (
            snapshot.top_item.as_ref().filter(|_| snapshot.tile_exists),
            SelectedItemSource::TileTopItem,
        ),
        ItemTarget::ContainerOrEquipment if snapshot.container_exists => (
            snapshot.container_slot_item.as_ref(),
            SelectedItemSource::ContainerSlot,
        ),
        ItemTarget::ContainerOrEquipment => (
            snapshot.equipment_slot_item.as_ref(),
            SelectedItemSource::EquipmentSlot,
        ),
    };
    let item = item.ok_or_else(|| refusal(ItemRefusal::MissingTarget))?;
    validate_world_item(item)?;
    if !item.movable {
        return Err(refusal(ItemRefusal::ImmovableItem));
    }
    Ok(ItemOperationPlan {
        operation: ItemOperation::MimicItem {
            item: item.item.clone(),
            duration_ms,
            selected_from,
        },
        success_presentation: success,
        consume_charge: true,
    })
}

fn disintegrate(p: &Value, snapshot: &ItemWorldSnapshot) -> Result<ItemOperationPlan, Error> {
    expect_sources(p, &["tile_items"])?;
    expect_bool(p, "require_movable", true)?;
    expect_u32(p, "max_items", 500)?;
    expect_string(p, "max_items_counts", "visited")?;
    for key in [
        "exclude_script_tagged",
        "exclude_action_tagged",
        "allow_in_pz",
        "empty_tile_succeeds",
        "missing_tile_succeeds",
    ] {
        expect_bool(p, key, true)?;
    }
    expect_bool(p, "aggressive", false)?;
    expect_bool(p, "send_cancel_on_success", false)?;
    let excluded: BTreeSet<_> = item_refs(p, "exclude_items")?.into_iter().collect();
    let success = presentation(
        p,
        "success_effect_asset_binding",
        EffectPosition::TargetTile,
    )?;
    let mut remove_instances = Vec::new();
    let mut instances = BTreeSet::new();
    let mut visited_items = 0;
    if snapshot.tile_exists {
        for item in snapshot.tile_items.iter().take(500) {
            validate_world_item(item)?;
            if !instances.insert(&item.instance_key) {
                return Err(Error::InvalidParameter("tile_items duplicate instance"));
            }
            visited_items += 1;
            if item.movable
                && !item.script_tagged
                && !item.action_tagged
                && !excluded.contains(&item.item)
            {
                remove_instances.push(item.instance_key.clone());
            }
        }
    }
    Ok(ItemOperationPlan {
        operation: ItemOperation::Disintegrate {
            remove_instances,
            visited_items,
        },
        success_presentation: success,
        consume_charge: true,
    })
}

fn remove_field(p: &Value, snapshot: &ItemWorldSnapshot) -> Result<ItemOperationPlan, Error> {
    expect_sources(p, &["first_magic_field"])?;
    expect_bool(p, "allow_in_pz", false)?;
    expect_string(p, "failure_effect_position", "caster")?;
    let fields: BTreeSet<_> = item_refs(p, "field_items")?.into_iter().collect();
    let success = presentation(
        p,
        "success_effect_asset_binding",
        EffectPosition::TargetTile,
    )?;
    let failure = presentation(p, "failure_effect_asset_binding", EffectPosition::Caster)?;
    let message = string(p, "failure_message")?.to_owned();
    let refusal = |reason| Error::Refused {
        reason,
        message: Some(message.clone()),
        presentation: Some(failure.clone()),
    };
    if snapshot.caster_in_pz {
        return Err(refusal(ItemRefusal::ProtectionZone));
    }
    if !snapshot.tile_exists || snapshot.target != ItemTarget::Tile {
        return Err(refusal(ItemRefusal::NoListedField));
    }
    let field = snapshot
        .first_magic_field
        .as_ref()
        .ok_or_else(|| refusal(ItemRefusal::NoListedField))?;
    validate_world_item(field)?;
    if !fields.contains(&field.item) {
        return Err(refusal(ItemRefusal::NoListedField));
    }
    Ok(ItemOperationPlan {
        operation: ItemOperation::RemoveField {
            remove_instance: field.instance_key.clone(),
        },
        success_presentation: success,
        consume_charge: true,
    })
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BarrierTileFacts {
    pub(crate) exists: bool,
    pub(crate) floor_change: bool,
    pub(crate) creature_on_tile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BarrierPlan {
    pub(crate) item: NativeItemRef,
    pub(crate) count: u32,
    pub(crate) duration_ms: u32,
    pub(crate) description: String,
    pub(crate) projectile_asset_binding: String,
}

pub(crate) fn plan_barrier(
    effect: &Value,
    tile: &BarrierTileFacts,
    optional_pvp: bool,
    sample_seconds: u32,
    caster_name: &str,
) -> Result<BarrierPlan, Error> {
    expect_string(effect, "operation", "create_item")?;
    expect_string(effect, "duration_selection", "uniform_integer_seconds")?;
    expect_string(effect, "safe_world_type", "optional_pvp")?;
    if effect.get("refuse_on")
        != Some(&serde_json::json!([
            "floor_change_tile",
            "creature_on_tile"
        ]))
    {
        return Err(Error::InvalidParameter("refuse_on"));
    }
    expect_string(effect, "description_template", "Casted by: {caster_name}")?;
    if caster_name.is_empty() {
        return Err(Error::InvalidParameter("caster_name"));
    }
    let normal = item_ref(effect.get("created_item").ok_or(Error::InvalidReference)?)?;
    let safe = item_ref(effect.get("pvp_safe_item").ok_or(Error::InvalidReference)?)?;
    let duration = effect
        .get("duration_range_ms")
        .ok_or(Error::InvalidParameter("duration_range_ms"))?;
    let minimum = positive_u32(duration, "minimum")?;
    let maximum = positive_u32(duration, "maximum")?;
    if minimum > maximum || !minimum.is_multiple_of(1000) || !maximum.is_multiple_of(1000) {
        return Err(Error::InvalidParameter("duration_range_ms"));
    }
    let duration_ms = sample_seconds
        .checked_mul(1000)
        .ok_or(Error::RandomOutOfRange)?;
    if duration_ms < minimum || duration_ms > maximum {
        return Err(Error::RandomOutOfRange);
    }
    let projectile = string(
        effect
            .get("presentation")
            .ok_or(Error::InvalidParameter("presentation"))?,
        "projectile_asset_binding",
    )?;
    if projectile != "canary.appearance:missile/energy" {
        return Err(Error::InvalidParameter("projectile_asset_binding"));
    }
    let refusal = |reason| Error::Refused {
        reason,
        message: None,
        presentation: None,
    };
    if !tile.exists {
        return Err(refusal(ItemRefusal::MissingTile));
    }
    if tile.floor_change {
        return Err(refusal(ItemRefusal::FloorChange));
    }
    if tile.creature_on_tile {
        return Err(refusal(ItemRefusal::CreatureOnTile));
    }
    Ok(BarrierPlan {
        item: if optional_pvp { safe } else { normal },
        count: 1,
        duration_ms,
        description: format!("Casted by: {caster_name}"),
        projectile_asset_binding: projectile.to_owned(),
    })
}

fn string<'a>(value: &'a Value, key: &'static str) -> Result<&'a str, Error> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(Error::InvalidParameter(key))
}

fn expect_string(value: &Value, key: &'static str, expected: &str) -> Result<(), Error> {
    if string(value, key)? == expected {
        Ok(())
    } else {
        Err(Error::InvalidParameter(key))
    }
}

fn positive_u32(value: &Value, key: &'static str) -> Result<u32, Error> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or(Error::InvalidParameter(key))
}

fn expect_u32(value: &Value, key: &'static str, expected: u32) -> Result<(), Error> {
    if value.get(key).and_then(Value::as_u64) == Some(u64::from(expected)) {
        Ok(())
    } else {
        Err(Error::InvalidParameter(key))
    }
}

fn expect_bool(value: &Value, key: &'static str, expected: bool) -> Result<(), Error> {
    if value.get(key).and_then(Value::as_bool) == Some(expected) {
        Ok(())
    } else {
        Err(Error::InvalidParameter(key))
    }
}

fn expect_sources(value: &Value, expected: &[&str]) -> Result<(), Error> {
    if value.get("source") == Some(&serde_json::json!(expected)) {
        Ok(())
    } else {
        Err(Error::InvalidParameter("source"))
    }
}

fn item_ref(value: &Value) -> Result<NativeItemRef, Error> {
    let object = value.as_object().ok_or(Error::InvalidReference)?;
    if object.len() != 3 || object.get("family").and_then(Value::as_str) != Some("Item") {
        return Err(Error::InvalidReference);
    }
    let reference = NativeItemRef {
        key: object
            .get("key")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidReference)?
            .to_owned(),
        revision: object
            .get("revision")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidReference)?
            .to_owned(),
    };
    reference.numeric_id()?;
    Ok(reference)
}

fn item_refs(value: &Value, key: &'static str) -> Result<Vec<NativeItemRef>, Error> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or(Error::InvalidParameter(key))?;
    let items: Vec<_> = array.iter().map(item_ref).collect::<Result<_, _>>()?;
    if items.iter().collect::<BTreeSet<_>>().len() != items.len() {
        return Err(Error::InvalidParameter(key));
    }
    Ok(items)
}

fn validate_world_item(item: &WorldItemFacts) -> Result<(), Error> {
    item.item.numeric_id()?;
    if item.instance_key.is_empty() {
        Err(Error::InvalidParameter("instance_key"))
    } else {
        Ok(())
    }
}

fn presentation(
    value: &Value,
    key: &'static str,
    position: EffectPosition,
) -> Result<ItemPresentation, Error> {
    Ok(ItemPresentation {
        asset_binding: string(value, key)?.to_owned(),
        position,
    })
}

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    fn reference(id: u32) -> Value {
        json!({"family":"Item","key":format!("candidate:item/{id}"),"revision":"spell-p2-r20"})
    }

    fn item(instance: &str, id: u32) -> WorldItemFacts {
        WorldItemFacts {
            instance_key: instance.to_owned(),
            item: item_ref(&reference(id)).unwrap(),
            movable: true,
            script_tagged: false,
            action_tagged: false,
        }
    }

    fn food() -> Value {
        json!({"pool":[reference(3577),reference(3582),reference(3592),reference(3585),reference(3600),reference(3601),reference(3607)],"guaranteed":1,"extra":1,"extra_chance_percent":50,"selection":"uniform_independent","overflow":"drop_on_caster_tile","draw_order":"extra_chance_then_optional_then_guaranteed","effect_asset_binding":"canary.appearance:effect/magic_green","always_succeeds":true})
    }

    fn mimic() -> Value {
        json!({"operation":"mimic_item","source":["tile_top_item","container_slot","equipment_slot"],"require_movable":true,"reject_creature_target":true,"duration_ms":200000,"success_effect_asset_binding":"canary.appearance:effect/magic_red","failure_effect_asset_binding":"canary.appearance:effect/poff","failure_message":"not_possible"})
    }

    fn disintegrate_parameters() -> Value {
        json!({"operation":"disintegrate","source":["tile_items"],"require_movable":true,"max_items":500,"max_items_counts":"visited","exclude_items":[reference(4240),reference(4241),reference(4242),reference(4243),reference(4246),reference(4247),reference(4248)],"exclude_script_tagged":true,"exclude_action_tagged":true,"aggressive":false,"allow_in_pz":true,"empty_tile_succeeds":true,"missing_tile_succeeds":true,"success_effect_asset_binding":"canary.appearance:effect/poff","send_cancel_on_success":false})
    }

    fn destroy() -> Value {
        json!({"operation":"remove_field","source":["first_magic_field"],"field_items":[reference(2118)],"allow_in_pz":false,"success_effect_asset_binding":"canary.appearance:effect/poff","failure_effect_asset_binding":"canary.appearance:effect/poff","failure_message":"not_possible","failure_effect_position":"caster"})
    }

    fn barrier() -> Value {
        json!({"operation":"create_item","created_item":reference(2128),"pvp_safe_item":reference(10181),"duration_range_ms":{"minimum":16000,"maximum":24000},"duration_selection":"uniform_integer_seconds","safe_world_type":"optional_pvp","refuse_on":["floor_change_tile","creature_on_tile"],"description_template":"Casted by: {caster_name}","presentation":{"projectile_asset_binding":"canary.appearance:missile/energy"}})
    }

    #[test]
    fn food_preserves_optional_first_draw_and_guaranteed_draw_order() {
        let mut calls = Vec::new();
        let mut values = [1, 7, 1].into_iter();
        let planned = plan_random_item_grant(&food(), &mut |lo, hi| {
            calls.push((lo, hi));
            values.next().unwrap()
        })
        .unwrap();
        assert_eq!(calls, vec![(0, 1), (1, 7), (1, 7)]);
        assert_eq!(
            planned
                .iter()
                .map(|r| r.numeric_id().unwrap())
                .collect::<Vec<_>>(),
            vec![3607, 3577]
        );
        assert_eq!(
            random_item_grant_overflow(&food()),
            Ok(ItemGrantOverflow::DropOnCasterTile)
        );
        let mut values = [0, 1].into_iter();
        assert_eq!(
            plan_random_item_grant(&food(), &mut |_, _| values.next().unwrap())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn invalid_food_reference_is_rejected_before_rng_consumption() {
        let mut p = food();
        p["pool"][0]["family"] = json!("Creature");
        let mut calls = 0;
        assert_eq!(
            plan_random_item_grant(&p, &mut |_, _| {
                calls += 1;
                0
            }),
            Err(Error::InvalidReference)
        );
        assert_eq!(calls, 0);
        assert_eq!(
            plan_random_item_grant(&food(), &mut |_, _| 8),
            Err(Error::RandomOutOfRange)
        );
    }

    #[test]
    fn exact_positive_references_reject_zero_negative_and_unrecognized_identity() {
        for key in [
            "candidate:item/0",
            "candidate:item/-1",
            "candidate:item/01",
            "candidate:item/4294967296",
            "invented:item/1",
        ] {
            let mut r = reference(1);
            r["key"] = json!(key);
            assert_eq!(item_ref(&r), Err(Error::InvalidReference));
        }
        let mut canonical = reference(2128);
        canonical["key"] = json!("oteryn:item.tibia.i2128");
        assert_eq!(item_ref(&canonical).unwrap().numeric_id(), Ok(2128));
    }

    #[test]
    fn chameleon_selects_container_before_equipment_and_never_falls_back_from_empty_slot() {
        let mut world = ItemWorldSnapshot {
            target: ItemTarget::ContainerOrEquipment,
            container_exists: true,
            container_slot_item: Some(item("container", 3577)),
            equipment_slot_item: Some(item("equipment", 3582)),
            ..Default::default()
        };
        let result = resolve_tile_item_operation(&mimic(), &world).unwrap();
        assert!(matches!(
            result.operation,
            ItemOperation::MimicItem {
                selected_from: SelectedItemSource::ContainerSlot,
                ..
            }
        ));
        world.container_slot_item = None;
        assert!(matches!(
            resolve_tile_item_operation(&mimic(), &world),
            Err(Error::Refused {
                reason: ItemRefusal::MissingTarget,
                ..
            })
        ));
        world.container_exists = false;
        assert!(matches!(
            resolve_tile_item_operation(&mimic(), &world)
                .unwrap()
                .operation,
            ItemOperation::MimicItem {
                selected_from: SelectedItemSource::EquipmentSlot,
                ..
            }
        ));
    }

    #[test]
    fn chameleon_refuses_creatures_and_immovable_map_items_without_mutation() {
        let mut world = ItemWorldSnapshot {
            tile_exists: true,
            top_item: Some(item("map", 3577)),
            ..Default::default()
        };
        world.top_item.as_mut().unwrap().movable = false;
        let before = world.clone();
        assert!(matches!(
            resolve_tile_item_operation(&mimic(), &world),
            Err(Error::Refused {
                reason: ItemRefusal::ImmovableItem,
                ..
            })
        ));
        assert_eq!(world, before);
        world.target = ItemTarget::Creature;
        assert!(matches!(
            resolve_tile_item_operation(&mimic(), &world),
            Err(Error::Refused {
                reason: ItemRefusal::CreatureTarget,
                ..
            })
        ));
    }

    #[test]
    fn disintegrate_counts_protected_items_toward_the_500_visited_limit() {
        let mut world = ItemWorldSnapshot {
            tile_exists: true,
            caster_in_pz: true,
            ..Default::default()
        };
        world.tile_items = (0..501).map(|n| item(&format!("i{n}"), 3577)).collect();
        for entry in world.tile_items.iter_mut().take(499) {
            entry.script_tagged = true;
        }
        let result = resolve_tile_item_operation(&disintegrate_parameters(), &world).unwrap();
        assert_eq!(
            result.operation,
            ItemOperation::Disintegrate {
                remove_instances: vec!["i499".to_owned()],
                visited_items: 500
            }
        );
        assert!(result.consume_charge);
        assert_eq!(world.tile_items.len(), 501);
    }

    #[test]
    fn disintegrate_protects_each_flag_and_human_corpse_but_succeeds_on_missing_tile() {
        let mut entries = vec![
            item("movable", 3577),
            item("immovable", 3577),
            item("script", 3577),
            item("action", 3577),
            item("corpse", 4240),
        ];
        entries[1].movable = false;
        entries[2].script_tagged = true;
        entries[3].action_tagged = true;
        let world = ItemWorldSnapshot {
            tile_exists: true,
            tile_items: entries,
            ..Default::default()
        };
        assert_eq!(
            resolve_tile_item_operation(&disintegrate_parameters(), &world)
                .unwrap()
                .operation,
            ItemOperation::Disintegrate {
                remove_instances: vec!["movable".to_owned()],
                visited_items: 5
            }
        );
        let empty =
            resolve_tile_item_operation(&disintegrate_parameters(), &ItemWorldSnapshot::default())
                .unwrap();
        assert_eq!(
            empty.operation,
            ItemOperation::Disintegrate {
                remove_instances: Vec::new(),
                visited_items: 0
            }
        );
        assert!(empty.consume_charge);
    }

    #[test]
    fn destroy_refuses_pz_and_unlisted_first_field_even_if_later_item_is_listed() {
        let mut world = ItemWorldSnapshot {
            tile_exists: true,
            caster_in_pz: true,
            first_magic_field: Some(item("first", 2118)),
            ..Default::default()
        };
        assert!(matches!(
            resolve_tile_item_operation(&destroy(), &world),
            Err(Error::Refused {
                reason: ItemRefusal::ProtectionZone,
                ..
            })
        ));
        world.caster_in_pz = false;
        world.first_magic_field = Some(item("first", 2131));
        world.tile_items.push(item("later", 2118));
        assert!(matches!(
            resolve_tile_item_operation(&destroy(), &world),
            Err(Error::Refused {
                reason: ItemRefusal::NoListedField,
                ..
            })
        ));
        world.first_magic_field = Some(item("first", 2118));
        assert_eq!(
            resolve_tile_item_operation(&destroy(), &world)
                .unwrap()
                .operation,
            ItemOperation::RemoveField {
                remove_instance: "first".to_owned()
            }
        );
    }

    #[test]
    fn barriers_select_pvp_variant_and_validate_lifetime_and_refusals() {
        let tile = BarrierTileFacts {
            exists: true,
            ..Default::default()
        };
        let normal = plan_barrier(&barrier(), &tile, false, 16, "Caster").unwrap();
        assert_eq!(normal.item.numeric_id(), Ok(2128));
        assert_eq!(normal.duration_ms, 16000);
        let safe = plan_barrier(&barrier(), &tile, true, 24, "Caster").unwrap();
        assert_eq!(safe.item.numeric_id(), Ok(10181));
        assert_eq!(safe.description, "Casted by: Caster");
        for sample in [15, 25, u32::MAX] {
            assert_eq!(
                plan_barrier(&barrier(), &tile, false, sample, "Caster"),
                Err(Error::RandomOutOfRange)
            );
        }
        for blocked in [
            BarrierTileFacts::default(),
            BarrierTileFacts {
                exists: true,
                floor_change: true,
                creature_on_tile: false,
            },
            BarrierTileFacts {
                exists: true,
                floor_change: false,
                creature_on_tile: true,
            },
        ] {
            assert!(matches!(
                plan_barrier(&barrier(), &blocked, false, 20, "Caster"),
                Err(Error::Refused { .. })
            ));
        }
    }

    #[test]
    fn barrier_rejects_fractional_second_range_and_bad_reference() {
        let tile = BarrierTileFacts {
            exists: true,
            ..Default::default()
        };
        let mut p = barrier();
        p["duration_range_ms"]["minimum"] = json!(16001);
        assert_eq!(
            plan_barrier(&p, &tile, false, 20, "Caster"),
            Err(Error::InvalidParameter("duration_range_ms"))
        );
        p = barrier();
        p["pvp_safe_item"]["family"] = json!("Creature");
        assert_eq!(
            plan_barrier(&p, &tile, true, 20, "Caster"),
            Err(Error::InvalidReference)
        );
    }
}

/// Current Item owner witnesses in source x-major/y-minor iteration order.
/// `top_visible_item=None` includes a creature/empty top; never infer it from tile_items.
#[derive(Debug, Clone)]
pub(crate) struct SourceRemovalTile {
    pub(crate) offset: (i8, i8),
    pub(crate) tile_exists: bool,
    pub(crate) tile_items: Vec<WorldItemFacts>,
    pub(crate) top_visible_item: Option<WorldItemFacts>,
}
/// Source selection only. Output reuses the existing exact-instance durable removal operation;
/// caller still needs current transaction/custody authority and retained commit/replay.
pub(crate) fn plan_source_item_removals(
    selection: crate::content::ProjectV2RemoveItemsSelection,
    listed: &[NativeItemRef],
    tiles: &[SourceRemovalTile],
) -> Result<Vec<(usize, ItemOperation)>, Error> {
    use crate::content::ProjectV2RemoveItemsSelection as S;
    let expected: &[u32] = match selection {
        S::FirstListedPerTile => &[2130, 2129],
        S::TopItemFirstTile => &[10181, 2128, 10182, 2130],
    };
    if listed.len() != expected.len()
        || listed
            .iter()
            .zip(expected)
            .any(|(r, id)| r.numeric_id() != Ok(*id))
    {
        return Err(Error::InvalidParameter("source removal list"));
    }
    let radius: i8 = if selection == S::FirstListedPerTile {
        1
    } else {
        2
    };
    let side =
        usize::try_from(radius * 2 + 1).map_err(|_| Error::InvalidParameter("source radius"))?;
    if tiles.len() != side * side {
        return Err(Error::InvalidParameter("source tile closure"));
    }
    // Qualify the whole bounded closure before any caller can begin physical writes.
    let mut identities = BTreeSet::new();
    for (index, tile) in tiles.iter().enumerate() {
        let expected_offset = (
            i8::try_from(index / side).map_err(|_| Error::InvalidParameter("offset"))? - radius,
            i8::try_from(index % side).map_err(|_| Error::InvalidParameter("offset"))? - radius,
        );
        if tile.offset != expected_offset
            || (!tile.tile_exists
                && (!tile.tile_items.is_empty() || tile.top_visible_item.is_some()))
        {
            return Err(Error::InvalidParameter("source ordered tile witness"));
        }
        for item in &tile.tile_items {
            validate_world_item(item)?;
            if !identities.insert(item.instance_key.clone()) {
                return Err(Error::InvalidParameter("duplicate source item identity"));
            }
        }
        if let Some(top) = &tile.top_visible_item {
            validate_world_item(top)?;
            if !tile.tile_items.contains(top) {
                return Err(Error::InvalidParameter("top visible witness mismatch"));
            }
        }
    }
    let mut result = Vec::new();
    for (index, tile) in tiles.iter().enumerate() {
        if !tile.tile_exists {
            continue;
        }
        let chosen = match selection {
            S::FirstListedPerTile => listed
                .iter()
                .find_map(|wanted| tile.tile_items.iter().find(|i| &i.item == wanted)),
            S::TopItemFirstTile => tile
                .top_visible_item
                .as_ref()
                .filter(|top| listed.contains(&top.item)),
        };
        if let Some(item) = chosen {
            result.push((
                index,
                ItemOperation::RemoveField {
                    remove_instance: item.instance_key.clone(),
                },
            ));
            if selection == S::TopItemFirstTile {
                break;
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod source_removal_tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::content::ProjectV2RemoveItemsSelection as S;
    fn item(id: u32, key: &str) -> WorldItemFacts {
        WorldItemFacts {
            instance_key: key.into(),
            item: NativeItemRef {
                key: format!("oteryn:item.tibia.i{id}"),
                revision: "definition-r1".into(),
            },
            movable: false,
            script_tagged: false,
            action_tagged: false,
        }
    }
    fn tiles(radius: i8) -> Vec<SourceRemovalTile> {
        (-radius..=radius)
            .flat_map(|x| {
                (-radius..=radius).map(move |y| SourceRemovalTile {
                    offset: (x, y),
                    tile_exists: true,
                    tile_items: vec![],
                    top_visible_item: None,
                })
            })
            .collect()
    }
    #[test]
    fn source_anomaly_prioritizes_id2130_over_earlier2129_and_removes_one_per_tile() {
        let listed = vec![item(2130, "a").item, item(2129, "b").item];
        let mut t = tiles(1);
        t[0].tile_items = vec![item(2129, "old-first"), item(2130, "priority")];
        t[1].tile_items = vec![item(2129, "next")];
        let p = plan_source_item_removals(S::FirstListedPerTile, &listed, &t)
            .expect("qualified source removal fixture");
        assert_eq!(
            p,
            vec![
                (
                    0,
                    ItemOperation::RemoveField {
                        remove_instance: "priority".into()
                    }
                ),
                (
                    1,
                    ItemOperation::RemoveField {
                        remove_instance: "next".into()
                    }
                )
            ]
        );
    }
    #[test]
    fn source_destroy_uses_real_top_visible_and_stops_after_first_source_order_tile() {
        let listed = [10181, 2128, 10182, 2130].map(|id| item(id, "list").item);
        let mut t = tiles(2);
        t[0].tile_items = vec![item(2130, "under-creature")]; // None top: current visible creature cannot be an Item.
        t[1].tile_items = vec![item(2128, "first")];
        t[1].top_visible_item = Some(t[1].tile_items[0].clone());
        t[2].tile_items = vec![item(2130, "later")];
        t[2].top_visible_item = Some(t[2].tile_items[0].clone());
        assert_eq!(
            plan_source_item_removals(S::TopItemFirstTile, &listed, &t)
                .expect("qualified source removal fixture"),
            vec![(
                1,
                ItemOperation::RemoveField {
                    remove_instance: "first".into()
                }
            )]
        );
        let mut invalid = t.clone();
        invalid[24].top_visible_item = Some(item(2130, "forged"));
        assert!(plan_source_item_removals(S::TopItemFirstTile, &listed, &invalid).is_err()); // all preflight, even after chosen first
        let mut reversed = t;
        reversed.swap(0, 1);
        assert!(plan_source_item_removals(S::TopItemFirstTile, &listed, &reversed).is_err());
    }
}
