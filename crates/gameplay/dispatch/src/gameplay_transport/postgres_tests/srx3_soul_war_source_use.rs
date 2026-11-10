//! Real source Sanity USE must fuse its Quest-only wrong-target cooldown.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

#[test]
fn srx3_soul_war_sanity_wrong_native_target_commits_cooldown_without_burn_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-sanity-source-use", || {
        run(async |admin| {
            super::reset();
            // Exact source walkable tile next to Murcion's literal flame.
            let (scene, activation_pin) = SourceScene::new_with_activation(
                PinnedActivitySourceRoom::Bakragore,
                MovementLocalPosition {
                    x: 33084,
                    y: 32398,
                    floor: 15,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_sanity_use", false, false).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            // Imported historical inventory only, before fresh authority proof.
            super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i33950", 1)]).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Bakragore)?;
            assert_ne!(
                activation_pin.frame_binding_digest(),
                scene.pin.frame_binding_digest(),
                "real native-entry issuance and bundle runtime must keep their distinct frames"
            );
            let map_revision = crate::map::overlay::map_revision(
                owner
                    .bundle_map()
                    .ok_or("actual source bundle map")?
                    .overlay
                    .base(),
            );
            super::boss_activity::issue_content_and_reset_with_runtime_pin(
                &h,
                &activation_pin,
                &scene.pin,
                &map_revision,
            )
            .await?;
            let catalogue =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &[4, 6, 7, 10, 11, 15, 18, 23, 92, 117],
                )
                .ok_or("actual source USE capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let native = scene
                .controller
                .active()
                .and_then(|g| g.native_gameplay())
                .ok_or("native policy")?;
            let source_item = native
                .item_index()
                .and_then(|index| {
                    index.definition_ref("Item", "oteryn:item.tibia.i33950", "definition-r1")
                })
                .ok_or("actual carried Cleansed Sanity")?;
            let map = owner.bundle_map().ok_or("actual captured public map")?;
            let tile = crate::map::overlay::TilePos {
                x: 33085,
                y: 32398,
                floor: -15,
            };
            let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("actual source flame tile")?;
            let mut selected = stack
                .entries
                .iter()
                .filter(|entry| entry.facts.appearance_id == 43983);
            let entry = selected.next().ok_or("actual inactive Murcion flame")?;
            assert!(selected.next().is_none());
            let crate::map::view::EntrySource::Base {
                placement_key: key, ..
            } = entry.source
            else {
                return Err("source flame must be its actual fixed WorldObject".into());
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            };
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(source_item),
                use_with: None,
                use_with_map_item: Some(
                    oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target),
                ),
                use_with_position: None,
            };
            let actor = admitted
                .runtime_actor
                .ok_or("actual current Source USE actor")?;
            let command = crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1,
                item_fence: admitted.item_fence,
            };
            let (actual, qualified, _, _)=owner.qualify_soul_war_source_use(actor,command,&intent,None,None)
                .await.ok_or("actual native Source5 target qualification must reach the paired writer boundary")?;
            assert_eq!(
                actual.item.definition.production_key,
                "oteryn:item.tibia.i33950"
            );
            assert_eq!(
                qualified.source_use(),
                crate::durability::item_use::soul_war_source::SourceUse::CleansedSanity
            );
            assert!(
                !qualified.target_accepted(),
                "real adjacent flame is a valid wrong Sanity target"
            );
            assert_eq!(
                qualified.runtime_scope(),
                owner.runtime.lock().await.binding().runtime_scope()
            );
            let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
                .map_err(debug)?;
            let reply =
                super::quest_world_callbacks::wire(&owner, &mut admitted, 1, 2, payload.clone())
                    .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::NotPossible,
                "source Sanity returns false on a wrong target despite committing its cooldown"
            );
            let blocked =
                super::quest_world_callbacks::wire(&owner, &mut admitted, 2, 2, payload).await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&blocked).map_err(debug)?,
                UseDisposition::NotPossible,
                "the actual thirty-second cooldown rejects another source USE"
            );
            let track = oteryn_gameplay_quests::quest::soul_war::source_items::SANITY;
            let cooldown: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(id(41).as_slice()).bind(track).fetch_optional(&h.pool).await?;
            assert!(
                cooldown.is_some_and(|at| at > 0),
                "wrong-target action failure must still write its real source cooldown"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i33950' AND i.lifecycle=1 AND i.quantity=1")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?,1,
                "wrong-target Sanity retains the original physical item");
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/soul-war/source-items/cleansed-sanity'")
                .fetch_one(&h.pool).await?,1);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_soul_war_source_use_proofs WHERE source_key='oteryn:item.tibia.i33950' AND NOT accepted")
                .fetch_one(&h.pool).await?,1,
                "cooldown refusal must roll back the second selected-target witness");
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_use_records WHERE cause_kind=5"
                )
                .fetch_one(&h.pool)
                .await?,
                0
            );
            source_pair_refusals(&h).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_SOUL_WAR_SANITY_USE_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// Exercise the new deferred reciprocal boundary with the live authority and
