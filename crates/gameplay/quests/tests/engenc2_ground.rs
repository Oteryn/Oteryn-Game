use oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement;

#[test]
fn mutable_ground_stack_has_a_separate_typed_requirement() {
    let value = serde_json::json!({
        "position": {"x": 100, "y": 101, "floor": -7},
        "ground": {"stacks": [
            {"item": "oteryn:item.tibia.i3031", "revision": "definition-r1", "quantity": 1}
        ]}
    });
    let result = serde_json::from_value::<WorldRequirement>(value);
    assert!(
        result.is_ok(),
        "mutable Ground prerequisites are not represented: {result:?}"
    );
}

#[test]
fn mutable_ground_requires_exact_quantity_order_instance_and_revision() {
    use oteryn_gameplay_quests::quest::world_callbacks::ground::{
        GroundStackItem, GroundStackObserved, GroundStackRequirement,
    };
    let required = GroundStackRequirement {
        stacks: vec![
            GroundStackItem {
                item: "oteryn:item.tibia.i3031".into(),
                revision: "definition-r1".into(),
                quantity: 2,
            },
            GroundStackItem {
                item: "oteryn:item.tibia.i3035".into(),
                revision: "definition-r1".into(),
                quantity: 1,
            },
        ],
    };
    let rows = vec![
        GroundStackObserved {
            instance: [1; 16],
            item: "oteryn:item.tibia.i3031",
            revision: "definition-r1",
            quantity: 2,
            ordinal: Some(10),
        },
        GroundStackObserved {
            instance: [2; 16],
            item: "oteryn:item.tibia.i3035",
            revision: "definition-r1",
            quantity: 1,
            ordinal: Some(20),
        },
    ];
    assert!(required.matches(&rows));
    for mutation in 0..8 {
        let mut changed = rows.clone();
        match mutation {
            0 => changed[0].quantity = 3,
            1 => changed.swap(0, 1),
            2 => changed[1].instance = changed[0].instance,
            3 => changed[0].ordinal = None,
            4 => changed[0].ordinal = Some(0),
            5 => changed[1].ordinal = Some(10),
            6 => changed[1].revision = "definition-r2",
            _ => changed.push(rows[0].clone()),
        }
        assert!(
            !required.matches(&changed),
            "accepted mutable Ground mismatch {mutation}"
        );
    }
}

#[test]
fn mutable_ground_schema_rejects_ambiguous_and_unbounded_conditions() {
    use oteryn_gameplay_quests::quest::world_callbacks::ground::{
        GroundStackItem, GroundStackRequirement,
    };
    let row = GroundStackItem {
        item: "oteryn:item.tibia.i3031".into(),
        revision: "definition-r1".into(),
        quantity: 1,
    };
    assert!(!GroundStackRequirement { stacks: vec![] }.valid());
    assert!(
        !GroundStackRequirement {
            stacks: vec![row.clone(); 9]
        }
        .valid()
    );
    let mixed = serde_json::json!({"position":{"x":1,"y":1,"floor":-7},"appearance":3031,"ground":{"stacks":[row]}});
    assert!(serde_json::from_value::<WorldRequirement>(mixed).is_err());
}
