//! Source-bound data consumed by the production Primal cadence and death owners.
//! Crystal201189f the_primal_menace.lua:27-39,66-69,156-347;
//! hazard_primal.lua:24-91 and creaturescripts_the_primal_menace_killed.lua:3-18.
use crate::content::native_gameplay::{
    CreatureProfileRecord, ItemProfileRecord, NativeGameplayState,
};
use crate::content::{ProjectV2AuthoringProfileData, ReferenceItemField};

pub const PRIMAL_MENACE_KEY: &str = "oteryn:creature.the_primal_menace";
pub const FUNGOSAURUS_KEY: &str = "oteryn:creature.fungosaurus";
// One roster supplies both data qualification and actual native scheduled spawns.
pub const PRIMAL_BEAST_POOL: [&str; 11] = [
    "oteryn:creature.emerald_tortoise_primal",
    "oteryn:creature.gore_horn_primal",
    "oteryn:creature.gorerilla_primal",
    "oteryn:creature.headpecker_primal",
    "oteryn:creature.hulking_prehemoth_primal",
    "oteryn:creature.mantosaurus_primal",
    "oteryn:creature.nighthunter_primal",
    "oteryn:creature.noxious_ripptor_primal",
    "oteryn:creature.sabretooth_primal",
    "oteryn:creature.stalking_stalk_primal",
    "oteryn:creature.sulphider_primal",
];

/// Only the complete current generation can construct this capability. Both the
/// room qualifier and its real native cadence consume it; callback spelling alone
/// never admits an encounter, a creature or a portable Item.
pub struct QualifiedPrimalClosure {
    digest: [u8; 32],
}
impl QualifiedPrimalClosure {
    pub fn source_digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(super) fn implements(&self, hook: &str) -> bool {
        matches!(
            hook,
            "ThePrimalMenaceDeath" | "ThePrimeOrdealBossDeath" | "mType.onThink"
        )
    }
}

pub fn qualify(native: &NativeGameplayState) -> Option<QualifiedPrimalClosure> {
    for key in PRIMAL_BEAST_POOL
        .into_iter()
        .chain([PRIMAL_MENACE_KEY, FUNGOSAURUS_KEY])
    {
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == key)?;
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|p| p.definition_key == key)?;
        if policy.maximum_health <= 0 || !abilities_closed(record) {
            return None;
        }
    }
    // Crystal the_primal_menace.lua:79,118: source corpse and literal empty loot. An absent
    // binding is Unknown, not a source proof that a kill has no loot entries.
    let rewards = native.reward_table().row(PRIMAL_MENACE_KEY).ok()?;
    if rewards.corpse_item.family != "Item"
        || rewards.corpse_item.production_key != "oteryn:item.tibia.i39530"
        || rewards.loot_table_ref.family != "Loot"
        || rewards.loot_table_ref.production_key != "oteryn:loot.creature.the_primal_menace"
        || rewards.loot_table_ref.revision_ref != "definition-r1"
        || !rewards.loot_table.entries.is_empty()
    {
        return None;
    }
    // The source corpse's complete decay chain stays on existing Item owners.
    for id in 39527..=39530 {
        native.item_policy(&format!("oteryn:item.tibia.i{id}"), "definition-r1")?;
    }
    // hazard_primal.lua:78-87 creates a real immovable, unpickupable pod. It
    // blocks no tile; only this exact pinned source policy may project it.
    let pod = native
        .item_policy("oteryn:item.tibia.i39176", "definition-r1")?
        .record();
    if !pod_closed(pod) {
        return None;
    }
    Some(QualifiedPrimalClosure {
        digest: native.source_digest(),
    })
}

fn pod_closed(pod: &ItemProfileRecord) -> bool {
    let ReferenceItemField::Known(physical) = &pod.semantics.physical else {
        return false;
    };
    physical.movable == ReferenceItemField::Known(false)
        && physical.pickupable == ReferenceItemField::Known(false)
        && pod.attributes.blocks_movement == Some(false)
}

pub(super) fn abilities_closed(record: &CreatureProfileRecord) -> bool {
    let Some(crate::content::ProjectV2AuthoringProfile {
        data: ProjectV2AuthoringProfileData::Behavior(behavior),
        ..
    }) = &record.behavior
    else {
        return false;
    };
    behavior.summons.is_none()
        && behavior.event_bindings.is_empty()
        && behavior
            .attacks
            .iter()
            .chain(behavior.defenses.iter())
            .all(|attack| {
                record
                    .monster_melee
                    .as_ref()
                    .is_some_and(|m| m.ability == attack.ability)
                    || record
                        .source_item_abilities
                        .iter()
                        .any(|p| p.target == attack.ability)
                    || record
                        .attack_definitions
                        .as_ref()
                        .is_some_and(|d| d.profiles.iter().any(|p| p.target == attack.ability))
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_closure_requires_exact_source_pod_and_complete_ability_profiles() -> Result<(), String>
    {
        let items: crate::content::native_gameplay::ItemProfilesDocument =
            serde_json::from_slice(include_bytes!(
                "../../../../../../content/items/definitions/spell-native-profiles.json"
            ))
            .map_err(|e| e.to_string())?;
        let pod = items
            .records
            .iter()
            .find(|r| r.production_definition.production_key == "oteryn:item.tibia.i39176")
            .ok_or("actual source Primal pod profile")?;
        assert!(pod_closed(pod));
        let mut changed = pod.clone();
        changed.attributes.blocks_movement = Some(true);
        assert!(!pod_closed(&changed));
        let ReferenceItemField::Known(physical) = &mut changed.semantics.physical else {
            return Err("source physical facts".into());
        };
        changed.attributes.blocks_movement = Some(false);
        physical.pickupable = ReferenceItemField::Known(true);
        assert!(!pod_closed(&changed));
        let ReferenceItemField::Known(physical) = &mut changed.semantics.physical else {
            return Err("source physical facts".into());
        };
        physical.pickupable = ReferenceItemField::Known(false);
        physical.movable = ReferenceItemField::Unknown;
        assert!(!pod_closed(&changed));
        let creatures: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(include_bytes!(
                "../../../../../../content/creatures/definitions/spell-native-profiles.json"
            ))
            .map_err(|e| e.to_string())?;
        for key in PRIMAL_BEAST_POOL
            .into_iter()
            .chain([PRIMAL_MENACE_KEY, FUNGOSAURUS_KEY])
        {
            let source = creatures
                .records
                .iter()
                .find(|r| r.profile.target.key == key)
                .ok_or_else(|| format!("source profile {key}"))?;
            assert!(
                abilities_closed(source),
                "actual source ability closure {key}"
            );
            let mut unsupported = source.clone();
            let Some(crate::content::ProjectV2AuthoringProfile {
                data: ProjectV2AuthoringProfileData::Behavior(behavior),
                ..
            }) = &mut unsupported.behavior
            else {
                return Err("source Behavior profile".into());
            };
            let attack = behavior.attacks.first_mut().ok_or("source attack")?;
            attack.ability.revision = "unadmitted-revision".into();
            assert!(
                !abilities_closed(&unsupported),
                "a missing ability cannot inherit its source name"
            );
        }
        Ok(())
    }
}
