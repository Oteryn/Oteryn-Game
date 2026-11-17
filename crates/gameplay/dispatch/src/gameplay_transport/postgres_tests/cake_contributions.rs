//! Source-backed stage-one substances through the real item owner and PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::consume_transform::{CakeBakerContext, ConsumeTransformRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[test]
fn w05d_cake_baker_all_carried_equipment_source_caps_and_replays_postgres() -> TestResult {
    case(true, false)
}
#[test]
fn w05d_cake_baker_all_carried_partial_equipment_extra_postgres() -> TestResult {
    case(false, false)
}
#[test]
fn w05d_cake_baker_two_hundred_nested_inputs_use_one_compact_audit_postgres() -> TestResult {
    case(false, true)
}

fn case(equipped: bool, tiny: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(equipped, tiny).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Cake PostgreSQL thread")??;
    Ok(())
}
fn uuid(number: u16) -> [u8; 16] {
    let mut result = id(100);
    result[14..16].copy_from_slice(&number.to_be_bytes());
    result
}
fn scenario(equipped: bool, tiny: bool) -> TestResult {
    run(async move |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "cake_baker_owner", true, false).await?;
        super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active native generation")?;
        let native = active.native_gameplay().ok_or("native")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'cake-fixture-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2030-02-21 06:00 Europe/Berlin','2030-02')").bind(id(42).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(42).as_slice())
            .execute(&h.pool)
            .await?;
        sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement,seasonal_calendar_revision,seasonal_event_keys) VALUES(encode($1,'hex')::uuid,1,true,'A Piece of Cake is active.',1,ARRAY['piece_of_cake'])").bind(id(42).as_slice()).execute(&h.pool).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i12143", 100)]).await?;
        let source = if tiny {
            uuid(200)
        } else if equipped {
            uuid(199)
        } else {
            id(71)
        };
        // The ordinary migration-owned inventory fixture is prepared administratively;
        // production mutation and all deferred guards are enabled for every call below.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        if tiny {
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).execute(&mut*tx).await?;
            sqlx::query(
                "DELETE FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(71).as_slice())
            .execute(&mut *tx)
            .await?;
        }
        let mut add_item = Vec::new();
        if tiny {
            for group in 0..10_u16 {
                add_item.push((uuid(100 + group), "oteryn:item.tibia.i2854", 1_i64));
                for i in 0..20 {
                    add_item.push((uuid(200 + group * 20 + i), "oteryn:item.tibia.i12143", 1));
                }
            }
        } else {
            add_item.push((uuid(100), "oteryn:item.tibia.i2854", 1));
            add_item.push((
                uuid(200),
                "oteryn:item.tibia.i12143",
                if equipped { 100 } else { 50 },
            ));
            add_item.push((
                uuid(199),
                "oteryn:item.tibia.i12143",
                if equipped { 50 } else { 100 },
            ));
        }
        for (item, key, quantity) in add_item {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)").bind(item.as_slice()).bind(id(42).as_slice()).bind(key).bind(quantity).bind(id(72).as_slice()).execute(&mut*tx).await?;
        }
        for group in 0..if tiny { 10_u16 } else { 1_u16 } {
            sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(uuid(100+group).as_slice()).bind(i64::from(group)+2).bind(id(72).as_slice()).execute(&mut*tx).await?;
            for i in 0..if tiny { 20_u16 } else { 1_u16 } {
                sqlx::query("INSERT INTO game_item_nested_entries(world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)").bind(id(42).as_slice()).bind(uuid(100+group).as_slice()).bind(uuid(200+group*20+i).as_slice()).bind(i64::from(i)+1).bind(id(72).as_slice()).execute(&mut*tx).await?;
            }
        }
        if !tiny {
            sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,5,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(uuid(199).as_slice()).bind(id(72).as_slice()).execute(&mut*tx).await?;
        }
        tx.commit().await?;
        let progress = h
            .root
            .cake_world_progress(owner.world_id, 1)
            .await?
            .ok_or("Cake starts February21")?;
        assert_eq!((progress.substances, progress.golems), (0, 0));
        let permit = owner
            .spell_lane_permit()
            .await
            .ok_or("current physical permit")?;
        let request = ConsumeTransformRequest {
            command: CommandRef::new(fence.game_session_id, CommandId::new(61).map_err(debug)?),
            source,
            target: None,
            intent: b"substance".to_vec(),
        };
        let context = CakeBakerContext {
            npc_key: "oteryn:npc.biff_the_baker".to_owned(),
            expected_epoch: 1,
        };
        let content = NativeItemWeights(native).with_wheel(owner.premium_coordinator);
        let donation_started = std::time::Instant::now();
        let first = h
            .root
            .consume_cake_substances(
                &permit,
                &authority,
                &h.node,
                fence,
                request.clone(),
                native,
                &content,
                &context,
            )
            .await?
            .ok_or("typed Cake outcome")?;
        let donation_elapsed_ms = donation_started.elapsed().as_millis();
        assert_eq!(first.disposition, UseDisposition::Committed);
        assert!(first.fresh);
        assert_eq!(first.effects["accepted"], 200);
        let progress = h
            .root
            .cake_world_progress(owner.world_id, 1)
            .await?
            .ok_or("same annual occurrence")?;
        assert_eq!((progress.substances, progress.golems), (200, 0));
        assert!(progress.phase_one_active());
        let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12143' AND lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(remaining, if tiny { 0 } else { 50 });
        let records:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:cake.baker-substances'").fetch_one(&h.pool).await?;
        assert_eq!(records, 1);
        let replay = h
            .root
            .consume_cake_substances(
                &permit,
                &authority,
                &h.node,
                fence,
                request.clone(),
                native,
                &content,
                &context,
            )
            .await?
            .ok_or("replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert!(!replay.fresh);
        assert_eq!(replay.effects, first.effects);
        let probed = h
            .root
            .reconcile_cake_substances(
                &permit,
                &authority,
                &h.node,
                fence,
                request.clone(),
                native,
                &content,
                &context,
            )
            .await?
            .ok_or("closed Cake receipt probe after source exhaustion")?;
        assert_eq!(probed.disposition, UseDisposition::Committed);
        assert!(!probed.fresh);
        assert_eq!(probed.effects, first.effects);
        for foreign in [
            CakeBakerContext {
                npc_key: "oteryn:npc.benny_the_baker".into(),
                expected_epoch: context.expected_epoch,
            },
            CakeBakerContext {
                npc_key: context.npc_key.clone(),
                expected_epoch: context.expected_epoch + 1,
            },
        ] {
            let refused = h
                .root
                .reconcile_cake_substances(
                    &permit,
                    &authority,
                    &h.node,
                    fence,
                    request.clone(),
                    native,
                    &content,
                    &foreign,
                )
                .await?
                .ok_or("foreign Cake context refusal")?;
            assert_eq!(refused.disposition, UseDisposition::Rejected);
        }
        use crate::durability::runtime_scope_assignment::{
            BootstrapSecret, NodeIncarnationProof, NodeRegistrationFact,
        };
        let stale_node = NodeIncarnationProof::new(
            NodeRegistrationFact::new(
                h.node.fact().node_id(),
                h.node.fact().registration_revision() + 1,
            ),
            BootstrapSecret::from_bytes([241; 32]),
        );
        let stale = h
            .root
            .consume_cake_substances(
                &permit,
                &authority,
                &stale_node,
                fence,
                request.clone(),
                native,
                &content,
                &context,
            )
            .await?
            .ok_or("stale node replay refusal")?;
        assert_ne!(stale.disposition, UseDisposition::Committed);
        let wrong_caller = h
            .root
            .consume_transform_detailed(
                &permit,
                &authority,
                &h.node,
                fence,
                request.clone(),
                native,
                &content,
                false,
            )
            .await?
            .ok_or("ordinary USE cannot replay NPC turn-in")?;
        assert_ne!(wrong_caller.disposition, UseDisposition::Committed);
        let mut changed = request;
        changed.intent = b"deliver changed".to_vec();
        let refused = h
            .root
            .consume_cake_substances(
                &permit, &authority, &h.node, fence, changed, native, &content, &context,
            )
            .await?
            .ok_or("conflicting replay")?;
        assert_ne!(refused.disposition, UseDisposition::Committed);
        let audits:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox a JOIN game_cake_donation_receipts r USING(transaction_id)").fetch_one(&h.pool).await?;
        assert_eq!(audits, 1);
        if tiny {
            // The optimized deferred proof must still reject an omitted input,
            // a forged decrement and drift in otherwise unchanged metadata.
            for (path, value) in [
                (
                    vec!["extra_inputs", "0", "item"],
                    serde_json::json!("00000000-0000-0000-0000-000000000000"),
                ),
                (
                    vec!["extra_inputs", "0", "before", "quantity"],
                    serde_json::json!(2),
                ),
                (
                    vec!["extra_inputs", "0", "before", "world_id"],
                    serde_json::json!("00000000-0000-0000-0000-000000000000"),
                ),
            ] {
                let rejected=sqlx::query("SELECT game_cake_donation_extra_complete(jsonb_populate_record(NULL::game_item_consume_transform_records,jsonb_set(to_jsonb(r),$1,$2))) FROM game_item_consume_transform_records r WHERE recipe='oteryn:cake.baker-substances'")
                    .bind(path).bind(value).execute(&h.pool).await;
                let Err(sqlx::Error::Database(error)) = rejected else {
                    return Err(
                        "tampered Cake completion did not produce a database constraint refusal"
                            .into(),
                    );
                };
                assert_eq!(error.code().as_deref(), Some("23514"));
            }
        }
        if !tiny {
            let revision:String=sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(revision, "2");
        }
        eprintln!(
            "W05D_CAKE_BAKER_PG_EXECUTED equipment_source={equipped} tiny200={tiny} donation_elapsed_ms={donation_elapsed_ms} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        drop(permit);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
