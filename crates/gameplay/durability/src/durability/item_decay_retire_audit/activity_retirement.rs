//! Additive exact-Instance cause. Existing Channel payloads omit tag 9.
use super::*;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemActivityRetirementV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub handoff_id: Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub ownership_generation: u64,
    #[prost(bytes = "vec", tag = "5")]
    pub item_instance_id: Vec<u8>,
}

pub(super) fn check(value: &OneItemDecayRetireV1) -> Result<DecayRetireShape, AuditError> {
    let cause = value
        .activity_retirement
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let before = value.before.as_ref().ok_or(AuditError::InvalidInput)?;
    let after = value.after.as_ref().ok_or(AuditError::InvalidInput)?;
    check_state(before)?;
    check_state(after)?;
    mint::check_key_number(before)?;
    check_uuid_v7(&cause.world_id)?;
    check_uuid_v7(&cause.instance_id)?;
    check_uuid_v7(&cause.handoff_id)?;
    check_uuid_v7(&cause.item_instance_id)?;
    let ground = scope_ground(value)?;
    check_ground(ground)?;
    if value.cause.is_some()
        || value.world_reset.is_some()
        || value.corpse_entry.is_some()
        || before.item_instance_id != after.item_instance_id
        || before.world_id != after.world_id
        || before.definition != after.definition
        || before.key_number != after.key_number
        || before.lifecycle != ITEM_LIFECYCLE_LIVE
        || before.quantity == 0
        || after.lifecycle != ITEM_LIFECYCLE_RETIRED
        || after.quantity != 0
        || cause.world_id != before.world_id
        || ground.world_id != before.world_id
        || cause.item_instance_id != before.item_instance_id
        || ground.instance_id.as_ref() != Some(&cause.instance_id)
        || !ground.channel_id.is_empty()
        || cause.ownership_generation == 0
        || cause.ownership_generation > value.runtime_scope_ownership_generation
        || cause.ownership_generation != ground.runtime_scope_ownership_generation
    {
        return Err(AuditError::InvalidInput);
    }
    DECAY_RETIRE_USAGE.check()?;
    if let Some(entry) = &value.ground_container_entry {
        check_uuid_v7(&entry.parent_item_instance_id)?;
        if entry.parent_item_instance_id == before.item_instance_id
            || !(1..=mint::GAMEITEM01_CORPSE_CONTAINER_ENTRIES_MAX)
                .contains(&entry.placement_ordinal)
        {
            return Err(AuditError::InvalidInput);
        }
        Ok(DecayRetireShape::GroundContainerEntry)
    } else {
        Ok(DecayRetireShape::Ground)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use crate::foundation::{RuntimeScopeRefV1, WorldId};

    fn id(tag: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16];
        bytes[5] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = tag;
        bytes
    }
    fn sample() -> OneItemDecayRetireV1 {
        let definition = mint::OneItemTypedDefinitionRevisionV1 {
            family: "ItemType".into(),
            production_key: "fixture:retired-ground".into(),
            revision_ref: "rev-1".into(),
        };
        let state = |quantity, lifecycle| OneItemStateV1 {
            item_instance_id: id(9),
            world_id: id(1),
            definition: Some(definition.clone()),
            quantity,
            lifecycle,
            key_number: 0,
        };
        let ground = OneItemGroundV1 {
            world_id: id(1),
            channel_id: Vec::new(),
            instance_id: Some(id(2)),
            spatial_position: vec![9, 9, 0],
            corpse_ref: id(3),
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            native_room_placement_context: id(6),
            runtime_scope_ownership_generation: 7,
        };
        OneItemDecayRetireV1 {
            before: Some(state(3, ITEM_LIFECYCLE_LIVE)),
            after: Some(state(0, ITEM_LIFECYCLE_RETIRED)),
            ground: Some(ground),
            corpse_entry: None,
            cause: None,
            runtime_scope_ownership_generation: 7,
            world_reset: None,
            ground_container_entry: None,
            activity_retirement: Some(OneItemActivityRetirementV1 {
                world_id: id(1),
                instance_id: id(2),
                handoff_id: id(4),
                ownership_generation: 7,
                item_instance_id: id(9),
            }),
        }
    }

    #[test]
    fn activity_retirement_round_trips_in_its_real_instance_envelope() {
        let value = sample();
        assert_eq!(check_decay_retire(&value), Ok(DecayRetireShape::Ground));
        let wire = encode_decay_retire_event(
            DecayRetireEventIdentity {
                event_id: id(30).try_into().unwrap(),
                transaction_id: id(40).try_into().unwrap(),
                occurred_at_unix_ms: 1_790_000_060_000,
                server_build_id: "oteryn-game-server/test",
            },
            value.clone(),
            mint::Type2EventTuple::V1,
        )
        .unwrap();
        let (envelope, decoded) = decode_decay_retire_envelope(&wire).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(envelope.instance_id, Some(id(2)));
        assert!(envelope.channel_id.is_none());
        assert!(envelope.game_session_id.is_none());
        assert!(envelope.command_id.is_none());
        let mut aliased = envelope;
        aliased.channel_id = aliased.instance_id.take();
        assert!(decode_decay_retire_envelope(&aliased.encode_to_vec()).is_err());
    }

    #[test]
    fn activity_retirement_cannot_claim_a_channel_or_another_generation() {
        let alterations: [fn(&mut OneItemDecayRetireV1); 6] = [
            |v: &mut OneItemDecayRetireV1| {
                v.ground.as_mut().unwrap().channel_id = id(2);
            },
            |v: &mut OneItemDecayRetireV1| {
                v.ground.as_mut().unwrap().instance_id = None;
                v.ground.as_mut().unwrap().channel_id = id(2);
            },
            |v: &mut OneItemDecayRetireV1| {
                v.activity_retirement.as_mut().unwrap().instance_id = id(8);
            },
            |v: &mut OneItemDecayRetireV1| {
                v.activity_retirement.as_mut().unwrap().ownership_generation = 8;
            },
            |v: &mut OneItemDecayRetireV1| {
                v.after.as_mut().unwrap().quantity = 1;
            },
            |v: &mut OneItemDecayRetireV1| {
                v.cause = Some(OneItemCorpseDecayV1 {
                    corpse_item_instance_id: id(9),
                    deadline_unix_ms: 60_000,
                });
            },
        ];
        for alter in alterations {
            let mut value = sample();
            alter(&mut value);
            assert!(check_decay_retire(&value).is_err());
        }
        let mut entry = sample();
        entry.ground_container_entry = Some(OneItemGroundContainerEntryV1 {
            parent_item_instance_id: id(8),
            placement_ordinal: 1,
            parent_ground: entry.ground.take(),
        });
        assert_eq!(
            check_decay_retire(&entry),
            Ok(DecayRetireShape::GroundContainerEntry)
        );
    }

    #[test]
    fn channel_decay_omits_the_new_cause_and_preserves_legacy_wire() {
        #[derive(Clone, PartialEq, Message)]
        struct LegacyDecay {
            #[prost(message, optional, tag = "1")]
            before: Option<OneItemStateV1>,
            #[prost(message, optional, tag = "2")]
            after: Option<OneItemStateV1>,
            #[prost(message, optional, tag = "3")]
            ground: Option<OneItemGroundV1>,
            #[prost(message, optional, tag = "4")]
            corpse_entry: Option<OneItemCorpseSourceV1>,
            #[prost(message, optional, tag = "5")]
            cause: Option<OneItemCorpseDecayV1>,
            #[prost(uint64, tag = "6")]
            runtime_scope_ownership_generation: u64,
            #[prost(message, optional, tag = "7")]
            world_reset: Option<OneItemWorldResetV1>,
            #[prost(message, optional, tag = "8")]
            ground_container_entry: Option<OneItemGroundContainerEntryV1>,
        }
        let mut value = sample();
        value.activity_retirement = None;
        value.cause = Some(OneItemCorpseDecayV1 {
            corpse_item_instance_id: id(9),
            deadline_unix_ms: 60_000,
        });
        let scope = RuntimeScopeRefV1::channel(
            WorldId::decode(&id(1)).unwrap(),
            crate::foundation::ChannelId::decode(&id(2)).unwrap(),
        );
        value.ground = value.ground.take().map(|g| g.with_runtime_scope(scope));
        assert_eq!(check_decay_retire(&value), Ok(DecayRetireShape::Corpse));
        let wire = value.encode_to_vec();
        let legacy = LegacyDecay::decode(wire.as_slice()).unwrap();
        assert_eq!(legacy.encode_to_vec(), wire);
        assert_eq!(
            OneItemDecayRetireV1::decode(legacy.encode_to_vec().as_slice()).unwrap(),
            value
        );
    }
}
