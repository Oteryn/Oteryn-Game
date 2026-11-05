//! SRVGAP-B9: native proficiency Mastery and shared workshop grades, without copied counters.
//! Global-only title rules are the accepted PAR-P12 difference recorded in
//! protocol-oteryn/src/character_info.rs:114-117,838-845 (Crystal has neither title).
//! Grade encoding follows Crystal 201189f player_wheel.cpp:1514-1597 and the Atelier owner:
//! zero is Grade I, two is Grade III, three is Grade IV.
use crate::content::{
    ProjectV2ProficiencyThresholdClass, import_committed_weapon_proficiency_data,
};
use crate::domain::CharacterId;
use crate::domain::weapon_proficiency::{ProficiencyThresholdClass, ProficiencyTrackShape};
use crate::durability::DurabilityError;
use oteryn_gameplay_progression::atelier::{AtelierState, FragmentKind};
use oteryn_gameplay_progression::wheel::base_vocation;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use std::collections::BTreeMap;
use std::sync::LazyLock;

struct Weapon {
    definition: String,
    revision: String,
    mastery: u32,
}

pub(super) struct Source {
    weapons: BTreeMap<String, Weapon>,
    weapon_keys: Vec<String>,
    gems: WheelGemData,
    mods: BTreeMap<String, Vec<(FragmentKind, u16)>>,
}

#[derive(Default)]
pub(super) struct Facts {
    pub weapon_mastery: bool,
    pub all_grade_three_mods: bool,
}

/// Resolve the same embedded catalogues as the native owners before opening a transaction.
pub(super) fn source() -> Option<&'static Source> {
    static SOURCE: LazyLock<Option<Source>> = LazyLock::new(|| {
        let proficiency = import_committed_weapon_proficiency_data().ok()?;
        let mut weapons = BTreeMap::new();
        for (key, binding) in proficiency.bindings() {
            let definition = proficiency
                .definitions()
                .get(&binding.profile_binding.key)?;
            let class = match binding.threshold_class {
                ProjectV2ProficiencyThresholdClass::Standard => ProficiencyThresholdClass::Standard,
                ProjectV2ProficiencyThresholdClass::Knight => ProficiencyThresholdClass::Knight,
                ProjectV2ProficiencyThresholdClass::Crossbow => ProficiencyThresholdClass::Crossbow,
            };
            let shape =
                ProficiencyTrackShape::new(class, u8::try_from(definition.levels.len()).ok()?)
                    .ok()?;
            weapons.insert(
                key.clone(),
                Weapon {
                    definition: definition.identity.key.clone(),
                    revision: definition.identity.revision.clone(),
                    mastery: shape.final_progress(),
                },
            );
        }
        let gems = WheelGemData::embedded().ok()?;
        let mut mods = BTreeMap::new();
        for (vocation, family) in gems.gems()["vocations"].as_object()? {
            let mut identities = std::collections::BTreeSet::new();
            for (kind, field) in [
                (FragmentKind::Basic, "basic_mods_position_1"),
                (FragmentKind::Basic, "basic_mods_position_2"),
                (FragmentKind::Supreme, "supreme_mods"),
            ] {
                for id in family[field].as_array()? {
                    identities.insert((kind, u16::try_from(id.as_u64()?).ok()?));
                }
            }
            if identities.is_empty() {
                return None;
            }
            mods.insert(vocation.clone(), identities.into_iter().collect());
        }
        Some(Source {
            weapon_keys: weapons.keys().cloned().collect(),
            weapons,
            gems,
            mods,
        })
    });
    SOURCE.as_ref()
}

impl Source {
    fn mastered(&self, key: &str, definition: &str, revision: &str, progress: i64) -> bool {
        self.weapons.get(key).is_some_and(|weapon| {
            weapon.definition == definition
                && weapon.revision == revision
                && progress >= i64::from(weapon.mastery)
        })
    }

    fn all_grade_three(&self, state: &AtelierState) -> bool {
        if state.grade_ruleset_revision.as_deref() != Some(self.gems.revision()) {
            return false;
        }
        base_vocation(&state.vocation)
            .and_then(|vocation| self.mods.get(vocation))
            .is_some_and(|mods| {
                mods.iter()
                    .all(|key| state.grades.get(key).is_some_and(|grade| *grade >= 2))
            })
    }

    /// The caller holds the Character root shared lock used by both native writers.
    pub(super) async fn read(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
    ) -> Result<Facts, DurabilityError> {
        // Bounded by the immutable active weapon catalogue and (Character, Item) primary key.
        let tracks: Vec<(String, String, String, i64)> = sqlx::query_as(
            "SELECT item_key,definition_key,definition_revision,progress FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid AND item_key=ANY($2::text[])",
        ).bind(character.as_bytes().as_slice()).bind(&self.weapon_keys).fetch_all(&mut **tx).await?;
        let atelier =
            crate::durability::character_atelier::load_in_transaction(tx, character).await?;
        Ok(Facts {
            weapon_mastery: tracks.iter().any(|(key, definition, revision, progress)| {
                self.mastered(key, definition, revision, *progress)
            }),
            all_grade_three_mods: atelier
                .as_ref()
                .is_some_and(|state| self.all_grade_three(state)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaudit_title_predicates_mastery_uses_the_bound_weapon_cap()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = source().ok_or("enhancement source")?;
        let (key, weapon) = source.weapons.first_key_value().ok_or("weapon")?;
        assert!(!source.mastered(
            key,
            &weapon.definition,
            &weapon.revision,
            i64::from(weapon.mastery) - 1
        ));
        assert!(source.mastered(
            key,
            &weapon.definition,
            &weapon.revision,
            i64::from(weapon.mastery)
        ));
        assert!(!source.mastered(key, &weapon.definition, "obsolete", i64::MAX));
        assert!(!source.mastered("unknown", &weapon.definition, &weapon.revision, i64::MAX));
        Ok(())
    }

    #[test]
    fn reaudit_title_predicates_grades_require_the_complete_current_vocation_set()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = source().ok_or("enhancement source")?;
        for (vocation, mods) in &source.mods {
            let mut state = AtelierState {
                vocation: vocation.clone(),
                grade_ruleset_revision: Some(source.gems.revision().into()),
                ..Default::default()
            };
            assert!(!source.all_grade_three(&state));
            for key in mods {
                state.grades.insert(*key, 2);
            }
            assert!(source.all_grade_three(&state));
            let first = *mods.first().ok_or("mod")?;
            state.grades.insert(first, 1);
            assert!(!source.all_grade_three(&state));
            state.grades.insert(first, 3);
            assert!(source.all_grade_three(&state));
            state.grade_ruleset_revision = Some("obsolete".into());
            assert!(!source.all_grade_three(&state));
        }
        Ok(())
    }
}
