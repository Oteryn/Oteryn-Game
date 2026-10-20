#![allow(clippy::expect_used)]

use super::*;
use crate::foundation::{CommandId, GameSessionId};

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn plan() -> RewardClaimInboxPlan {
    RewardClaimInboxPlan {
        command: CommandRef::new(
            GameSessionId::decode(&id(50)).expect("session"),
            CommandId::new(1).expect("command"),
        ),
        character_id: CharacterId::from_bytes(id(41)).expect("character"),
        world_id: WorldId::decode(&id(42)).expect("world"),
        channel_id: ChannelId::decode(&id(43)).expect("channel"),
        transaction_id: id(70),
        source_binding: [5; 32],
        item_instance_ids: vec![id(71), id(72)],
    }
}

#[test]
fn plan_refuses_empty_oversize_duplicate_and_malformed_frozen_ids() {
    let mut value = plan();
    value.item_instance_ids = (1..=20).map(id).collect();
    assert!(value.validate().is_ok());
    value.item_instance_ids.push(id(21));
    assert!(matches!(
        value.validate(),
        Err(RewardClaimInboxError::InvalidPlan)
    ));
    value.item_instance_ids.clear();
    assert!(matches!(
        value.validate(),
        Err(RewardClaimInboxError::InvalidPlan)
    ));
    value.item_instance_ids = vec![id(71), id(71)];
    assert!(matches!(
        value.validate(),
        Err(RewardClaimInboxError::InvalidPlan)
    ));
    value.item_instance_ids = vec![[0; 16]];
    assert!(matches!(
        value.validate(),
        Err(RewardClaimInboxError::InvalidPlan)
    ));
    value.item_instance_ids = vec![id(71)];
    value.transaction_id = [0; 16];
    assert!(matches!(
        value.validate(),
        Err(RewardClaimInboxError::InvalidPlan)
    ));
}

#[test]
fn replay_binding_covers_each_scope_source_and_output_component() {
    let original = plan();
    let binding = original.binding();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed.command = CommandRef::new(
        GameSessionId::decode(&id(51)).expect("session"),
        original.command.command_id(),
    );
    variants.push(changed);
    let mut changed = original.clone();
    changed.command = CommandRef::new(
        original.command.game_session_id(),
        CommandId::new(u64::MAX).expect("command"),
    );
    variants.push(changed);
    let mut changed = original.clone();
    changed.character_id = CharacterId::from_bytes(id(44)).expect("character");
    variants.push(changed);
    let mut changed = original.clone();
    changed.world_id = WorldId::decode(&id(45)).expect("world");
    variants.push(changed);
    let mut changed = original.clone();
    changed.channel_id = ChannelId::decode(&id(46)).expect("channel");
    variants.push(changed);
    let mut changed = original.clone();
    changed.transaction_id = id(73);
    variants.push(changed);
    let mut changed = original.clone();
    changed.source_binding[31] ^= 1;
    variants.push(changed);
    let mut changed = original.clone();
    changed.item_instance_ids = vec![id(73), id(74)];
    variants.push(changed);
    let mut changed = original.clone();
    changed.item_instance_ids.reverse();
    variants.push(changed);
    let mut changed = original.clone();
    changed.item_instance_ids.pop();
    variants.push(changed);
    for changed in variants {
        assert!(changed.validate().is_ok());
        assert_ne!(
            changed.binding(),
            binding,
            "changed intent must not replay: {changed:?}"
        );
    }
    assert_eq!(original.clone().binding(), binding);
}
