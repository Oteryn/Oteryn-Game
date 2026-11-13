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
fn mutable_ground_schema_rejects_ambiguous_and_unbounded_conditions()
-> Result<(), serde_json::Error> {
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
    let mixed = serde_json::json!({"position":{"x":1,"y":1,"floor":-7},"appearance":3031,"ground":{"stacks":[row.clone()]}});
    assert!(serde_json::from_value::<WorldRequirement>(mixed).is_err());
    let ground =
        serde_json::json!({"position":{"x":1,"y":1,"floor":-7},"ground":{"stacks":[row.clone()]}});
    let parsed: WorldRequirement = serde_json::from_value(ground.clone())?;
    assert_eq!(serde_json::to_value(&parsed)?, ground);
    for mutation in 0..5 {
        let mut invalid = ground.clone();
        match mutation {
            0 => invalid["ground"]["stacks"] = serde_json::json!([]),
            1 => invalid["ground"]["stacks"] = serde_json::json!(vec![row.clone(); 9]),
            2 => invalid["ground_stack"] = serde_json::json!([{"item":row.item,"count":1}]),
            3 => invalid["appearance"] = serde_json::json!(3031),
            _ => invalid["unexpected"] = serde_json::json!(true),
        }
        assert!(
            serde_json::from_value::<WorldRequirement>(invalid).is_err(),
            "invalid Ground shape {mutation}"
        );
    }
    // Existing exact static-anchor plus ordered native stack remains admitted.
    let anchored = serde_json::json!({"position":{"x":1,"y":1,"floor":-7},"appearance":3031,"ground_stack":[{"item":"oteryn:item.tibia.i3031","count":1}]});
    let parsed: WorldRequirement = serde_json::from_value(anchored.clone())?;
    assert_eq!(serde_json::to_value(parsed)?, anchored);
    Ok(())
}

#[test]
fn mutable_ground_evidence_cannot_be_stripped_into_a_generic_quest_write() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::{admission_binding, world_callbacks};
    let mut callback = world_callbacks::load()?
        .into_iter()
        .find(|c| {
            !c.storage.is_empty()
                && c.native_rule.is_none()
                && !c.requires_world_admission
                && !c.opening_only
                && c.storage.iter().any(|s| !s.preserve)
        })
        .ok_or("ordinary world callback fixture")?;
    callback.key = "oteryn:quest-callback/engenc2/ground-admission-fixture".into();
    callback.storage.truncate(1);
    callback.storage[0].track = "oteryn:quest-progress/engenc2/ground-admission-fixture".into();
    callback.storage[0].preserve = false;
    callback.storage[0].increment = false;
    callback.storage[0].at_least = false;
    callback.storage[0].set = 1;
    callback.world_requirements.clear();
    assert_eq!(
        world_callbacks::gated_binding(&callback)?,
        None,
        "legacy callbacks retain their original binding behavior"
    );
    callback.world_requirements.push(serde_json::from_value(serde_json::json!({
        "position":{"x":33216,"y":32854,"floor":-9},
        "ground":{"stacks":[{"item":"oteryn:item.tibia.i2471","revision":"definition-r1","quantity":1}]}
    })).map_err(|e| e.to_string())?);
    let transition = world_callbacks::primary_transition(&callback)?;
    assert!(
        admission_binding::validate(&transition, false).is_err(),
        "a renamed generic transition must not bypass physical Ground evidence"
    );
    let binding = world_callbacks::gated_binding(&callback)?.ok_or("missing Ground binding")?;
    callback.world_requirements[0]=serde_json::from_value(serde_json::json!({
        "position":{"x":33216,"y":32854,"floor":-9},
        "ground":{"stacks":[{"item":"oteryn:item.tibia.i2471","revision":"definition-r1","quantity":2}]}
    })).map_err(|e|e.to_string())?;
    assert_ne!(
        world_callbacks::gated_binding(&callback)?,
        Some(binding),
        "physical quantity belongs to the exact admission binding"
    );
    Ok(())
}
