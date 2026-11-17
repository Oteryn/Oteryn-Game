//! ITEM_INSCRIPTION_READ_V1 through the real composed owner on PostgreSQL: the carried item's
//! stored text under the session's item fence, refusals for anything outside that custody.
use super::fixture::{Harness,debug,id,run};
use super::{Dependencies,TestResult,prepare_current_session};
use crate::gameplay_transport::{capabilities::SelectedCapabilities,connection::FreshAdmissionAuthority,item_view::ItemKey};
use oteryn_protocol_oteryn::item_inscription::Reply;
use oteryn_protocol_oteryn::mail::{MailIntent,MailOutcome,encode_intent};

#[test]
fn item_inscription_read_returns_carried_text_and_refuses_foreign_items_postgres()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        run(async |admin| {
            let h=Harness::create_with_admission_seed(admin,"item_inscription",true,false).await?;
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
            // Item 71: a write-once document (2818 -> read-only 2834); item 72: a letter.
            super::item_look::seed_main_backpack(&h,&[("oteryn:item.tibia.i2818",1),("oteryn:item.tibia.i3505",1)]).await?;
            let mut admitted=deps.admitted;
            admitted.item_fence=Some(fence);
            let read=async |admitted:crate::gameplay_transport::connection::AdmittedSession,item:[u8;16]| owner.item_inscription_read(admitted,Some(ItemKey::Instance(item))).await.map(|bytes| Reply::decode(&bytes));
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(owner.offered_capabilities(),&[4,6,115]).ok_or("mail offer")?;
            assert_eq!(read(admitted,id(72)).await,None,"unselected capability is refused");
            // This direct owner has entry-room movement; 511 is offered only on bundle Worlds.
            // Model that negotiated offer explicitly instead of silently dropping 511.
            admitted.continuity.selected_capabilities=SelectedCapabilities::select(crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,&[4,6,115,511]).ok_or("inscription offer")?;
            assert!(admitted.continuity.selected_capabilities.contains(511));
            assert_eq!(read(admitted,id(72)).await,Some(Some(Reply{text:String::new(),max_length:1_999,writable:true})));
            let payload=encode_intent(&MailIntent::WriteText{item_handle:1,text:"One inscription: żółw 🐢".into()}).map_err(debug)?;
            assert_eq!(owner.mail_command(deps.actor,admitted,1,&payload,Some(ItemKey::Instance(id(72))),None).await,Some(MailOutcome::TextWritten));
            assert_eq!(read(admitted,id(72)).await,Some(Some(Reply{text:"One inscription: żółw 🐢".into(),max_length:1_999,writable:true})),"the owned item's stored text");
            assert_eq!(owner.mail_command(deps.actor,admitted,2,&payload,Some(ItemKey::Instance(id(71))),None).await,Some(MailOutcome::TextWritten));
            let sealed=read(admitted,id(71)).await.flatten().ok_or("sealed document")?;
            assert_eq!(sealed.text,"One inscription: żółw 🐢");
            assert!(!sealed.writable,"the write-once target only shows its text");
            sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($1,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Foreign Reader' FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid").bind(id(90).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
            super::item_look::seed_character_backpack(&h,90,98,&[("oteryn:item.tibia.i3505",1)]).await?;
            let foreign_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id<>encode($2,'hex')::uuid").bind(id(99).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(foreign_count,1,"foreign letter exists in another character's bag");
            // Not carried (unknown), the main backpack itself and a map entry are refused.
            assert_eq!(read(admitted,id(99)).await,None,"another character owns the letter");
            assert_eq!(read(admitted,id(100)).await,None);
            assert_eq!(read(admitted,id(70)).await,None);
            assert_eq!(owner.item_inscription_read(admitted,Some(ItemKey::MapBase{bundle_digest:[0;32],placement_key:1,reset_epoch:0})).await,None);
            admitted.item_fence=None;
            assert_eq!(read(admitted,id(72)).await,None,"an unfenced session is refused");
            drop(owner);drop(authority);drop(seal);h.cleanup().await?;
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"item inscription PG worker panicked")??;
    Ok(())
}
