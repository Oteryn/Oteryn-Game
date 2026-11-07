//! The live Workshop catalogue is independent of the character's revealed gems.
//! Only Query308 uses these rows; stored grades and mutation receipts stay sparse.
use std::collections::BTreeSet;

use oteryn_gameplay_progression::atelier::{AtelierState, FragmentKind};
use oteryn_gameplay_progression::wheel::base_vocation;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use oteryn_protocol_oteryn::atelier::{GradeView, MAX_GRADE_ENTRIES};

pub(super) fn grades_for_query(state: &AtelierState) -> Option<Vec<GradeView>> {
    let data = WheelGemData::embedded().ok()?;
    let family = &data.gems()["vocations"][base_vocation(&state.vocation)?];
    let mut compatible = BTreeSet::new();
    for (kind, field) in [
        (FragmentKind::Basic, "basic_mods_position_1"),
        (FragmentKind::Basic, "basic_mods_position_2"),
        (FragmentKind::Supreme, "supreme_mods"),
    ] {
        let choices = family[field].as_array()?;
        if choices.is_empty() || choices.len() > MAX_GRADE_ENTRIES {
            return None;
        }
        for choice in choices {
            compatible.insert((kind, u16::try_from(choice.as_u64()?).ok()?));
            if compatible.len() > MAX_GRADE_ENTRIES {
                return None;
            }
        }
    }
    compatible
        .into_iter()
        .map(|(kind, mod_id)| {
            let grade = state.grades.get(&(kind, mod_id)).copied().unwrap_or(0);
            (grade <= 3).then_some(GradeView {
                kind: u8::from(kind == FragmentKind::Supreme),
                mod_id,
                grade,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_query_lists_all_compatible_modifiers_without_revealed_gems()
    -> Result<(), &'static str> {
        let expected_basic: Vec<u16> = (0..=31).chain(33..=41).chain(44..=48).collect();
        for (vocation, exclusive_supreme, foreign_supreme) in [
            ("knight", 23, 41),
            ("paladin", 41, 58),
            ("sorcerer", 58, 75),
            ("druid", 75, 93),
            ("monk", 93, 23),
        ] {
            let state = AtelierState {
                vocation: vocation.into(),
                ..AtelierState::default()
            };
            let rows = grades_for_query(&state).ok_or("compatible Workshop catalogue")?;
            assert_eq!(rows.len(), 69, "{vocation}");
            assert!(rows.len() <= MAX_GRADE_ENTRIES);
            assert_eq!(rows.iter().filter(|row| row.kind == 1).count(), 23);
            assert_eq!(
                rows.iter()
                    .filter(|row| row.kind == 0)
                    .map(|row| row.mod_id)
                    .collect::<Vec<_>>(),
                expected_basic,
                "{vocation}"
            );
            assert!(rows.iter().all(|row| row.grade == 0));
            assert!(
                rows.windows(2)
                    .all(|pair| (pair[0].kind, pair[0].mod_id) < (pair[1].kind, pair[1].mod_id))
            );
            assert!(
                rows.iter()
                    .any(|row| row.kind == 1 && row.mod_id == exclusive_supreme)
            );
            assert!(
                !rows
                    .iter()
                    .any(|row| row.kind == 1 && row.mod_id == foreign_supreme)
            );
        }
        Ok(())
    }

    #[test]
    fn workshop_query_overlays_sparse_grades_and_normalizes_promoted_vocation()
    -> Result<(), &'static str> {
        let mut state = AtelierState {
            vocation: "master_sorcerer".into(),
            ..AtelierState::default()
        };
        state.grades.insert((FragmentKind::Basic, 0), 2);
        state.grades.insert((FragmentKind::Supreme, 58), 3);
        let rows = grades_for_query(&state).ok_or("promoted Workshop catalogue")?;
        assert_eq!(rows.len(), 69);
        for row in &rows {
            assert_eq!(
                row.grade,
                match (row.kind, row.mod_id) {
                    (0, 0) => 2,
                    (1, 58) => 3,
                    _ => 0,
                }
            );
        }
        state.vocation = "sorcerer".into();
        assert_eq!(
            grades_for_query(&state).ok_or("base Workshop catalogue")?,
            rows
        );
        state.grades.insert((FragmentKind::Basic, 0), 4);
        assert!(
            grades_for_query(&state).is_none(),
            "an invalid grade cannot cross the wire bound"
        );
        state.vocation = "none".into();
        assert!(grades_for_query(&state).is_none());
        Ok(())
    }
}
