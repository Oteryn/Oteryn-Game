use super::*;
use crate::durability::DurabilityError;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

fn session(byte: u8) -> GameSessionId {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70 | (byte & 0x0f);
    bytes[8] = 0x80 | (byte & 0x3f);
    GameSessionId::decode(&bytes).expect("UUIDv7 session")
}

fn intent() -> NpcTradeIntent {
    NpcTradeIntent {
        npc_actor: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        catalogue_revision: 9,
        offer_index: 2,
        side: NpcTradeSide::Buy,
        quantity: 3,
        expected_unit_price: 40,
    }
}

fn receipt() -> NpcTradeReceipt {
    NpcTradeReceipt {
        npc_key: "npc.alesar".to_owned(),
        catalogue_revision: 9,
        offer_index: 2,
        side: npc_trade::NpcTradeSide::Buy,
        quantity: 3,
        unit_price: 40,
    }
}

#[test]
fn identities_are_uuid_v7_fixed_per_command_and_distinct_per_tag() {
    let (first, second) = (session(1), session(2));
    let npc = intent().npc_actor;
    let occurrence_of = occurrence(first, 5, npc);
    assert_eq!(
        occurrence_of,
        occurrence(first, 5, npc),
        "a retry reuses it"
    );
    assert_ne!(occurrence_of, occurrence(first, 6, npc));
    assert_ne!(occurrence_of, occurrence(second, 5, npc));
    let other_npc = EntityRef {
        identity: [9; 16],
        ..npc
    };
    let next_generation = EntityRef {
        generation: npc.generation + 1,
        ..npc
    };
    assert_ne!(
        occurrence_of,
        occurrence(first, 5, other_npc),
        "bound to the NPC"
    );
    assert_ne!(
        occurrence_of,
        occurrence(first, 5, next_generation),
        "and to its generation"
    );
    let ids = [
        occurrence_of,
        derived(b"oteryn:npc-trade-transaction:v1", first, 5),
        derived(b"oteryn:npc-trade-event:v1", first, 5),
        derived(b"oteryn:npc-trade-output-1:v1", first, 5),
        derived(b"oteryn:npc-trade-output-2:v1", first, 5),
        derived(b"oteryn:npc-trade-output-3:v1", first, 5),
    ];
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(id[6] >> 4, 7, "version 7");
        assert_eq!(id[8] >> 6, 0b10, "RFC 4122 variant");
        assert!(!ids[..index].contains(id), "distinct per tag");
    }
}

#[test]
fn replay_answers_only_the_intent_it_committed() {
    assert!(answers(&receipt(), &intent()));
    let changed = [
        NpcTradeIntent {
            catalogue_revision: 10,
            ..intent()
        },
        NpcTradeIntent {
            offer_index: 1,
            ..intent()
        },
        NpcTradeIntent {
            side: NpcTradeSide::Sell,
            ..intent()
        },
        NpcTradeIntent {
            quantity: 4,
            ..intent()
        },
        NpcTradeIntent {
            expected_unit_price: 41,
            ..intent()
        },
    ];
    for intent in changed {
        assert!(!answers(&receipt(), &intent), "{intent:?}");
    }
}

#[test]
fn only_a_storage_failure_leaves_the_outcome_unknown() {
    assert!(matches!(
        step_of(Err(NpcTradeError::Unavailable(
            DurabilityError::Unavailable
        ))),
        NpcTradeStep::Unknown
    ));
    for refusal in [
        NpcTradeError::InvalidInput,
        NpcTradeError::AuthorityRejected,
        NpcTradeError::NoBackpack,
        NpcTradeError::InsufficientFunds,
        NpcTradeError::CapacityExceeded,
        NpcTradeError::DoesNotFit,
        NpcTradeError::ItemNotFound,
        NpcTradeError::ConflictingOccurrence,
        NpcTradeError::ConflictingCandidate,
    ] {
        let NpcTradeStep::Refused(refusal) = step_of(Err(refusal)) else {
            panic!("a refusal wrote nothing");
        };
        assert_eq!(refusal.disposition, NpcIntentDisposition::Rejected);
        assert!(refusal.update.conversation.is_none() && refusal.update.window.is_none());
    }
}
