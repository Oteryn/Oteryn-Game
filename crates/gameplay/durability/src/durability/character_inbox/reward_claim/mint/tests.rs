#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::foundation::{CommandId, GameSessionId};

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    bytes
}

fn request() -> RewardClaimInboxMintRequest {
    RewardClaimInboxMintRequest {
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "daily-fixture".into(),
            revision_ref: "r1".into(),
        },
        content_revision: "content-r1".into(),
        ruleset_revision: "rules-r1".into(),
        sim_revision: "sim-r1".into(),
        outputs: vec![RewardClaimInboxMintOutput {
            source_line: 0,
            item: ItemDefinitionFacts {
                weight: None,
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "item-fixture".into(),
                    revision_ref: "r1".into(),
                },
                stack: ItemStackClass::Stackable {
                    proven_maximum: Some(10),
                },
                container_capacity: None,
                container_slot_equip_pattern: false,
            },
            quantity: 3,
            key_number: None,
        }],
    }
}

fn plan() -> RewardClaimInboxPlan {
    RewardClaimInboxPlan {
        command: CommandRef::new(
            GameSessionId::decode(&id(1)).unwrap(),
            CommandId::new(u64::MAX).unwrap(),
        ),
        character_id: CharacterId::from_bytes(id(2)).unwrap(),
        world_id: WorldId::decode(&id(3)).unwrap(),
        channel_id: ChannelId::decode(&id(4)).unwrap(),
        transaction_id: id(5),
        source_binding: [7; 32],
        item_instance_ids: vec![id(6)],
    }
}

#[test]
fn mint_spec_and_full_plan_bind_every_admitted_output_fact_and_source_revision() {
    let original = request();
    let source_plan = plan();
    let spec = mint_spec(&original).unwrap();
    assert_eq!(mint_spec(&original).unwrap(), spec);
    let binding = bound_plan(&source_plan, &spec).binding();
    type Mutation = Box<dyn Fn(&mut RewardClaimInboxMintRequest)>;
    for mutate in [
        Box::new(|r: &mut RewardClaimInboxMintRequest| r.claim.family.push('x')) as Mutation,
        Box::new(|r| r.claim.production_key.push('x')),
        Box::new(|r| r.claim.revision_ref.push('x')),
        Box::new(|r| r.content_revision.push('x')),
        Box::new(|r| r.ruleset_revision.push('x')),
        Box::new(|r| r.sim_revision.push('x')),
        Box::new(|r| r.outputs[0].source_line = 1),
        Box::new(|r| r.outputs[0].item.definition.family.push('x')),
        Box::new(|r| r.outputs[0].item.definition.production_key.push('x')),
        Box::new(|r| r.outputs[0].item.definition.revision_ref.push('x')),
        Box::new(|r| r.outputs[0].quantity = 4),
        Box::new(|r| {
            r.outputs[0].item.stack = ItemStackClass::Stackable {
                proven_maximum: Some(11),
            }
        }),
    ] {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_ne!(
            bound_plan(&source_plan, &mint_spec(&changed).unwrap()).binding(),
            binding
        );
    }
    let mut changed_plan = source_plan.clone();
    changed_plan.source_binding[31] ^= 1;
    assert_ne!(bound_plan(&changed_plan, &spec).binding(), binding);
    assert_eq!(source_plan.source_binding, [7; 32]);
    assert_eq!(source_plan.command.command_id().get(), u64::MAX);

    let mut key = original;
    key.outputs[0].item.stack = ItemStackClass::NonStackable;
    key.outputs[0].quantity = 1;
    let unkeyed = mint_spec(&key).unwrap();
    key.outputs[0].key_number = Some(65535);
    assert_ne!(
        bound_plan(&source_plan, &unkeyed).binding(),
        bound_plan(&source_plan, &mint_spec(&key).unwrap()).binding()
    );
}

#[test]
fn mint_outputs_refuse_unknown_stacks_overflow_containers_and_invalid_keys() {
    type Mutation = Box<dyn Fn(&mut RewardClaimInboxMintRequest)>;
    for mutate in [
        Box::new(|r: &mut RewardClaimInboxMintRequest| r.outputs.clear()) as Mutation,
        Box::new(|r| r.outputs = vec![r.outputs[0].clone(); 21]),
        Box::new(|r| r.outputs[0].quantity = 0),
        Box::new(|r| r.outputs[0].quantity = 11),
        Box::new(|r| r.outputs[0].item.stack = ItemStackClass::Unknown),
        Box::new(|r| {
            r.outputs[0].item.stack = ItemStackClass::Stackable {
                proven_maximum: Some(0),
            }
        }),
        Box::new(|r| {
            r.outputs[0].item.stack = ItemStackClass::Stackable {
                proven_maximum: Some(101),
            }
        }),
        Box::new(|r| r.outputs[0].item.stack = ItemStackClass::NonStackable),
        Box::new(|r| r.outputs[0].item.container_capacity = Some(20)),
        Box::new(|r| r.outputs[0].item.container_slot_equip_pattern = true),
        Box::new(|r| r.outputs[0].key_number = Some(0)),
        Box::new(|r| r.outputs[0].key_number = Some(1)),
        Box::new(|r| r.outputs[0].item.definition.production_key.push('\n')),
    ] {
        let mut changed = request();
        mutate(&mut changed);
        assert!(mint_spec(&changed).is_err(), "{changed:?}");
    }
}

#[test]
fn maximum_twenty_output_spec_fits_even_with_maximum_json_escaping() {
    let mut value = request();
    value.claim = TypedDefinitionRef {
        family: "\\".repeat(128),
        production_key: "\"".repeat(512),
        revision_ref: "\\".repeat(512),
    };
    value.content_revision = "\"".repeat(512);
    value.ruleset_revision = "\\".repeat(512);
    value.sim_revision = "\"".repeat(512);
    value.outputs[0].item.definition = value.claim.clone();
    value.outputs[0].quantity = 100;
    value.outputs[0].item.stack = ItemStackClass::Stackable {
        proven_maximum: None,
    };
    value.outputs[0].source_line = u32::MAX;
    value.outputs = vec![value.outputs[0].clone(); 20];
    let spec = mint_spec(&value).unwrap();
    assert!(spec.len() <= 65_536);
    eprintln!(
        "reward Inbox native canonical JSON cap measurement: outputs=20 spec_bytes={} spec_cap=65536",
        spec.len()
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&spec).unwrap()["outputs"]
            .as_array()
            .unwrap()
            .len(),
        20
    );
    value.outputs.push(value.outputs[0].clone());
    assert!(matches!(
        mint_spec(&value),
        Err(RewardClaimInboxError::CapacityExceeded)
    ));
}
