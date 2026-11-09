//! T125: real command envelope, PostgreSQL gold, contract-exact Platform test producer.
use super::fixture::{Harness,debug,id,run};
use super::{Dependencies,TestResult,prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1,MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome,FreshAdmissionAuthority};
use crate::map::boot::{BootPins,boot,tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::coin_market::{self as wire,Request};
use serde_json::Value;
#[path="coin_market/peer.rs"]mod peer;
fn uuid(v:&[u8;16])->String{let h=v.iter().map(|b|format!("{b:02x}")).collect::<String>();format!("{}-{}-{}-{}-{}",&h[..8],&h[8..12],&h[12..16],&h[16..20],&h[20..])}
async fn call(owner:&crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,session:&mut crate::gameplay_transport::connection::AdmittedSession,id:u64,r:Request)->TestResult<Value>{
 let bytes=wire::encode(&r).map_err(debug)?;let (status,result)=super::cyclopedia::support::exchange(owner,session,id,wire::COMMAND,&bytes).await?;
 assert_eq!(status,oteryn_protocol_oteryn::CommandStatus::Accepted);Ok(serde_json::from_str(&wire::decode_reply(&result).map_err(debug)?)?)
}
#[test]fn coin_market_postgres_real_wire_reserve_partial_fill_replay_restart_and_gold_conservation()->TestResult{
 if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none(){return Ok(());}
 std::thread::Builder::new().stack_size(256<<20).spawn(||scenario().map_err(|e|e.to_string()))?.join().map_err(|_|"coin scenario panicked")??;Ok(())
}
fn scenario()->TestResult{run(async |admin|{
 let harness=Harness::create_with_admission_seed(admin,"coin_market",true,false).await?;super::reset();prepare_current_session(&harness).await?;
 let seal=harness.recovery.seal_current().map_err(debug)?;let authority=harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let backpack_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
                .ok_or("backpack ref")?,
        )
        .ok_or("backpack definition")?;
        let coin_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
                .ok_or("coin ref")?,
        )
        .ok_or("coin definition")?;

        let box_id = index
            .definition_ref(
                "Item",
                &backpack_definition.production_key,
                &backpack_definition.revision_ref,
            )
            .ok_or("box id")?
            .get()
            - 1;
        let coin_id = index
            .definition_ref(
                "Item",
                &coin_definition.production_key,
                &coin_definition.revision_ref,
            )
            .ok_or("coin id")?
            .get()
            - 1;
        let (bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (&backpack_definition.production_key, box_id),
            (&coin_definition.production_key, coin_id),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let locker = bundle
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.tiles)
            .find(|t| t.x == 6)
            .ok_or("locker tile")?;
        locker.items.get_mut(1).ok_or("locker item")?.attrs.depot = Some(0);
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == backpack_definition.production_key),
                    blocks_projectile: false,
                    pickupable: key == coin_definition.production_key,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (7, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        scenario_body(&harness,&authority,&mut owner,&deps).await?;
        drop(owner);drop(authority);drop(seal);harness.cleanup().await
    })}
