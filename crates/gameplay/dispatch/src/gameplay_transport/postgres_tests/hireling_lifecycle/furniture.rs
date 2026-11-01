//! Crystal createItemBatch: a Ground-only catalogue target is bought as a proven kit.
use super::{Harness, TestResult, debug};
use super::cook::{Client, count, drive, prepare_npc};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::{CommandStatus, house_wrap as wrap, npc_service as npc};

pub(super) async fn scenario(h:&Harness,owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession,
    actor:ExactActorRef,hireling:[u8;16])->TestResult {
    use crate::domain::hireling::catalogue;
    let mut entries=vec![("oteryn:item.tibia.i3031",100)];
    entries.extend(std::iter::repeat_n(("oteryn:item.tibia.i3028",1),19));
    super::super::item_look::seed_main_backpack(h,&entries).await?;
    let native=owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native catalogue")?;
    let target=crate::durability::item_mint::TypedDefinitionRef {family:"Item".into(),
        production_key:"oteryn:item.tibia.i2352".into(),revision_ref:"definition-r1".into()};
    let ground=crate::gameplay_transport::item_ref_admission::generation_item_facts(native,&target).map_err(debug)?;
    assert_eq!(ground.stack,crate::durability::item_transfer::ItemStackClass::Unknown,
        "a source offer never grants portable facts to its furniture target");
    let npc=prepare_npc(owner,admitted,actor,hireling).await?;
    let (mut server,stream)=tokio::io::duplex(1 <<20);
    let mut client=Client::new(stream,admitted);
    let client_run=async {
        client.talk(npc,"hi").await?;
        client.talk(npc,"goods").await?;
        let window=client.talk_window(npc,"various").await?.window.ok_or("free various window")?;
        let (_,revision)=window.open.ok_or("open goods")?;
        let offers=catalogue::offers(0).collect::<Vec<_>>();
        let position=offers.iter().position(|offer|catalogue::ROWS[offer.row].item_id==2352
            && offer.side==catalogue::Side::Buy).ok_or("source furniture offer")?;
        let shown=&window.offers[position];
        assert_eq!((shown.side,shown.unit_price),(npc::NpcTradeSide::Buy,20));
        let intent=npc::NpcTradeIntent {npc_actor:npc,catalogue_revision:revision,offer_index:position as u32,
            side:npc::NpcTradeSide::Buy,quantity:1,expected_unit_price:20};
        let refused=client.next_command;
        assert_eq!(client.trade(&intent,refused).await?,npc::NpcIntentDisposition::Rejected);
        assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,0);
        assert_eq!(count(h,"SELECT count(*) FROM game_house_furniture_kit_origins").await?,0);
        assert_eq!(count(h,"SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").await?,100);

        // Release one fixture filler; the real BUY must account for its retained coin stack.
        let mut tx=h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        let character=admitted.item_fence.ok_or("current House item fence")?.character_id;
        let filler:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i \
            JOIN game_item_container_entries e USING(item_instance_id,world_id) \
            JOIN game_item_container_slots b ON b.item_instance_id=e.parent_item_instance_id \
             AND b.character_id=e.character_id AND b.world_id=e.world_id \
            WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 \
             AND i.definition_production_key='oteryn:item.tibia.i3028' \
            ORDER BY e.placement_ordinal DESC LIMIT 1")
            .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        assert_eq!(sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(&filler).execute(&mut *tx).await?.rows_affected(),1);
        assert_eq!(sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(&filler).execute(&mut *tx).await?.rows_affected(),1);
        tx.commit().await?;
        let purchase=client.next_command;
        assert_eq!(client.trade(&intent,purchase).await?,npc::NpcIntentDisposition::Ok);
        assert_eq!(client.trade(&intent,purchase).await?,npc::NpcIntentDisposition::Ok);
        assert_eq!(count(h,"SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").await?,80);
        assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,1);
        assert_eq!(count(h,"SELECT count(*) FROM game_house_furniture_kit_origins").await?,1);
        assert_eq!(count(h,"SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2352'").await?,0);
        let bytes:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_house_furniture_kit_origins")
            .fetch_one(&h.pool).await?;
        let kit:[u8;16]=bytes.try_into().map_err(|_|"kit UUID")?;
        let envelope:Vec<u8>=sqlx::query_scalar("SELECT a.envelope FROM game_item_audit_outbox a JOIN game_npc_trades t USING(transaction_id)")
            .fetch_one(&h.pool).await?;
        let (_,audit)=crate::durability::npc_trade_audit::decode_npc_trade_envelope(&envelope).map_err(debug)?;
        let origin=audit.house_furniture_kit.ok_or("native kit audit")?;
        assert_eq!(origin.item_instance_id,kit);
        assert_eq!(origin.source_row,offers[position].row as u32);
        assert_eq!(origin.target.ok_or("source target")?.production_key,target.production_key);
        assert_eq!(audit.house_key.as_deref(),Some(super::HOUSE));
        assert!(sqlx::query("UPDATE game_house_furniture_kit_origins SET target_key='oteryn:item.tibia.i2426'")
            .execute(&h.pool).await.is_err(),"immutable unWrapId cannot be replaced");

        let request=wrap::Request {action:wrap::Action::Unwrap,item:kit,expected_revision:1,
            acl_revision:1,position:[32311,32804,7]};
        let stale=wrap::Request {acl_revision:2,..request};
        let command=client.next_command;
        let (status,body)=client.opaque(wrap::COMMAND_WRAP,&stale.encode(),command).await?;
        assert_eq!((status,wrap::Reply::decode(&body)),(CommandStatus::Rejected,Some(wrap::Reply::Refused(wrap::Refusal::Stale))));
        assert_eq!(count(h,"SELECT count(*) FROM game_house_furniture_wrap_receipts").await?,0);
        let unwrap_command=client.next_command;
        let (status,body)=client.opaque(wrap::COMMAND_WRAP,&request.encode(),unwrap_command).await?;
        assert_eq!((status,wrap::Reply::decode(&body)),(CommandStatus::Accepted,Some(wrap::Reply::Committed(2))));
        custody(h,kit,true,"oteryn:item.tibia.i2352",2).await?;
        assert_native_replay(owner,admitted,unwrap_command,request,"currently placed").await?;
        custody(h,kit,true,"oteryn:item.tibia.i2352",2).await?;
        let (status,body)=client.opaque(wrap::COMMAND_WRAP,&request.encode(),unwrap_command).await?;
        assert_eq!((status,wrap::Reply::decode(&body)),(CommandStatus::DuplicateReplay,Some(wrap::Reply::Committed(2))));
        let rewrap=wrap::Request {action:wrap::Action::Wrap,expected_revision:2,..request};
        let command=client.next_command;
        let (status,body)=client.opaque(wrap::COMMAND_WRAP,&rewrap.encode(),command).await?;
        assert_eq!((status,wrap::Reply::decode(&body)),(CommandStatus::Accepted,Some(wrap::Reply::Committed(3))));
        custody(h,kit,false,"oteryn:item.tibia.i23398",3).await?;
        // Exercise the wrap owner's durable replay after the item has since changed again.
        assert_native_replay(owner,admitted,unwrap_command,request,"rewrapped").await?;
        custody(h,kit,false,"oteryn:item.tibia.i23398",3).await?;
        assert_eq!(count(h,"SELECT count(*) FROM game_house_furniture_wrap_receipts").await?,2);
        assert_eq!(count(h,"SELECT count(*) FROM game_house_furniture_kit_origins").await?,1);
        assert_eq!(count(h,"SELECT count(*) FROM game_npc_trades").await?,1);
        client.close().await?;
        Ok(())
    };
    drive(owner,admitted,&mut server,client_run).await?;
    Ok(())
}

