//! Server Item authoring carried by the existing native Item profile JSON section.
//! No Reference binary grammar, item admission or combat behaviour changes here.
use super::*;
use crate::content::ReferenceAmmoType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectV2RangedElement {
    Physical,
    Death,
    Earth,
    Energy,
    Fire,
    Ice,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum ProjectV2RangedWeapon {
    Distance {
        attack: u16,
        range_cells: u8,
        /// Signed launcher bonus; zero selects the source's skill/distance formula.
        hit_chance: i8,
        max_hit_chance: u8,
        ammunition: ReferenceAmmoType,
    },
    Throwing {
        attack: u16,
        range_cells: u8,
        /// Zero selects the source formula; positive values are absolute overrides.
        hit_chance: i8,
        max_hit_chance: u8,
        element: ProjectV2RangedElement,
        elemental_attack: u16,
    },
    Ammunition {
        attack: u16,
        ammunition: ReferenceAmmoType,
        hit_chance: i8,
        max_hit_chance: u8,
        element: ProjectV2RangedElement,
        elemental_attack: u16,
    },
    Wand(ProjectV2MagicWeapon),
    Rod(ProjectV2MagicWeapon),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2MagicWeapon {
    pub damage_min: u16,
    pub damage_max: u16,
    pub element: ProjectV2RangedElement,
    pub mana_per_shot: u16,
    pub range_cells: u8,
}

impl ProjectV2RangedWeapon {
    pub(super) fn validate(&self) -> Result<(), ProjectError> {
        let valid = match self {
            Self::Distance {
                range_cells,
                hit_chance,
                max_hit_chance,
                ..
            } => {
                (1..=7).contains(range_cells)
                    && (-100..=100).contains(hit_chance)
                    && (1..=100).contains(max_hit_chance)
            }
            Self::Throwing {
                range_cells,
                hit_chance,
                max_hit_chance,
                element,
                elemental_attack,
                ..
            } => {
                (1..=7).contains(range_cells)
                    && (0..=100).contains(hit_chance)
                    && (1..=100).contains(max_hit_chance)
                    && (*element == ProjectV2RangedElement::Physical) == (*elemental_attack == 0)
            }
            Self::Ammunition {
                hit_chance,
                max_hit_chance,
                element,
                elemental_attack,
                ..
            } => {
                (0..=100).contains(hit_chance)
                    && (1..=100).contains(max_hit_chance)
                    && (*element == ProjectV2RangedElement::Physical) == (*elemental_attack == 0)
            }
            Self::Wand(weapon) | Self::Rod(weapon) => {
                (1..=7).contains(&weapon.range_cells)
                    && weapon.damage_min > 0
                    && weapon.damage_min <= weapon.damage_max
                    && weapon.mana_per_shot > 0
                    && weapon.element != ProjectV2RangedElement::Physical
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ProjectError::InvalidProject("invalid ranged Item profile"))
        }
    }
}

// The source profile is separate from the sealed Item authoring map. Resolve only
// fields explicitly owned there; missing unowned facts still fail strict decoding.
impl TryFrom<(&serde_json::Value, &ProjectV2ItemAuthoring)> for ProjectV2RangedWeapon {
    type Error = ProjectError;

    fn try_from(
        (profile, authoring): (&serde_json::Value, &ProjectV2ItemAuthoring),
    ) -> Result<Self, Self::Error> {
        let invalid = || ProjectError::InvalidProject("invalid ranged Item profile owners");
        let mut profile = profile.clone();
        let fields = profile.as_object_mut().ok_or_else(invalid)?;
        if let Some(attack) = authoring.weapon_attack_modifier_points
            && fields.insert("attack".into(), attack.0.into()).is_some()
        {
            return Err(invalid());
        }
        if let Some(hit) = authoring.weapon_absolute_hit_chance_percent {
            let numerator = i128::from(hit.numerator);
            let denominator = i128::from(hit.denominator);
            if denominator == 0 || numerator % denominator != 0 {
                return Err(invalid());
            }
            let hit = i64::try_from(numerator / denominator).map_err(|_| invalid())?;
            if fields.insert("hit_chance".into(), hit.into()).is_some() {
                return Err(invalid());
            }
        }
        let weapon: Self = serde_json::from_value(profile).map_err(|_| invalid())?;
        weapon.validate()?;
        Ok(weapon)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use serde_json::{Value, json};
    use std::{collections::BTreeMap, path::Path};

    fn items() -> BTreeMap<String, ProjectV2ItemAuthoring> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let index: Value =
            serde_json::from_slice(&std::fs::read(root.join("content/items/index.json")).unwrap())
                .unwrap();
        let mut result = BTreeMap::new();
        let profiles: Value = serde_json::from_slice(
            &std::fs::read(root.join("content/items/ranged-profiles.json")).unwrap(),
        )
        .unwrap();
        let profiles: BTreeMap<_, _> = profiles["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| (record["item"]["key"].as_str().unwrap(), record))
            .collect();
        for path in index["shards"].as_array().unwrap() {
            let shard: Value =
                serde_json::from_slice(&std::fs::read(root.join(path.as_str().unwrap())).unwrap())
                    .unwrap();
            for row in shard["records"].as_array().unwrap() {
                let key = row["definition"]["identity"]["key"].as_str().unwrap();
                let Some(profile) = profiles.get(key) else {
                    continue;
                };
                // The profile moved out of sealed authoring; resolve its existing owners.
                assert!(row["authoring"].get("ranged").is_none());
                let mut authoring = row.get("authoring").cloned().unwrap_or(json!({}));
                authoring["item"] = row["definition"]["identity"].clone();
                assert_eq!(profile["item"], authoring["item"]);
                let mut authoring: ProjectV2ItemAuthoring =
                    serde_json::from_value(authoring).unwrap();
                authoring.ranged = Some(
                    ProjectV2RangedWeapon::try_from((&profile["profile"], &authoring)).unwrap(),
                );
                validate_native_gameplay_item(&authoring).unwrap();
                assert!(
                    result
                        .insert(authoring.item.key.clone(), authoring)
                        .is_none()
                );
            }
        }
        result
    }

    #[test]
    fn ranged_bow_and_crossbow_load_ammunition_attack_hit_and_range() {
        let all = items();
        for (id, ammunition, range_cells) in [
            (3350, ReferenceAmmoType::Arrow, 6),
            (3349, ReferenceAmmoType::Bolt, 5),
        ] {
            assert_eq!(
                all[&format!("oteryn:item.tibia.i{id}")].ranged,
                Some(ProjectV2RangedWeapon::Distance {
                    attack: 0,
                    range_cells,
                    hit_chance: 0,
                    max_hit_chance: 90,
                    ammunition,
                })
            );
        }
    }

    #[test]
    fn ranged_spear_arrow_bolt_and_elemental_arrow_load_source_values() {
        let all = items();
        assert_eq!(
            all["oteryn:item.tibia.i3277"].ranged,
            Some(ProjectV2RangedWeapon::Throwing {
                attack: 25,
                range_cells: 3,
                hit_chance: 0,
                max_hit_chance: 76,
                element: ProjectV2RangedElement::Physical,
                elemental_attack: 0,
            })
        );
        // Absolute hit chance comes from the sealed Item owner, replacing Canary's zero default.
        for (id, ammunition, attack, hit_chance, max_hit_chance, element, elemental_attack) in [
            (
                3447,
                ReferenceAmmoType::Arrow,
                25,
                90,
                91,
                ProjectV2RangedElement::Physical,
                0,
            ),
            (
                3446,
                ReferenceAmmoType::Bolt,
                30,
                86,
                87,
                ProjectV2RangedElement::Physical,
                0,
            ),
            (
                761,
                ReferenceAmmoType::Arrow,
                14,
                90,
                91,
                ProjectV2RangedElement::Energy,
                14,
            ),
        ] {
            assert_eq!(
                all[&format!("oteryn:item.tibia.i{id}")].ranged,
                Some(ProjectV2RangedWeapon::Ammunition {
                    attack,
                    ammunition,
                    hit_chance,
                    max_hit_chance,
                    element,
                    elemental_attack,
                })
            );
        }
    }

    #[test]
    fn ranged_wand_of_vortex_and_snakebite_rod_load_damage_element_mana_and_range() {
        let all = items();
        let wand = ProjectV2MagicWeapon {
            damage_min: 8,
            damage_max: 18,
            element: ProjectV2RangedElement::Energy,
            mana_per_shot: 1,
            range_cells: 3,
        };
        assert_eq!(
            all["oteryn:item.tibia.i3074"].ranged,
            Some(ProjectV2RangedWeapon::Wand(wand.clone()))
        );
        assert_eq!(
            all["oteryn:item.tibia.i3066"].ranged,
            Some(ProjectV2RangedWeapon::Rod(ProjectV2MagicWeapon {
                element: ProjectV2RangedElement::Earth,
                mana_per_shot: 2,
                ..wand
            }))
        );
        assert_eq!(all.len(), 162);
        assert!(!all.contains_key("oteryn:item.tibia.i49522"));
        assert!(!all.contains_key("oteryn:item.tibia.i15792"));
    }

    #[test]
    fn ranged_missing_required_fields_and_invalid_profiles_are_rejected() {
        let all = items();
        for (id, fields) in [
            (
                3350,
                vec![
                    "ammunition",
                    "attack",
                    "range_cells",
                    "hit_chance",
                    "max_hit_chance",
                ],
            ),
            (
                3277,
                vec![
                    "attack",
                    "range_cells",
                    "hit_chance",
                    "max_hit_chance",
                    "element",
                    "elemental_attack",
                ],
            ),
            (
                3447,
                vec![
                    "attack",
                    "ammunition",
                    "hit_chance",
                    "max_hit_chance",
                    "element",
                    "elemental_attack",
                ],
            ),
            (
                3074,
                vec![
                    "damage_min",
                    "damage_max",
                    "element",
                    "mana_per_shot",
                    "range_cells",
                ],
            ),
            (
                3066,
                vec![
                    "damage_min",
                    "damage_max",
                    "element",
                    "mana_per_shot",
                    "range_cells",
                ],
            ),
        ] {
            let original = serde_json::to_value(&all[&format!("oteryn:item.tibia.i{id}")]).unwrap();
            for field in fields {
                let mut missing = original.clone();
                missing["ranged"].as_object_mut().unwrap().remove(field);
                assert!(
                    serde_json::from_value::<ProjectV2ItemAuthoring>(missing).is_err(),
                    "{id}: {field}"
                );
            }
        }
        let original = serde_json::to_value(&all["oteryn:item.tibia.i3074"]).unwrap();
        for (field, value) in [
            ("range_cells", json!(0)),
            ("range_cells", json!(8)),
            ("damage_min", json!(19)),
            ("mana_per_shot", json!(0)),
            ("element", json!("PHYSICAL")),
        ] {
            let mut invalid = original.clone();
            invalid["ranged"][field] = value;
            let item: ProjectV2ItemAuthoring = serde_json::from_value(invalid).unwrap();
            assert!(validate_native_gameplay_item(&item).is_err());
        }
        let mut absent = original;
        absent.as_object_mut().unwrap().remove("ranged");
        let absent: ProjectV2ItemAuthoring = serde_json::from_value(absent).unwrap();
        assert!(absent.ranged.is_none());
    }
}
