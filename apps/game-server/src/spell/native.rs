//! Source-qualified candidate planners for the 67 formerly unresolved spell records.
//!
//! The closed reader binds every mechanic to its complete qualified cast header and
//! dependencies. Plans use owner-supplied facts and never write world or durable state.
//! Registration here does not expand the admitted V1 spell book or its wire protocol.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde_json::Value;

use super::{
    Vocation, native_actor_states as actor, native_combat as combat,
    native_companions as companions, native_delayed as delayed, native_house_movement as house,
    native_items as items,
};

const PROFILE_JSON: &str = include_str!(
    "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
);
const REVISION: &str = "spell-p2-r20";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Error(pub(crate) String);

fn profiles() -> Result<&'static Vec<Value>, Error> {
    static PROFILES: OnceLock<Result<Vec<Value>, Error>> = OnceLock::new();
    PROFILES
        .get_or_init(|| {
            let document: Value =
                serde_json::from_str(PROFILE_JSON).map_err(|e| Error(e.to_string()))?;
            if document["revision"] != REVISION {
                return Err(Error("unqualified native profile revision".into()));
            }
            let profiles = document["profiles"]
                .as_array()
                .ok_or_else(|| Error("missing native profiles".into()))?;
            if profiles.len() != 67 {
                return Err(Error("incomplete native profile coverage".into()));
            }
            let mut identities = BTreeSet::new();
            for profile in profiles {
                let spell = &profile["spell"];
                if profile["name"] != spell["name"]
                    || profile["carrier"] != spell["carrier"]
                    || profile["execution"] != spell["execution"]
                    || spell["identity"]["revision"] != REVISION
                    || !identities
                        .insert((profile["carrier"].to_string(), profile["name"].to_string()))
                {
                    return Err(Error("inconsistent or duplicate native profile".into()));
                }
            }
            Ok(profiles.clone())
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Construction is private; altered parameters, costs, targeting and dependencies
/// cannot reach any planner through this reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledNativeSpell {
    spell: Value,
    dependencies: Value,
}

pub(crate) fn spell_from_bundle(
    bundle: &Value,
    dependencies: &Value,
) -> Result<CompiledNativeSpell, Error> {
    let object = bundle
        .as_object()
        .ok_or_else(|| Error("expected spell bundle".into()))?;
    if object.len() != 1 || !object.contains_key("spell") {
        return Err(Error("unexpected spell bundle properties".into()));
    }
    let spell = &bundle["spell"];
    let profile = profiles()?
        .iter()
        .find(|profile| profile["name"] == spell["name"] && profile["carrier"] == spell["carrier"])
        .ok_or_else(|| Error("spell has no qualified native profile".into()))?;
    if spell != &profile["spell"] || dependencies != &profile["dependencies"] {
        return Err(Error(
            "native spell differs from its qualified profile".into(),
        ));
    }
    Ok(CompiledNativeSpell {
        spell: spell.clone(),
        dependencies: dependencies.clone(),
    })
}

/// These inputs are snapshots from the owning domains, never client authority.
#[derive(Debug)]
pub(crate) enum Facts<'a> {
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
pub(crate) enum Plan {
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

impl CompiledNativeSpell {
    /// The complete qualified cast header remains available to the cast owner for
    /// its common checks and the single cost/cooldown commitment.
    pub(crate) fn dependencies(&self) -> &Value {
        &self.dependencies
    }
    pub(crate) fn spell(&self) -> &Value {
        &self.spell
    }

    pub(crate) fn plan(
        &self,
        facts: Facts<'_>,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<Plan, Error> {
        let execution = &self.spell["execution"];
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
            let effects = self.dependencies["effects"]
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

#[cfg(test)]
mod tests {
    // Panicking assertions are confined to regression tests.
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::json;

    #[test]
    fn all_67_profiles_load_and_semantic_edits_are_refused() {
        for profile in profiles().unwrap() {
            let bundle = json!({"spell": profile["spell"]});
            let dependencies = &profile["dependencies"];
            assert!(
                spell_from_bundle(&bundle, dependencies).is_ok(),
                "{}",
                profile["name"]
            );
            for altered in ["costs", "targeting", "execution", "requirements"] {
                let mut changed = bundle.clone();
                changed["spell"][altered]["unqualified"] = json!(true);
                assert!(spell_from_bundle(&changed, dependencies).is_err());
            }
            let mut changed = dependencies.clone();
            changed["unqualified"] = json!(true);
            assert!(spell_from_bundle(&bundle, &changed).is_err());
        }
    }

    #[test]
    fn wrong_facts_fail_before_rng_and_authoring_retains_native_profile() {
        let profile = profiles()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "Blood Rage")
            .unwrap();
        let compiled = spell_from_bundle(
            &json!({"spell": profile["spell"]}),
            &profile["dependencies"],
        )
        .unwrap();
        let mut calls = 0;
        assert!(
            compiled
                .plan(Facts::Food, &mut |_, _| {
                    calls += 1;
                    0
                })
                .is_err()
        );
        assert_eq!(calls, 0);
        assert!(
            super::super::authoring::spell_from_bundle(
                &json!({"spell": profile["spell"]}),
                &profile["dependencies"]
            )
            .is_ok()
        );
    }

    #[test]
    fn food_and_barrier_dispatch_real_qualified_data() {
        let all = profiles().unwrap();
        let food = all.iter().find(|p| p["name"] == "Food").unwrap();
        let compiled =
            spell_from_bundle(&json!({"spell": food["spell"]}), &food["dependencies"]).unwrap();
        let mut draws = vec![1, 2, 7].into_iter();
        let Plan::Food { items, overflow } = compiled
            .plan(Facts::Food, &mut |_, _| draws.next().unwrap())
            .unwrap()
        else {
            panic!("wrong plan")
        };
        assert_eq!(items.len(), 2);
        assert_eq!(overflow, items::ItemGrantOverflow::DropOnCasterTile);
        let wall = all
            .iter()
            .find(|p| p["name"] == "Magic Wall Rune" && p["carrier"] == "rune")
            .unwrap();
        let compiled =
            spell_from_bundle(&json!({"spell": wall["spell"]}), &wall["dependencies"]).unwrap();
        let tile = items::BarrierTileFacts {
            exists: true,
            floor_change: false,
            creature_on_tile: false,
        };
        let Plan::Barrier(plan) = compiled
            .plan(
                Facts::Barrier {
                    tile: &tile,
                    optional_pvp: true,
                    sample_seconds: 20,
                    caster_name: "Caster",
                },
                &mut |_, _| panic!("barrier sampling already supplied"),
            )
            .unwrap()
        else {
            panic!("wrong plan")
        };
        assert_eq!(plan.item.numeric_id().unwrap(), 10181);
        assert_eq!(plan.duration_ms, 20000);
    }
}