async fn assert_native_replay(owner:&ComposedFreshAdmission<'_, '_, '_>,admitted:AdmittedSession,
    command:u64,request:wrap::Request,stage:&str)->TestResult {
    // The live server may still own the shared durability holder while sending
    // mandatory inventory effects after its ACK. Wait through the existing
    // bounded command budget, keeping the same historical native CommandRef.
    // A known refusal is final and fails this assertion; only None is unknown.
    let retry=crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root);
    let replay=retry.retry_unavailable(async || owner.house_wrap(admitted,command,request).await
        .ok_or(crate::gameplay_transport::connection::AdmissionRefusal::Unavailable))
        .await.map_err(|error|format!("native furniture replay while {stage}: {error:?}"))?;
    assert_eq!(replay,wrap::Reply::Committed(2),"native replay while {stage}");
    Ok(())
}

async fn custody(h:&Harness,item:[u8;16],house:bool,key:&str,revision:i64)->TestResult {
    let state:(String,i64,bool,bool,bool)=sqlx::query_as("SELECT i.definition_production_key,i.state_revision::bigint,\
        EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id),\
        EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id),\
        i.minted_transaction_id=k.mint_transaction_id \
        FROM game_item_instances i JOIN game_house_furniture_kit_origins k USING(item_instance_id) \
        WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state,(key.into(),revision,house,!house,true));
    Ok(())
}
