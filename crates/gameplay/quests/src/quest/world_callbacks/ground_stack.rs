//! Read-only ordered Ground prerequisites; these never authorize an Item debit.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundStackEntry {
    pub item: String,
    pub count: u32,
}

/// The native custody owner supplies the instance identity, not the client.
pub struct GroundStackFact<'a> {
    pub instance: [u8; 16],
    pub item: &'a str,
    pub count: u32,
}

pub(super) fn valid(required: &[GroundStackEntry]) -> bool {
    !required.is_empty()
        && required.len() <= super::MAX_EFFECTS
        && required.iter().all(|entry| {
            entry.item.starts_with("oteryn:item.")
                && super::super::valid_quest_key(&entry.item)
                && (1..=100).contains(&entry.count)
        })
}

/// Compare the complete sequence. Unknown/volatile roots, duplicate instances,
/// surplus roots and aggregated quantities cannot stand in for authored ordinals.
pub fn ground_stack_matches<'a>(
    required: &[GroundStackEntry],
    mut observed: impl Iterator<Item = Option<GroundStackFact<'a>>>,
) -> bool {
    if !valid(required) {
        return false;
    }
    let mut instances = BTreeSet::new();
    for expected in required {
        let Some(Some(actual)) = observed.next() else {
            return false;
        };
        if actual.instance == [0; 16]
            || !instances.insert(actual.instance)
            || actual.item != expected.item
            || actual.count != expected.count
        {
            return false;
        }
    }
    observed.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::super::WorldRequirement;
    use super::*;

    #[test]
    fn srv7_m02_ordered_ground_stack_is_an_explicit_world_predicate() -> Result<(), String> {
        let requirement: WorldRequirement = serde_json::from_value(serde_json::json!({
            "position": {"x": 1, "y": 2, "floor": -7},
            "appearance": 2772,
            "ground_stack": [
                {"item": "oteryn:item.tibia.i1739", "count": 1},
                {"item": "oteryn:item.tibia.i1739", "count": 1},
                {"item": "oteryn:item.tibia.i2025", "count": 1}
            ]
        }))
        .map_err(|e| e.to_string())?;
        assert_eq!(
            serde_json::to_value(requirement).map_err(|e| e.to_string())?["ground_stack"]
                .as_array()
                .map(Vec::len),
            Some(3)
        );
        Ok(())
    }

    #[test]
    fn srv7_m02_ground_stack_preserves_order_quantity_and_custody_identity() {
        let required = [
            GroundStackEntry {
                item: "oteryn:item.crate".into(),
                count: 1,
            },
            GroundStackEntry {
                item: "oteryn:item.crate".into(),
                count: 1,
            },
            GroundStackEntry {
                item: "oteryn:item.amphora".into(),
                count: 1,
            },
        ];
        let facts = |items: &[(&'static str, u32, u8)]| {
            items
                .iter()
                .map(|(item, count, id)| {
                    Some(GroundStackFact {
                        instance: [*id; 16],
                        item,
                        count: *count,
                    })
                })
                .collect::<Vec<_>>()
        };
        let correct = [
            ("oteryn:item.crate", 1, 1),
            ("oteryn:item.crate", 1, 2),
            ("oteryn:item.amphora", 1, 3),
        ];
        assert!(ground_stack_matches(&required, facts(&correct).into_iter()));
        for wrong in [
            vec![correct[2], correct[0], correct[1]],
            vec![("oteryn:item.crate", 2, 1), correct[2]],
            vec![correct[0], correct[0], correct[2]],
            vec![correct[0], correct[1]],
            vec![correct[0], correct[1], correct[2], correct[2]],
            vec![correct[0], correct[1], ("oteryn:item.lookalike", 1, 3)],
            vec![correct[0], correct[1], ("oteryn:item.amphora", 1, 0)],
        ] {
            assert!(!ground_stack_matches(&required, facts(&wrong).into_iter()));
        }
        assert!(!ground_stack_matches(
            &required,
            [None, None, None].into_iter()
        ));
        assert!(!ground_stack_matches(&[], std::iter::empty()));
    }
}
