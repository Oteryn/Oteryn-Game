#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::domain::CharacterId;
use crate::foundation::{
    ChannelId, CommandId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, WorldId,
};

fn id(n: u8) -> [u8; 16] {
    let mut id = [n; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn uuid(n: u8) -> String {
    let s = id(n).iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    )
}
fn request() -> PlayerTradeRequest {
    let parties=[0,1].map(|n| {
        let accepted=CurrentCharacterItemFence {
            character_id:CharacterId::from_bytes(id(10+n)).unwrap(),game_session_id:GameSessionId::decode(&id(20+n)).unwrap(),
            connection_generation:ConnectionGeneration::new(u64::MAX).unwrap(),character_lease_generation:u64::MAX,
            runtime_scope:RuntimeScopeRefV1::channel(WorldId::decode(&id(1)).unwrap(),ChannelId::decode(&id(2)).unwrap()),
            scope_ownership_generation:ScopeOwnershipGeneration::new(u64::MAX).unwrap(),
        };
        TradeParticipant {accepted,command:CommandRef::new(accepted.game_session_id,CommandId::new(u64::MAX).unwrap()),leaf:TradeLeaf {
            item:json!({"item_instance_id":uuid(30+n),"world_id":uuid(1),"definition_family":"Item",
                "definition_production_key":"oteryn:item.coin","definition_revision_ref":"r1","quantity":100,
                "lifecycle":1,"state_revision":"18446744073709551615",
                "minted_transaction_id":uuid(60+n),"last_transaction_id":uuid(70+n),"created_xact_id":"18446744073709551615"}),
            entry:json!({"item_instance_id":uuid(30+n),"world_id":uuid(1),"character_id":uuid(10+n),
                "parent_item_instance_id":uuid(40+n),"placement_ordinal":u64::MAX-1,"placed_transaction_id":uuid(70+n)}),
            key:Some(json!({"item_instance_id":uuid(30+n),"key_number":65535,"minted_transaction_id":uuid(60+n),"created_xact_id":"18446744073709551615"})),
            timed:Some(json!({"item_instance_id":uuid(30+n),"charges":65535,"remaining_ms":604800000,"deadline_at":null,
                "state_revision":"18446744073709551615"})),text:Some(json!({"sha256":"f".repeat(64)})),
            text_revision:Some(json!({"item_instance_id":uuid(30+n),"last_revision":"18446744073709551615"})),
        }}
    });
    PlayerTradeRequest {
        pair: CommandRef::new(
            parties[0].accepted.game_session_id,
            CommandId::new(1).unwrap(),
        ),
        occurrence_id: id(3),
        transaction_id: id(4),
        event_id: id(5),
        occurred_at_unix_ms: i64::MAX - 8_000_000_000,
        participants: parties,
        content_digest: [0xff; 32],
        weight_digest: [0xff; 32],
        node_id: id(6),
        node_registration_revision: u64::MAX,
        content_revision: "content1".into(),
        ruleset_revision: "ruleset1".into(),
        sim_revision: "sim1".into(),
    }
}
#[test]
fn player_trade_audit_roundtrip_bounds_and_mutations() {
    let mut request = request();
    for p in &mut request.participants {
        p.leaf.item["definition_family"] = json!("F".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX));
        p.leaf.item["definition_production_key"] =
            json!("k".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX));
        p.leaf.item["definition_revision_ref"] =
            json!("r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX));
    }
    request.content_revision = "c".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    request.ruleset_revision = "r".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    request.sim_revision = "s".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    let binding = request.binding().unwrap();
    for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
        let wire = encode(&request, &binding, [(u64::MAX, 20); 2], tuple)
            .expect("maximal leaf event fits registered limits");
        let (envelope, value) = decode(&wire).unwrap();
        eprintln!(
            "TRADE-PLAYER-1 maximal payload={} envelope={}",
            envelope.payload.len(),
            wire.len()
        );
        assert!(envelope.payload.len() <= mint::RL07_PAYLOAD_BYTES_MAX);
        assert!(wire.len() <= mint::RL07_ENVELOPE_BYTES_MAX);
        assert_eq!(value.request_binding, binding);
        assert_eq!(envelope.event_schema_revision, tuple.schema_revision());
        let mut invalid = envelope.clone();
        invalid.command_id = Some(1);
        assert!(decode(&invalid.encode_to_vec()).is_err());
    }
    for edit in [0, 1, 2] {
        let mut bad = request.clone();
        match edit {
            0 => bad.participants[1].leaf = bad.participants[0].leaf.clone(),
            1 => {
                bad.participants[1].accepted.game_session_id =
                    bad.participants[0].accepted.game_session_id
            }
            _ => bad.participants[1].leaf.item["quantity"] = json!(0),
        }
        assert!(
            encode(
                &bad,
                &bad.binding().unwrap(),
                [(u64::MAX, 20); 2],
                Type2EventTuple::V2
            )
            .is_err()
        );
    }
    assert!(decode(&[]).is_err());
    assert!(decode(&[255; 16]).is_err());
}
