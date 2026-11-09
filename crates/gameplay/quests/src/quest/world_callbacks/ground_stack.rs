//! Read-only adapter to the shared QMECH ordered Ground rule owner.
use super::super::ground_stack::{
    GroundStackEntry as NativeGroundStackFact, GroundStackRequirement, GroundStackSnapshot,
    StackItemRequirement, match_stack,
};
use super::Position;
use serde::{Deserialize, Serialize};

/// Stable M02 wire shape, shared with the durable Ground participant.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundStackEntry {
    pub item: String,
    pub count: u32,
}

/// The caller proves exact custody and definition revision before supplying facts.
pub struct GroundStackFact<'a> {
    pub instance: [u8; 16],
    pub item: &'a str,
    pub count: u32,
}

fn requirement(position: Position, required: &[GroundStackEntry]) -> GroundStackRequirement {
    GroundStackRequirement {
        position,
        items: required
            .iter()
            .map(|r| StackItemRequirement {
                item: r.item.clone(),
                revision: "definition-r1".into(),
                quantity: r.count,
            })
            .collect(),
    }
}

pub(super) fn valid(required: &[GroundStackEntry]) -> bool {
    !required.is_empty()
        && required.len() <= super::MAX_EFFECTS
        && required.iter().all(|r| r.count <= 100)
        && requirement(
            Position {
                x: 0,
                y: 0,
                floor: 0,
            },
            required,
        )
        .validate()
        .is_ok()
}

/// Compare native facts with their actual append ordinals and definition revisions.
/// A readset is never a write permit; the durable participant retains its own locks.
pub fn ground_stack_matches_at(
    position: Position,
    required: &[GroundStackEntry],
    mut observed: impl Iterator<Item = Option<NativeGroundStackFact>>,
) -> bool {
    if !valid(required) {
        return false;
    }
    let mut entries = Vec::with_capacity(required.len());
    for _ in required {
        let Some(Some(actual)) = observed.next() else {
            return false;
        };
        if actual.instance == [0; 16] {
            return false;
        }
        entries.push(actual);
    }
    if observed.next().is_some() {
        return false;
    }
    match_stack(
        &requirement(position, required),
        &GroundStackSnapshot { position, entries },
    )
    .is_ok_and(|witness| witness.is_some())
}

/// Compatibility for already-qualified ordered observations. Global position,
/// revision and custody are proved by the caller; this compares the local sequence.
pub fn ground_stack_matches<'a>(
    required: &[GroundStackEntry],
    observed: impl Iterator<Item = Option<GroundStackFact<'a>>>,
) -> bool {
    ground_stack_matches_at(
        Position {
            x: 0,
            y: 0,
            floor: 0,
        },
        required,
        observed.enumerate().map(|(ordinal, fact)| {
            let fact = fact?;
            Some(NativeGroundStackFact {
                instance: fact.instance,
                item: fact.item.into(),
                revision: "definition-r1".into(),
                quantity: fact.count,
                ordinal: u64::try_from(ordinal).ok()?.checked_add(1)?,
            })
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::super::WorldRequirement;
    use super::*;

    #[test]
    fn srv7_m02_mutable_ground_projection_requires_durable_world_admission() -> Result<(), String> {
        let mut callback = super::super::load()?
            .into_iter()
            .find(|c| c.key.ends_with("/behemoth"))
            .ok_or("callback template")?;
        callback.key = "oteryn:quest-callback/srv7-m02-readonly-fixture".into();
        callback.storage.clear();
        callback.state_ops.clear();
        callback.effects = Default::default();
        callback.minimum_level = None;
        callback.experience = None;
        callback.monk_reward = None;
        callback.native_rule = None;
        callback.offerings.clear();
        callback.item_on_target = None;
        callback.requires_world_admission = false;
        callback.opening_only = false;
        callback.reversible = false;
        callback.world_requirements = vec![super::super::WorldRequirement {
            ground: None,
            position: Position {
                x: 1,
                y: 2,
                floor: -7,
            },
            appearance: 2772,
            ground_stack: Some(vec![GroundStackEntry {
                item: "oteryn:item.fixture_crate".into(),
                count: 1,
            }]),
        }];
        let encode = |c: &super::super::Callback| {
            serde_json::json!({"schema":1,"callbacks":[c]}).to_string()
        };
        super::super::parse(&encode(&callback))?;
        callback.storage.push(super::super::Storage {
            track: "oteryn:quest-progress/srv7-m02-fixture".into(),
            minimum: 0,
            maximum: 0,
            set: 1,
            preserve: false,
            increment: false,
            at_least: false,
        });
        assert!(
            super::super::parse(&encode(&callback)).is_err(),
            "a read-only projection cannot authorize an unfenced personal write"
        );
        callback.requires_world_admission = true;
        super::super::parse(&encode(&callback))?;
        assert!(
            super::super::gated_binding(&callback)?.is_some(),
            "personal Ground predicates require the registered fenced TX participant"
        );
        Ok(())
    }

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
                .enumerate()
                .map(|(ordinal, (item, count, id))| {
                    Some(NativeGroundStackFact {
                        instance: [*id; 16],
                        item: (*item).into(),
                        revision: "definition-r1".into(),
                        quantity: *count,
                        ordinal: u64::try_from(ordinal + 1).unwrap_or(u64::MAX),
                    })
                })
                .collect::<Vec<_>>()
        };
        let correct = [
            ("oteryn:item.crate", 1, 1),
            ("oteryn:item.crate", 1, 2),
            ("oteryn:item.amphora", 1, 3),
        ];
        assert!(ground_stack_matches_at(
            super::super::Position {
                x: 1,
                y: 2,
                floor: -7
            },
            &required,
            facts(&correct).into_iter()
        ));
        for wrong in [
            vec![correct[2], correct[0], correct[1]],
            vec![("oteryn:item.crate", 2, 1), correct[2]],
            vec![correct[0], correct[0], correct[2]],
            vec![correct[0], correct[1]],
            vec![correct[0], correct[1], correct[2], correct[2]],
            vec![correct[0], correct[1], ("oteryn:item.lookalike", 1, 3)],
            vec![correct[0], correct[1], ("oteryn:item.amphora", 1, 0)],
        ] {
            assert!(!ground_stack_matches_at(
                super::super::Position {
                    x: 1,
                    y: 2,
                    floor: -7
                },
                &required,
                facts(&wrong).into_iter()
            ));
        }
        assert!(!ground_stack_matches_at(
            super::super::Position {
                x: 1,
                y: 2,
                floor: -7
            },
            &required,
            [None, None, None].into_iter()
        ));
        assert!(!ground_stack_matches_at(
            super::super::Position {
                x: 1,
                y: 2,
                floor: -7
            },
            &[],
            std::iter::empty()
        ));
    }
}