/// custody from the real wrong-target USE above. These are attempted receipt
/// forgeries; force this guard specifically, then roll back without admitting
/// them through any of the independent Item/Quest commit-time guards.
async fn source_pair_refusals(h: &Harness) -> TestResult {
    let original = sqlx::query_scalar::<_, sqlx::types::Json<serde_json::Value>>(
        "SELECT to_jsonb(p) FROM game_soul_war_source_use_proofs p WHERE source_key='oteryn:item.tibia.i33950' AND NOT accepted",
    ).fetch_one(&h.pool).await?.0;
    let columns: Vec<String> = sqlx::query_scalar(
        "SELECT attname::text FROM pg_attribute WHERE attrelid='game_soul_war_source_use_proofs'::regclass \
         AND attnum>0 AND NOT attisdropped AND attgenerated='' ORDER BY attnum",
    ).fetch_all(&h.pool).await?;
    let insert = format!(
        "INSERT INTO game_soul_war_source_use_proofs ({}) SELECT {} FROM jsonb_populate_record(NULL::game_soul_war_source_use_proofs,$1::jsonb) p",
        columns.join(","),
        columns
            .iter()
            .map(|column| format!("p.{column}"))
            .collect::<Vec<_>>()
            .join(","),
    );
    for (command, mutation, expected) in [
        (
            901,
            "missing-burn",
            "accepted source USE requires its exact same-xact one-unit burn",
        ),
        (
            902,
            "false-with-burn",
            "wrong-target Sanity must retain its exact current carried Item without a burn",
        ),
        (
            903,
            "frame_digest",
            "source USE requires its exact current session and qualified source scope",
        ),
        (
            904,
            "map_digest",
            "source USE requires its exact current session and qualified source scope",
        ),
    ] {
        let transaction = crate::operator_files::uuid_v7().map_err(debug)?;
        let event = crate::operator_files::uuid_v7().map_err(debug)?;
        let mut proof = original.clone();
        proof["transaction_id"] = sqlx::types::Uuid::from_bytes(transaction)
            .to_string()
            .into();
        proof["event_id"] = sqlx::types::Uuid::from_bytes(event).to_string().into();
        proof["command_id"] = serde_json::json!(command);
        if mutation == "missing-burn" {
            proof["target_kind"] = serde_json::json!(1);
            proof["target_family"] = "Item".into();
            proof["target_key"] = "oteryn:item.tibia.i33890".into();
            proof["target_revision"] = "definition-r1".into();
            proof["target_actor"] = serde_json::Value::Null;
            proof["target_generation"] = serde_json::Value::Null;
        } else if matches!(mutation, "frame_digest" | "map_digest") {
            let different = serde_json::Value::String(format!("\\x{}", "a5".repeat(32)));
            assert_ne!(proof[mutation], different);
            proof[mutation] = different;
        }
        let mut tx = h.pool.begin().await?;
        sqlx::query(&insert)
            .bind(sqlx::types::Json(proof))
            .execute(&mut *tx)
            .await?;
        if mutation == "false-with-burn" {
            // A false target cannot be used to conceal a consuming receipt,
            // even when every identity/scope/custody field names its exact USE.
            sqlx::query(
                "INSERT INTO game_item_use_records (transaction_id,event_id,cause_kind,game_session_id,command_id,\
                  content_revision,ruleset_revision,sim_revision,request_binding,character_id,world_id,channel_id,\
                  runtime_scope_ownership_generation,expected_session_generation,shape,source_item_instance_id,\
                  source_definition_family,source_definition_production_key,source_definition_revision_ref,\
                  source_parent_item_instance_id,source_placement_ordinal,source_quantity_before,source_quantity_after,\
                  occurred_at,envelope_sha256,activity_instance_id) \
                 SELECT transaction_id,event_id,5,game_session_id,command_id,content_revision,ruleset_revision,sim_revision,\
                  item_request_binding,character_id,world_id,channel_id,scope_generation,connection_generation,1,source_item_instance_id,\
                  'Item',source_key,'definition-r1',source_parent_item_instance_id,source_placement_ordinal,source_quantity_before,\
                  source_quantity_before-1,occurred_at,decode(repeat('01',32),'hex'),activity_instance_id \
                 FROM game_soul_war_source_use_proofs WHERE transaction_id=encode($1,'hex')::uuid",
            ).bind(transaction.as_slice()).execute(&mut *tx).await?;
        }
        let error = sqlx::query("SET CONSTRAINTS game_soul_war_source_use_proof_pair IMMEDIATE")
            .execute(&mut *tx)
            .await
            .err()
            .ok_or("inconsistent source USE proof was accepted")?;
        let database = error
            .as_database_error()
            .ok_or("source pair refusal must be a database constraint")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        assert!(database.message().contains(expected), "{error:?}");
        tx.rollback().await?;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_soul_war_source_use_proofs")
            .fetch_one(&h.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_use_records WHERE cause_kind=5"
        )
        .fetch_one(&h.pool)
        .await?,
        0
    );
    Ok(())
}