async fn scenario_body(h:&Harness,authority:&crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,owner:&mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,deps:&Dependencies)->TestResult{
 use crate::durability::guild_chat_fixture as chars;
 use crate::native_admission_source::store::StoreClient;
 use crate::gameplay_transport::coin_market::reconcile_coin_market;
 let buyer_account=chars::sid(90,2);let buyer_character=chars::sid(90,1);
 chars::historical_character(&h.pool,90,crate::domain::CharacterId::from_bytes(buyer_character).map_err(debug)?,buyer_account,id(42),"Coin Buyer").await?;
 let peer_fence=chars::historical_session(&h.pool,90,super::fixture::fence(1)?.item_fence()).await?;
 seed_gold(h,[(id(40),id(41),200),(buyer_account,buyer_character,220)]).await?;
 let shared=std::sync::Arc::new(std::sync::Mutex::new(peer::State::default()));
 for a in [uuid(&id(40)),uuid(&buyer_account)]{shared.lock().map_err(|_|"coin peer")?.wallets.insert(a,peer::Wallet{transferable:100,non_transferable:5,reserved:0});}
 owner.store=Some(std::sync::Arc::new(tokio::sync::Mutex::new(StoreClient::new(Box::new(peer::NoStore),"platform-test".into()).with_coins(Some(peer::client(&shared))))));
 deps.initialize(owner).await;
 let mut admitted=deps.admitted;
 let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(admitted.game_session_id).await?.0;
 admitted.item_fence=crate::gameplay_transport::item_fence_of(current);admitted.first_entry=FirstEntryOutcome::Positioned;
 admitted.continuity.selected_capabilities=crate::gameplay_transport::capabilities::SelectedCapabilities::select(owner.offered_capabilities(),&[4,6,18,37,63]).ok_or("coin capabilities")?;
 let fence=admitted.item_fence.ok_or("coin fence")?;let world=admitted.world_id;
 let buyer=h.root.coin_owner(authority,&h.node,world,Some(peer_fence))?;
 let worker=h.root.coin_owner(authority,&h.node,world,None)?;
 let mut worker_client=peer::client(&shared);
 let b=call(owner,&mut admitted,1,Request{action:9,..Default::default()}).await?;
 assert_eq!((b["transferable"].as_str(),b["non_transferable"].as_str()),(Some("100"),Some("5")));
 let sell=Request{action:3,amount:30,price:100,operation:id(101),..Default::default()};
 let placed=call(owner,&mut admitted,2,sell.clone()).await?;assert_eq!(placed["pending"],false);assert_eq!(placed["receipt"]["receipt"]["kind"],"reserve");
 assert_eq!(bank(h,id(40)).await?,9940);
 let replay=call(owner,&mut admitted,3,sell).await?;assert_eq!(replay["operation"],uuid(&id(101)));assert_eq!(bank(h,id(40)).await?,9940);
 assert_eq!(shared.lock().map_err(|_|"coin peer")?.mutations,1);
 let book=call(owner,&mut admitted,4,Request::default()).await?;assert_eq!(book["offers"][0]["remaining"],30);
 let history=call(owner,&mut admitted,5,Request{action:7,..Default::default()}).await?;assert_eq!(history["history"][0]["kind"],"reserve");assert!(history.to_string().find("account_id").is_none());
 let detail=call(owner,&mut admitted,6,Request{action:8,offer:id(101),..Default::default()}).await?;assert_eq!(detail["pending"],false);assert!(detail.to_string().find("account_id").is_none());
 let take=Request{action:5,amount:10,offer:id(101),operation:id(102),..Default::default()};
 assert_eq!(buyer.prepare(take.clone()).await?["pending"],true);assert_eq!(bank(h,buyer_account).await?,9000);
 shared.lock().map_err(|_|"coin peer")?.dropped_ack=true;
 assert!(reconcile_coin_market(&worker,&mut worker_client).await);
 assert_eq!(bank(h,id(40)).await?,9940,"lost Platform acknowledgement cannot pay seller yet");
 assert_eq!(shared.lock().map_err(|_|"coin peer")?.wallets.get(&uuid(&buyer_account)).ok_or("buyer")?.transferable,110);
 // Restart the client/worker and retry the same durable intent after its retry deadline.
 sqlx::query("UPDATE game_coin_operations SET next_attempt=clock_timestamp()").execute(&h.pool).await?;
 let mut restarted=peer::client(&shared);let restarted_owner=h.root.coin_owner(authority,&h.node,world,None)?;
 assert!(reconcile_coin_market(&restarted_owner,&mut restarted).await);
 assert_eq!((bank(h,id(40)).await?,bank(h,buyer_account).await?),(10940,9000));
 assert_eq!(shared.lock().map_err(|_|"coin peer")?.mutations,2);
 let buyer_detail=buyer.detail(uuid(&id(102))).await?;assert!(buyer_detail["receipt"]["receipt"].get("transferable_after").is_none(),"counterparties cannot inspect the seller wallet");
 assert_eq!(buyer.prepare(take).await?["pending"],false);assert_eq!(bank(h,buyer_account).await?,9000);
 let own_take=call(owner,&mut admitted,7,Request{action:5,amount:1,offer:id(101),operation:id(103),..Default::default()}).await?;assert_eq!(own_take["result"],"rejected");
 let stale=h.root.coin_owner(authority,&h.node,world,Some(crate::durability::item_transfer::CurrentCharacterItemFence{character_lease_generation:fence.character_lease_generation+1,..fence}))?;
 assert!(stale.prepare(Request{action:4,amount:1,price:1,operation:id(104),..Default::default()}).await.is_err());
 let cancel=call(owner,&mut admitted,8,Request{action:6,offer:id(101),operation:id(105),..Default::default()}).await?;assert_eq!(cancel["pending"],false);assert_eq!(cancel["receipt"]["receipt"]["amount"],"20");
 assert_eq!(bank(h,id(40)).await?,10940,"opening fee is not refunded");
 let buy=call(owner,&mut admitted,9,Request{action:4,amount:25,price:90,operation:id(106),..Default::default()}).await?;assert_eq!(buy["pending"],false);assert_eq!(bank(h,id(40)).await?,8645);
 let accepted=buyer.prepare(Request{action:5,amount:5,offer:id(106),operation:id(107),..Default::default()}).await?;assert_eq!(accepted["pending"],true);
 assert!(reconcile_coin_market(&worker,&mut restarted).await);assert!(reconcile_coin_market(&worker,&mut restarted).await);
 assert_eq!(bank(h,buyer_account).await?,9450);
 let cancel_buy=call(owner,&mut admitted,10,Request{action:6,offer:id(106),operation:id(108),..Default::default()}).await?;assert_eq!(cancel_buy["pending"],false);assert_eq!(bank(h,id(40)).await?,10445);
 let sell=call(owner,&mut admitted,11,Request{action:3,amount:10,price:100,operation:id(109),..Default::default()}).await?;assert_eq!(sell["pending"],false);
 buyer.prepare(Request{action:4,amount:10,price:120,operation:id(110),..Default::default()}).await?;
 assert!(reconcile_coin_market(&worker,&mut restarted).await);
 assert_eq!((bank(h,id(40)).await?,bank(h,buyer_account).await?),(11425,8426),"maker price and buyer price improvement");
 let expired=call(owner,&mut admitted,12,Request{action:4,amount:1,price:1,operation:id(111),..Default::default()}).await?;assert_eq!(expired["pending"],false);
 sqlx::query("UPDATE game_coin_market_offers SET expires_at=clock_timestamp()-interval '1 second' WHERE offer_id=$1::uuid").bind(uuid(&id(111))).execute(&h.pool).await?;
 assert!(reconcile_coin_market(&worker,&mut restarted).await);assert_eq!(bank(h,id(40)).await?,11405);
 {
  let mut state=shared.lock().map_err(|_|"coin peer")?;let wallet=state.wallets.get_mut(&uuid(&id(40))).ok_or("seller wallet")?;wallet.transferable=0;wallet.non_transferable=100;
 }
 let refused=call(owner,&mut admitted,13,Request{action:3,amount:25,price:100,operation:id(112),..Default::default()}).await?;
 assert_eq!(refused["pending"],true);assert_eq!(refused["error"],"insufficient_transferable");assert_eq!(bank(h,id(40)).await?,11355);
 assert_eq!(shared.lock().map_err(|_|"coin peer")?.wallets.get(&uuid(&id(40))).ok_or("wallet")?.non_transferable,100);
 // A delayed reserve cannot open an already expired offer or consume its opening fee.
 sqlx::query("UPDATE game_coin_market_offers SET expires_at=clock_timestamp()-interval '1 second' WHERE offer_id=$1::uuid").bind(uuid(&id(112))).execute(&h.pool).await?;
 shared.lock().map_err(|_|"coin peer")?.wallets.get_mut(&uuid(&id(40))).ok_or("wallet")?.transferable=25;
 sqlx::query("UPDATE game_coin_operations SET next_attempt=clock_timestamp()").execute(&h.pool).await?;
 assert!(reconcile_coin_market(&worker,&mut restarted).await);assert!(reconcile_coin_market(&worker,&mut restarted).await);
 assert_eq!(bank(h,id(40)).await?,11405);
 let conflict=call(owner,&mut admitted,14,Request{action:3,amount:31,price:100,operation:id(101),..Default::default()}).await?;assert_eq!(conflict["result"],"idempotency_conflict");
 let sum:i64=sqlx::query_scalar("SELECT coalesce(sum(bank_delta+escrow_delta+burn),0)::bigint FROM game_coin_gold_events").fetch_one(&h.pool).await?;assert_eq!(sum,0);
 let fee:i64=sqlx::query_scalar("SELECT sum(burn)::bigint FROM game_coin_gold_events").fetch_one(&h.pool).await?;assert_eq!(fee,169);
 assert_eq!(bank(h,id(40)).await?+bank(h,buyer_account).await?,20000-fee);
 Ok(())
}
async fn bank(h:&Harness,account:[u8;16])->TestResult<i64>{Ok(sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(uuid(&account)).bind(uuid(&id(42))).fetch_one(&h.pool).await?)}
async fn seed_gold(h:&Harness,accounts:impl IntoIterator<Item=([u8;16],[u8;16],u8)>)->TestResult{
 let mut seed=h.pool.begin().await?;sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *seed).await?;
 for(account,character,tag)in accounts {
 sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('coin-seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,10000,0,1,sha256('coin-seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)")
 .bind(id(tag).as_slice()).bind(id(tag+1).as_slice()).bind(id(tag+2).as_slice()).bind(character.as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(tag+3).as_slice()).bind(id(tag+4).as_slice()).bind(id(tag+6).as_slice()).execute(&mut *seed).await?;
 sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,10000,0,10000,encode($5,'hex')::uuid)")
 .bind(id(tag+5).as_slice()).bind(id(tag+1).as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(character.as_slice()).execute(&mut *seed).await?;
 sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,10000,encode($3,'hex')::uuid)").bind(account.as_slice()).bind(id(42).as_slice()).bind(id(tag+5).as_slice()).execute(&mut *seed).await?;
 }
 seed.commit().await?;Ok(())
}
