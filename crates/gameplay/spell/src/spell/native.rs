//! Source-qualified candidate planners for the 67 formerly unresolved spell records.
//!
//! The closed reader binds every mechanic to its complete qualified cast header and
//! dependencies. Plans use owner-supplied facts and never write world or durable state.
//! Registration here does not expand the admitted V1 spell book or its wire protocol.

use {
    super::{
        Vocation, native_combat as combat, native_delayed as delayed,
        native_house_movement as house,
    },
    oteryn_gameplay_spell_rules::{
        native_actor_states as actor, native_companions as companions, native_items as items,
    },
};

pub use crate::content::spell_data::native::*;

/// These inputs are snapshots from the owning domains, never client authority.
#[derive(Debug)]
pub enum Facts<'a> {
    Focus(actor::FocusFacts),
    Stance {
        active: Option<actor::StandardStance>,
        vocation: Vocation,
    },
    Equipment(&'a actor::EquipmentFacts),
    Combat(&'a combat::NativeCombatFacts),
    Companion(&'a companions::CompanionFacts),
    Delayed(&'a delayed::NativeDelayedFacts),
    House(&'a house::HouseMovementFacts),
    Item(&'a items::ItemWorldSnapshot),
    Food,
    Barrier {
        tile: &'a items::BarrierTileFacts,
        optional_pvp: bool,
        sample_seconds: u32,
        caster_name: &'a str,
    },
}

#[derive(Debug)]
pub enum Plan {
    Focus(Box<actor::FocusPlan>),
    Stance(Box<actor::StancePlan>),
    Equipment(Box<actor::EquipmentAttackPlan>),
    Combat(Box<combat::NativeCombatPlan>),
    Companion(Box<companions::CompanionPlan>),
    Delayed(Box<delayed::NativeDelayedPlan>),
    House(Box<house::HouseMovementPlan>),
    Item(Box<items::ItemOperationPlan>),
    Food {
        items: Vec<items::NativeItemRef>,
        overflow: items::ItemGrantOverflow,
    },
    Barrier(Box<items::BarrierPlan>),
}

pub trait CompiledNativeSpellPlanExt {
    fn plan(&self, facts: Facts<'_>, draw: &mut dyn FnMut(i64, i64) -> i64) -> Result<Plan, Error>;
}

impl CompiledNativeSpellPlanExt for CompiledNativeSpell {
    fn plan(&self, facts: Facts<'_>, draw: &mut dyn FnMut(i64, i64) -> i64) -> Result<Plan, Error> {
        let execution = &self.spell()["execution"];
        if let Some(native) = execution.get("native_behavior") {
            let key = native["key"]
                .as_str()
                .ok_or_else(|| Error("missing native key".into()))?;
            let parameters = &native["parameters"];
            match (key, facts) {
                ("monk_focus", Facts::Focus(f)) => actor::plan_focus(parameters, f)
                    .map(|p| Plan::Focus(Box::new(p)))
                    .map_err(|e| Error(format!("{e:?}"))),
                ("stance_toggle", Facts::Stance { active, vocation }) => {
                    actor::plan_stance(parameters, active, vocation)
                        .map(|p| Plan::Stance(Box::new(p)))
                        .map_err(|e| Error(format!("{e:?}")))
                }
                ("equipment_attack", Facts::Equipment(f)) => actor::equipment_attack(parameters, f)
                    .map(|p| Plan::Equipment(Box::new(p)))
                    .map_err(|e| Error(format!("{e:?}"))),
                (
                    "wheel_combat"
                    | "avatar_state"
                    | "monster_ai_override"
                    | "mass_spirit_mend"
                    | "mana_shield_capacity",
                    Facts::Combat(f),
                ) => combat::plan(key, parameters, f, draw)
                    .map(|p| Plan::Combat(Box::new(p)))
                    .map_err(|e| Error(format!("{e:?}"))),
                ("familiar_summon" | "companion_haste" | "acquire_summon", Facts::Companion(f)) => {
                    companions::plan(parameters, f)
                        .map(|p| Plan::Companion(Box::new(p)))
                        .map_err(|e| Error(format!("{e:?}")))
                }
                ("delayed_strike" | "owned_field_buff", Facts::Delayed(f)) => {
                    delayed::plan(key, parameters, f, draw)
                        .map(|p| Plan::Delayed(Box::new(p)))
                        .map_err(|e| Error(format!("{e:?}")))
                }
                (
                    "house_access" | "locate_message" | "vertical_move" | "creature_appearance",
                    Facts::House(f),
                ) => house::plan(parameters, f)
                    .map(|p| Plan::House(Box::new(p)))
                    .map_err(|e| Error(format!("{e:?}"))),
                ("tile_item_operation", Facts::Item(f)) => {
                    items::resolve_tile_item_operation(parameters, f)
                        .map(|p| Plan::Item(Box::new(p)))
                        .map_err(|e| Error(format!("{e:?}")))
                }
                ("random_item_grant", Facts::Food) => {
                    let mut checked_draw = |min, max| {
                        u32::try_from(draw(i64::from(min), i64::from(max))).unwrap_or(u32::MAX)
                    };
                    let grants = items::plan_random_item_grant(parameters, &mut checked_draw)
                        .map_err(|e| Error(format!("{e:?}")))?;
                    let overflow = items::random_item_grant_overflow(parameters)
                        .map_err(|e| Error(format!("{e:?}")))?;
                    Ok(Plan::Food {
                        items: grants,
                        overflow,
                    })
                }
                _ => Err(Error("facts do not match the native mechanic".into())),
            }
        } else if let Facts::Barrier {
            tile,
            optional_pvp,
            sample_seconds,
            caster_name,
        } = facts
        {
            let effects = self.dependencies()["effects"]
                .as_array()
                .ok_or_else(|| Error("missing barrier effect".into()))?;
            let effect = effects
                .first()
                .ok_or_else(|| Error("missing barrier effect".into()))?;
            items::plan_barrier(effect, tile, optional_pvp, sample_seconds, caster_name)
                .map(|p| Plan::Barrier(Box::new(p)))
                .map_err(|e| Error(format!("{e:?}")))
        } else {
            Err(Error("barrier requires placement facts".into()))
        }
    }
}
