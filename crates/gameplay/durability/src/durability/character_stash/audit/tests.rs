#![allow(clippy::expect_used)]
use super::*;
use crate::foundation::{ChannelId,CommandId,ConnectionGeneration,GameSessionId,RuntimeScopeRefV1,ScopeOwnershipGeneration,WorldId};

fn text(seed:u8)->String {format!("00000000-0001-7000-8000-0000000000{seed:02x}")}
fn id(seed:u8)->[u8;16] {uuid_text(&text(seed)).expect("fixture UUID")}

#[test]
fn maximum_stash_outputs_and_named_house_fit_existing_audit_rows() {
    let world=WorldId::decode(&id(1)).expect("world");let channel=ChannelId::decode(&id(2)).expect("channel");
    let prefix="oteryn:content.house.";
    let house=format!("{prefix}{}","h".repeat(super::super::super::house_scope_handoff::HOUSE_KEY_MAX_BYTES-prefix.len()));
    let house_id=super::super::super::house_scope_handoff::HouseId::new(world,&house).expect("max house");
    let mut fence=CurrentCharacterItemFence {character_id:crate::domain::CharacterId::from_bytes(id(3)).expect("character"),
        game_session_id:GameSessionId::decode(&id(4)).expect("session"),connection_generation:ConnectionGeneration::new(u64::MAX).expect("connection"),
        character_lease_generation:1,runtime_scope:house_id.runtime_scope().expect("house scope"),
        scope_ownership_generation:ScopeOwnershipGeneration::new(u64::MAX).expect("generation")};
    let request=StashRequest {command:CommandRef::new(fence.game_session_id,CommandId::new(u64::MAX).expect("command")),
        intent:StashIntent {page:MAX_STASH_PAGE,action:StashAction::Withdraw {row:id(5),amount:MAX_STASH_OUTPUTS*100}},
        source:None,row_open:true,access_refusal:None};
    let plan=Plan {definition:TypedDefinitionRef {family:"Item".into(),production_key:"k".repeat(512),revision_ref:"r".repeat(512)},
        entry:text(6),amount:MAX_STASH_OUTPUTS*100,before:i64::from(MAX_STASH_UNITS),previous:Some(text(7)),parent:id(8).to_vec(),
        lines:(0..MAX_STASH_OUTPUTS).map(|index|json!({"item":text(index as u8+20),"count":100,"ordinal":u64::MAX-u64::from(index)})).collect()};
    let bytes=encode(fence,channel,&request,&plan,&text(10),&text(11),i64::MAX,Some(&house)).expect("max scoped stash audit");
    let decoded=EventEnvelopeV1::decode(bytes.as_slice()).expect("envelope");
    let operation=decode_operation(&decoded.payload).expect("closed payload");
    assert_eq!(operation.items.len(),MAX_STASH_OUTPUTS as usize);
    assert_eq!(operation.house_key,Some(house));assert_eq!(operation.house_scope_ownership_generation,Some(u64::MAX));
    assert_eq!(decoded.payload.len(),22_808,"twenty maximum definition/revision lines and full House metadata");
    assert!(decoded.payload.len()<=PAYLOAD_BYTES_MAX);assert!(bytes.len()<=ENVELOPE_BYTES_MAX);
    fence.runtime_scope=RuntimeScopeRefV1::channel(world,channel);
    let channel_bytes=encode(fence,channel,&request,&plan,&text(10),&text(11),i64::MAX,None).expect("original channel");
    let channel_decoded=EventEnvelopeV1::decode(channel_bytes.as_slice()).expect("channel envelope");
    assert_eq!(decoded.payload.len()-channel_decoded.payload.len(),526,"House512 field + maximum generation");
    assert_eq!(bytes.len()-channel_bytes.len(),526);
    let channel_operation=decode_operation(&channel_decoded.payload).expect("channel payload");
    assert!(channel_operation.house_key.is_none() && channel_operation.house_scope_ownership_generation.is_none());
    let mut partial=operation.clone();partial.house_scope_ownership_generation=None;
    assert!(decode_operation(&partial.encode_to_vec()).is_err());
    let mut zero=operation.clone();zero.house_scope_ownership_generation=Some(0);
    assert!(decode_operation(&zero.encode_to_vec()).is_err());
    let mut foreign=operation;foreign.house_key=Some("oteryn:activity.instance.fake".into());
    assert!(decode_operation(&foreign.encode_to_vec()).is_err());
    let mut unknown=channel_decoded.payload;unknown.extend_from_slice(&[0x40,1]);
    assert!(decode_operation(&unknown).is_err(),"unknown audit field");
}
