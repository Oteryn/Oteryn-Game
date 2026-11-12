//! Real MAIL command owner and pinned Decision-25 document profiles on PostgreSQL.
use super::fixture::{Harness,debug,id,run};
use super::{Dependencies,TestResult,prepare_current_session};
use crate::gameplay_transport::{capabilities::SelectedCapabilities,connection::FreshAdmissionAuthority,item_view::ItemKey};
use oteryn_protocol_oteryn::mail::{MailIntent,MailOutcome,encode_intent};

#[test]
fn srx1_write_once_document_production_dispatch_replay_and_read_only_postgres()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        run(async |admin| {
            let started=std::time::Instant::now();
            let h=Harness::create_with_admission_seed(admin,"srx1_writing",true,false).await?;
            super::reset();prepare_current_session(&h).await?;
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps=Dependencies::new()?;
            let (controller,pin)=crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id,None);
            let active=controller.active().ok_or("native generation")?;
            let content_pin=crate::foundation::ChannelContentPin::from_activation(
                deps.admitted.world_id,pin.activation_sequence(),pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),(6,0,7));
            let deps=Dependencies::with_content(content_pin,deps.room,deps.spells)?;

            deps.runtime.lock().await.initialize_pinned_test_position(deps.actor,
                crate::foundation::MovementLocalPosition{x:6,y:0,floor:7}).map_err(debug)?;
            let mut owner=deps.owner(&h,&authority);owner.active_generation=Some(active);
            deps.initialize(&owner).await;
            let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
            let fence=crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
            super::item_look::seed_main_backpack(&h,&[("oteryn:item.tibia.i2818",1)]).await?;
            let mut admitted=deps.admitted;
            admitted.item_fence=Some(fence);
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(owner.offered_capabilities(),&[4,6,115]).ok_or("writing capability offer")?;
            let payload=encode_intent(&MailIntent::WriteText {item_handle:1,text:"One inscription: żółw 🐢".into()}).map_err(debug)?;
            for _ in 0..2 {
                assert_eq!(owner.mail_command(deps.actor,admitted,1,&payload,Some(ItemKey::Instance(id(71))),None).await,Some(MailOutcome::TextWritten));
            }
            assert_eq!(owner.mail_command(deps.actor,admitted,2,&payload,Some(ItemKey::Instance(id(71))),None).await,Some(MailOutcome::NotWritable));
            let item:(String,String,i64)=sqlx::query_as("SELECT definition_production_key,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(item,("oteryn:item.tibia.i2834".into(),"2".into(),1));
            let texts:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(texts,1);
            let audits:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(audits,1,"one write-once transform despite replay and refused rewrite");
            drop(owner);drop(authority);drop(seal);h.cleanup().await?;
            eprintln!("SRX1_PG writing producer/replay/read-only elapsed={:?}",started.elapsed());
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"writing PG worker panicked")??;
    Ok(())
}
