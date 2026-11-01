#![allow(clippy::expect_used)]
use super::*;
use crate::durability::item_transfer::ItemStackClass;
use crate::foundation::{ChannelId,CommandId,ConnectionGeneration,GameSessionId,RuntimeScopeRefV1,ScopeOwnershipGeneration};

fn id(tag:u8)->[u8;16] {let mut id=[0;16];id[5]=1;id[6]=0x70;id[8]=0x80;id[15]=tag;id}
fn definition(key:&str)->TypedDefinitionRef {TypedDefinitionRef{family:"Item".into(),production_key:key.into(),revision_ref:"oteryn:rev.1".into()}}
fn facts(key:&str)->ItemDefinitionFacts {ItemDefinitionFacts {definition:definition(key),weight:None,stack:ItemStackClass::NonStackable,container_capacity:None,container_slot_equip_pattern:false}}
fn request()->NpcTradeRequest {
    NpcTradeRequest {
        cause:NpcTradeCause {npc_key:crate::durability::house_hireling::npc_key(id(60)),catalogue_revision:1,offer_index:0,side:NpcTradeSide::Sell,occurrence:id(70)},
        command:CommandRef::new(GameSessionId::decode(&id(50)).expect("session"),CommandId::new(9).expect("command")),
        quantity:1,unit_price:MAX_GOLD,offer_count:1,item:facts("oteryn:item.tibia.i43963"),transaction_id:id(80),event_id:id(81),
        occurred_at_unix_ms:1_790_000_000_000,output_item_instance_ids:[id(90),id(91),id(92)],
        coins:NpcTradeCoins {gold:definition(Coin::Gold.production_key()),platinum:definition(Coin::Platinum.production_key()),crystal:definition(Coin::Crystal.production_key())},
        backpack:ItemDefinitionFacts {container_capacity:Some(20),..facts("oteryn:item.tibia.i2854")},
    }
}
fn entries(request:&NpcTradeRequest,count:u8)->Vec<Entry> {
    (0..count).rev().map(|index|Entry {item_instance_id:id(index+1),placement_ordinal:u64::from(index+1),
        definition:if index==0 {request.item.definition.clone()} else {definition("oteryn:item.tibia.i3264")},
        quantity:1,live:true,not_default:false}).collect()
}

#[test]
fn source_maximum_and_residual_payout_are_bounded_and_conserved() {
    assert_eq!(crate::domain::hireling::catalogue::ROWS.iter().map(|row|row.sell).max(),Some(MAX_GOLD));
    assert_eq!(coins(MAX_GOLD),Some(vec![(Coin::Crystal,100),(Coin::Crystal,100),(Coin::Crystal,100),(Coin::Crystal,100),(Coin::Crystal,100),(Coin::Crystal,40)]));
    assert_eq!(coins(MAX_GOLD+1),None);
    for value in [0,1,99,100,9999,10000,1_009_999,1_010_000,MAX_GOLD-1,MAX_GOLD] {
        let result=coins(value).expect("within donor bound");
        assert!(result.len()<=MAX_COIN_STACKS);
        assert!(result.iter().all(|(_,count)|(1..=100).contains(count)));
        assert_eq!(result.iter().map(|(coin,count)|coin.worth()*u64::from(*count)).sum::<u64>(),value);
    }
    assert_eq!(coins(MAX_GOLD-1).expect("eight stacks").len(),MAX_COIN_STACKS);
}

#[test]
fn highest_offer_uses_native_burn_and_refuses_insufficient_room_before_materialization() {
    let request=request();let priced=validate(&request,false).expect("bounded input");
    assert!(super::super::validate(&request,false).is_err(),"ordinary NPC ceiling remains");
    let (plan,total,units)=plan(&request,priced,20,&entries(&request,15)).expect("one burn releases six slots");
    assert_eq!((total,units),(MAX_GOLD,1));assert_eq!(plan.lines.len(),7);
    assert_eq!(plan.lines[0].kind,NpcTradeLineKind::ItemBurn);assert_eq!(plan.lines[0].quantity_after,0);
    assert_eq!(plan.minted_gold_units,MAX_GOLD);assert_eq!(plan.bank_debit_gold_units,0);
    assert!(matches!(super::plan(&request,priced,20,&entries(&request,16)),Err(NpcTradeError::DoesNotFit)));
    assert_eq!(output_ids(&request),output_ids(&request.clone()),"retry identities fixed by command");
    let mut changed=request.clone();changed.command=CommandRef::new(request.command.game_session_id(),CommandId::new(10).expect("next command"));
    assert_ne!(output_ids(&request)[1],output_ids(&changed)[1]);
}

#[test]
fn high_value_trade_uses_existing_audit_owner_and_refuses_foreign_or_corrupt_coin_lines() {
    use crate::durability::npc_trade_audit::{check_npc_trade,decode_npc_trade_envelope,npc_trade_usage,SELL_ALL_RL06_PARTICIPANTS_MAX,SELL_ALL_RL06_EFFECT_WORK_UNITS_MAX};
    let request=request();let (plan,total,units)=plan(&request,validate(&request,false).expect("request"),20,&entries(&request,1)).expect("plan");
    let world=WorldId::decode(&id(40)).expect("world");let channel=ChannelId::decode(&id(41)).expect("channel");
    let fence=CurrentCharacterItemFence {character_id:CharacterId::from_bytes(id(42)).expect("character"),game_session_id:request.command.game_session_id(),
        connection_generation:ConnectionGeneration::new(1).expect("connection"),character_lease_generation:1,
        runtime_scope:RuntimeScopeRefV1::channel(world,channel),scope_ownership_generation:ScopeOwnershipGeneration::new(1).expect("generation")};
    let mut message=super::super::trade_message(&fence,world,channel,&id(43),&request,(units,total,false),&plan);
    message.house_key=Some("oteryn:content.house.fixture".into());
    check_npc_trade(&message).expect("source-derived payout");
    let usage=npc_trade_usage(&message);assert_eq!((usage.participants,usage.effect_work_units),(7,15));
    assert!(usage.participants<=SELL_ALL_RL06_PARTICIPANTS_MAX && usage.effect_work_units<=SELL_ALL_RL06_EFFECT_WORK_UNITS_MAX);
    let wire=encode_npc_trade_event(NpcTradeEventIdentity {event_id:request.event_id,transaction_id:request.transaction_id,
        occurred_at_unix_ms:request.occurred_at_unix_ms,server_build_id:SERVER_BUILD_ID},message.clone(),Type2EventTuple::V2).expect("encode");
    assert_eq!(decode_npc_trade_envelope(&wire).expect("decode").1,message);
    let mut foreign=message.clone();foreign.cause.as_mut().expect("cause").npc_key="oteryn:npc.tibia.sam".into();
    assert!(check_npc_trade(&foreign).is_err());
    let mut wrong_price=message.clone();wrong_price.unit_price-=1;wrong_price.total_gold_units-=1;
    assert!(check_npc_trade(&wrong_price).is_err());
    let mut missing_scope=message.clone();missing_scope.house_key=None;assert!(check_npc_trade(&missing_scope).is_err());
    let mut wrong_scope=message.clone();wrong_scope.house_key=Some("oteryn:activity.instance.fixture".into());assert!(check_npc_trade(&wrong_scope).is_err());
    let mut corrupt=message;corrupt.lines.swap(1,6);
    assert!(check_npc_trade(&corrupt).is_err());
}
