//! Q8: maximum donor SELL through real command7/8, native PostgreSQL owner and audit.
use super::{Harness,TestResult,debug,id};
use super::cook::{Client,drive,prepare_npc};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession,UseCommand};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::npc_service as wire;

pub(super) async fn scenario(h:&Harness,owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession,
    actor:ExactActorRef,hireling:[u8;16])->TestResult {
    use crate::domain::hireling::catalogue;
    let mut entries=vec![("oteryn:item.tibia.i43963",1)];
    entries.extend(std::iter::repeat_n(("oteryn:item.tibia.i3264",1),15));
    super::super::item_look::seed_main_backpack(h,&entries).await?;
    let mut tx=h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_hireling_entitlements(character_id,entitlement_id) VALUES(encode($1,'hex')::uuid,1004)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let npc=prepare_npc(owner,admitted,actor,hireling).await?;
    let (mut server,stream)=tokio::io::duplex(1 <<20);
    let mut client=Client::new(stream,admitted);
    let client_run=async {
    client.talk(npc,"hi").await?;
    client.talk(npc,"goods").await?;
    let position=catalogue::offers(11).position(|offer|catalogue::ROWS[offer.row].item_id==43963
        && offer.side==catalogue::Side::Sell).ok_or("highest executable sell row")?;
    let page=position/wire::MAX_OFFERS;
    let window=client.talk_window(npc,&format!("buy loot {}",page+1)).await?.window.ok_or("bounded trade window")?;
    let (_,revision)=window.open.ok_or("open window")?;
    let index=position%wire::MAX_OFFERS;
    let shown=window.offers.get(index).ok_or("highest offer visible")?;
    assert_eq!((shown.side,shown.unit_price),(wire::NpcTradeSide::Sell,5_400_000));
    let intent=wire::NpcTradeIntent {npc_actor:npc,catalogue_revision:revision,offer_index:index as u32,
        side:wire::NpcTradeSide::Sell,quantity:1,expected_unit_price:5_400_000};
    let full_command=client.next_command;
    let refused=client.trade(&intent,full_command).await?;
    assert_eq!(refused,wire::NpcIntentDisposition::Rejected);
    assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,0,"no-room creates no receipt");
    assert_eq!(count(h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i43963' AND lifecycle=1 AND quantity=1").await?,1,"no-room burns nothing");
    assert_eq!(count(h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3043'").await?,0,"no-room mints nothing");
    assert_eq!(count(h,"SELECT count(*) FROM game_item_audit_outbox a JOIN game_npc_trades t USING(transaction_id)").await?,0,"no-room writes no trade audit");

    // Fixture-only removal gives exactly six free entries after the native item BURN.
    let mut tx=h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    let character=admitted.item_fence.ok_or("current House item fence")?.character_id;
    let filler:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i \
        JOIN game_item_container_entries e USING(item_instance_id,world_id) \
        JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id \
         AND b.character_id=e.character_id AND b.world_id=e.world_id \
        WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 \
         AND i.definition_production_key='oteryn:item.tibia.i3264' \
        ORDER BY e.placement_ordinal DESC LIMIT 1")
        .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
    assert_eq!(sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(&filler).execute(&mut *tx).await?.rows_affected(),1);
    assert_eq!(sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(&filler).execute(&mut *tx).await?.rows_affected(),1);
    tx.commit().await?;
    // A previously opened expanded page grants no permanent Trader entitlement.
    // There is now enough room and the sold item exists, so this refusal isolates the gate.
    super::cook::entitlement(h,id(41),1004,false).await?;
    let revoked_command=client.next_command;
    let revoked=client.trade(&intent,revoked_command).await?;
    assert_eq!(revoked,wire::NpcIntentDisposition::Rejected);
    assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,0);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i43963' AND lifecycle=1 AND quantity=1").await?,1);
    super::cook::entitlement(h,id(41),1004,true).await?;
    let command=client.next_command;
    let accepted=client.trade(&intent,command).await?;
    assert_eq!(accepted,wire::NpcIntentDisposition::Ok);
    let amounts:Vec<i64>=sqlx::query_scalar("SELECT i.quantity FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3043' ORDER BY e.placement_ordinal")
        .fetch_all(&h.pool).await?;
    assert_eq!(amounts,vec![100,100,100,100,100,40]);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_entries").await?,20);
    assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades WHERE side=2 AND total_gold_units=5400000 AND bank_debit_gold_units=0 AND line_count=7").await?,1);
    let audit:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_npc_trades t USING(transaction_id)").fetch_one(&h.pool).await?;
    let (_,trade)=crate::durability::npc_trade_audit::decode_npc_trade_envelope(&audit).map_err(debug)?;
    assert_eq!(trade.minted_gold_units,5_400_000);assert_eq!(trade.lines.len(),7);
    assert_eq!(trade.house_key.as_deref(),Some(super::HOUSE));
    let stored_house:String=sqlx::query_scalar("SELECT house_key FROM game_npc_trades").fetch_one(&h.pool).await?;
    assert_eq!(stored_house,super::HOUSE);
    assert!(trade.bank_debit.is_none());
    assert_eq!(client.trade(&intent,command).await?,wire::NpcIntentDisposition::Ok);
    // Exercise the durable owner's replay too; transport DuplicateReplay alone would only prove
    // the connection cache. The stored occurrence is read before current conversation/state.
    assert!(matches!(crate::gameplay_transport::npc_trade_command::npc_trade(owner,actor,UseCommand {
        game_session_id:admitted.game_session_id,command_id:command,item_fence:admitted.item_fence,
    },intent).await,crate::gameplay_transport::npc_trade_command::NpcTradeStep::Traded));
    assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,1);
    assert_eq!(count(h,"SELECT count(*) FROM game_item_audit_outbox a JOIN game_npc_trades t USING(transaction_id)").await?,1);
    assert_eq!(count(h,"SELECT sum(quantity)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3043' AND lifecycle=1").await?,540);
    client.close().await?;
    Ok(())
    };
    drive(owner,admitted,&mut server,client_run).await?;
    Ok(())
}

async fn count(h:&Harness,sql:&'static str)->TestResult<i64> {Ok(sqlx::query_scalar(sql).fetch_one(&h.pool).await?)}
