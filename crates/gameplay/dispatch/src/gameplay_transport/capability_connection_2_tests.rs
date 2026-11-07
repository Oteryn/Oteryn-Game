#[test]
fn rune_wire_the_position_arm_is_rejected_for_missing_source_with_or_without_capability_22()
-> Result<(), Box<dyn Error>> {
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
        encode_use_result,
    };
    run(async {
        let authority = NegotiatingAuthority::new(Some(RUNE_USE_OFFERED));
        let at_position = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(ItemHandle::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: Some(SpellTargetPosition {
                x: 100,
                y: -100,
                floor: 7,
            }),
        })
        .map_err(|error| format!("{error:?}"))?;
        let frames = [command(1, COMMAND_TYPE_USE_INTENT, &at_position)];
        let rejected = encode_use_result(UseDisposition::Rejected).unwrap_or_default();
        // Without 22 the arm is undecodable; under 22 the authority refuses a missing source.
        for selected in [
            &[][..],
            &[CAPABILITY_ITEM_VIEW_MOVE_V1],
            &[
                CAPABILITY_ITEM_VIEW_MOVE_V1, // ITEM_VIEW_MOVE_V1
                CAPABILITY_ITEM_USE_V1,       // ITEM_USE_V1
            ],
            &[
                CAPABILITY_ITEM_VIEW_MOVE_V1, // ITEM_VIEW_MOVE_V1
                CAPABILITY_ITEM_USE_V1,       // ITEM_USE_V1
                CAPABILITY_RUNE_USE_V1,       // RUNE_USE_V1
            ],
        ] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let mut results = Vec::new();
            for frame in &written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    results.push((result.command_id, result.status, result.payload.to_vec()));
                }
            }
            assert_eq!(
                results,
                [(1, CommandStatus::Rejected, rejected.clone())],
                "{selected:?}"
            );
        }
        Ok(())
    })
}

/// ITEM-USE-1c: a main backpack entry is used only under capability 15; without it the handle
/// keeps the corpse path and the use is never reached.
#[test]
fn a_backpack_entry_is_used_only_under_capability_15() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::world_object::encode_use_item_intent;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, encode_use_result,
    };
    run(async {
        let authority = NegotiatingAuthority::new(Some(ITEM_USE_OFFERED));
        authority.backpack_entry.set(true);
        let entry = std::num::NonZeroU64::new(2).ok_or("handle")?;
        let frames = [command(
            1,
            COMMAND_TYPE_USE_INTENT,
            &encode_use_item_intent(entry),
        )];
        for (selected, used, disposition) in [
            (
                &[CAPABILITY_ITEM_VIEW_MOVE_V1][..],
                0,
                UseDisposition::StaleState,
            ),
            (
                &[
                    CAPABILITY_ITEM_VIEW_MOVE_V1, // ITEM_VIEW_MOVE_V1
                    CAPABILITY_ITEM_USE_V1,       // ITEM_USE_V1
                ][..],
                1,
                UseDisposition::Full,
            ),
        ] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let mut payloads = Vec::new();
            for frame in &written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    payloads.push(decode_command_result(envelope.payload())?.payload.to_vec());
                }
            }
            assert_eq!(
                payloads,
                [encode_use_result(disposition).unwrap_or_default()],
                "{selected:?}"
            );
            assert_eq!(authority.item_uses.borrow().len(), used, "{selected:?}");
        }
        assert_eq!(
            *authority.item_uses.borrow(),
            [ItemKey::Instance([0xa6; 16])]
        );
        Ok(())
    })
}

// MAP-WIRE-2: capability 18 WORLD_MAP_VIEW_V1 is registered and gated, not offered (MAP-CUTOVER-1
// offers it), and requires 4 and 6; domain 17 is never sent without it.

/// An injected offered set: 4, 6 and 18 requiring both.
const WORLD_MAP_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_MAP_VIEW_V1,
        requires: &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,      // ITEM_VIEW_MOVE_V1
            CAPABILITY_WORLD_SPATIAL_ENTITIES, // WORLD_SPATIAL_ENTITIES
        ],
    },
];

// CHAT-WIRE-1: capability 7 over two connections of one Channel — a say reaches the hearer in
// range as a domain 12 line delta and never the session out of range.

#[path = "capability_chat_wire_tests.rs"]
mod chat_wire;

// LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): command 23 on the connection, behind an
// authority that answers each logout with a scripted reply.

#[path = "capability_logout_wire_tests.rs"]
mod logout_wire;

/// PARITY-EVENT-CALLBACKS-1: capability 75 through the real admission and serve path.
#[path = "capability_server_save_countdown_wire_tests.rs"]
mod server_save_countdown_wire;

/// WEATHER-1 (D1023..D1026, D1448): capability 26 through the real admission and serve path.
/// The source is the shipped Oteryn-designed `content/weather` data, exactly what production
/// loads: production offers 26 once that data loads and fails closed when it does not.
#[path = "capability_weather_wire_tests.rs"]
mod weather_wire;

#[path = "daily_reward_tests.rs"]
mod daily_reward;
#[path = "guild_chat/negotiation_tests.rs"]
mod guild_chat;

#[path = "capability_selection_tests.rs"]
mod selection_tests;
