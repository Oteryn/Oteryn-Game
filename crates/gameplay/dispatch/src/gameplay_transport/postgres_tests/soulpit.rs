//! W07: existing USE/use-with bytes, two-input custody, immutable replay and source refusals.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use tokio::io::AsyncWriteExt;

#[test]
fn soulpit_craft_wire_custody_replay_refusals_and_ground_delivery() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SOULPIT POSTGRES_CASE_NOT_RUN: no configured PostgreSQL route");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join().map_err(|_| "Soulpit craft PostgreSQL scenario panicked")??;
    eprintln!("SOULPIT POSTGRES_CASE_COMPLETED: wire custody replay refusals ground delivery");
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "soulpit_craft", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let rat = "oteryn:item.tibia.i47398";
        let orc = "oteryn:item.tibia.i47384";
        let dragon = "oteryn:item.tibia.i47411";
        let prism = "oteryn:item.tibia.i49164";
        let exalted = "oteryn:item.tibia.i37110";
        let white_lion = "oteryn:item.tibia.i49038";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) = &policy.semantics.physical else { return None; };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
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
            content_pin,
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
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(
            &harness,
            &[(rat,4),(orc,2),(dragon,2),(prism,2),(exalted,2),(white_lion,2),("oteryn:item.tibia.i3577",1)],
        )
        .await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let refs = |source: &str, target: &str| -> TestResult<_> {
            Ok((reference(source).ok_or("source Item ref")?, reference(target).ok_or("target Item ref")?))
        };
        let initial = admitted;
        let placeholder = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
            use_with_position: None,
        }).map_err(debug)?;

        // A stale first USE reaches the real Soulpit refusal path with otherwise valid
        // inputs. On a resumed connection, setup::live first reconciles NPC purchases;
        // character_promotion::read_premium_purchase_state refuses this stale fence before
        // receipt lookup, so command::execute closes before USE routing. Do not weaken that
        // unrelated owner or mistake its EOF for a Soulpit result. Rebuild the composed
        // owner afterward, as the durable replay cases below simulate process restart.
        let before_stale = custody_snapshot(&harness).await?;
        let mut stale = initial;
        let mut stale_fence = fence;
        stale_fence.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
        stale.item_fence = Some(stale_fence);
        let (_, result, _) = exchange(&owner, stale, 1, &placeholder, Some(refs(rat, orc)?)).await?;
        assert_eq!(result, UseDisposition::Rejected);
        assert_eq!(custody_snapshot(&harness).await?, before_stale,
            "fresh stale generation changed inventory, custody, Ground, a receipt or audit");
        assert_eq!(receipt_count(&harness).await?, 0);
        assert_eq!(quantity(&harness, 71).await?, 4);
        assert_eq!(quantity(&harness, 72).await?, 2);
        drop(owner);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;

        // Two distinct inputs produce one receipt, one Item and one bounded audit envelope.
        let (_, result, first_payload) = exchange(&owner, admitted, 1, &placeholder, Some(refs(rat,orc)?)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(quantity(&harness,71).await?,3);
        assert_eq!(quantity(&harness,72).await?,1);
        let first = receipt(&harness,1).await?;
        assert_eq!(first.0,"oteryn:soulpit.fuse");
        assert_eq!(first.1["inputs"].as_array().ok_or("two inputs")?.len(),2);
        assert!(first.1["inputs"].as_array().ok_or("inputs")?.iter().all(|input|input["consume"]==1));
        assert_eq!(first.1["delivery"]["kind"],"container");
        assert_eq!(first.2.as_array().ok_or("outputs")?.len(),1);
        assert_eq!(first.2[0]["count"],1);
        let (envelope_size, matching_hash):(i32,bool)=sqlx::query_as(
            "SELECT octet_length(a.envelope),a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id) WHERE r.command_id=1"
        ).fetch_one(&harness.pool).await?;
        assert!(envelope_size<=9216 && matching_hash);
        let output_id=first.2[0]["item"].as_str().ok_or("output UUID")?.to_owned();

        // Rebuild the composed owner; receipt admission precedes current carried handle lookup.
        drop(owner);
        let mut owner=deps.owner(&harness,&authority);
        owner.active_generation=Some(active);
        owner.movement_cells=&cells;
        deps.initialize(&owner).await;
        let (_, replayed, _) = exchange(&owner,initial,1,&first_payload,None).await?;
        assert_eq!(replayed,UseDisposition::Committed);
        assert_eq!(receipt(&harness,1).await?,first);
        assert_eq!(receipt_count(&harness).await?,1);
        // Historical success precedes entry-only live generation checks. The fresh stale
        // command below still refuses; this retry must only read its original receipt.
        // A fresh connection owner makes this reach durable custody admission instead of
        // replaying the previous owner's transport-retained result.
        drop(owner);
        let mut owner=deps.owner(&harness,&authority);
        owner.active_generation=Some(active);
        owner.movement_cells=&cells;
        deps.initialize(&owner).await;
        let mut historical = initial;
        let mut historical_fence = fence;
        historical_fence.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(2).map_err(debug)?;
        historical.item_fence = Some(historical_fence);
        let (_, replayed, _) = exchange(&owner,historical,1,&first_payload,None).await?;
        assert_eq!(replayed,UseDisposition::Committed);
        assert_eq!(receipt(&harness,1).await?,first);
        assert_eq!(receipt_count(&harness).await?,1);
        drop(owner);
        let mut owner=deps.owner(&harness,&authority);
        owner.active_generation=Some(active);
        owner.movement_cells=&cells;
        deps.initialize(&owner).await;
        let mut changed=oteryn_protocol_oteryn::world_object::decode_use_intent_target(initial.continuity.selected_capabilities.as_slice(),&first_payload).map_err(debug)?;
        changed.use_with_map_item=Some(MapUseTarget::Item(std::num::NonZeroU64::MAX));
        let (next, conflict, _) = exchange(&owner,initial,1,&encode_item_use_intent(&changed).map_err(debug)?,None).await?;
        assert_eq!(conflict,UseDisposition::Rejected);
        assert_eq!(receipt_count(&harness).await?,1);
        assert_eq!(quantity(&harness,71).await?,3);

        // A single instance used on itself consumes exactly two units and records one input.
        let (next,result,_) = exchange(&owner,next,2,&placeholder,Some(refs(rat,rat)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        assert_eq!(quantity(&harness,71).await?,1);
        let second=receipt(&harness,2).await?;
        assert_eq!(second.1["inputs"].as_array().ok_or("one input")?.len(),1);
        assert_eq!(second.1["inputs"][0]["consume"],2);
        let (next,result,_) = exchange(&owner,next,3,&placeholder,Some(refs(rat,rat)?)).await?;
        assert_eq!(result,UseDisposition::NotPossible);
        assert_eq!(quantity(&harness,71).await?,1);
        assert_eq!(receipt_count(&harness).await?,2);

        // Literal source names distinguish registered cores, usable inputs, and fusion outputs.
        let (next,result,_) = exchange(&owner,next,4,&placeholder,Some(refs(white_lion,orc)?)).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        let (next,result,_) = exchange(&owner,next,5,&placeholder,Some(refs(orc,prism)?)).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        assert_eq!(quantity(&harness,76).await?,2);
        assert_eq!(quantity(&harness,74).await?,2);
        assert_eq!(receipt_count(&harness).await?,2);

        // The full source roster: Rat(stars1)->stars2 and Dragon(stars3)->stars2.
        // Both draws traverse all159 source candidates, independently of native creatures.
        let (next,result,_) = exchange(&owner,next,6,&placeholder,Some(refs(prism,rat)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        let prism_receipt=receipt(&harness,6).await?;
        assert_eq!(prism_receipt.0,"oteryn:soulpit.prism");
        assert_eq!(prism_receipt.2[0]["key"],"oteryn:item.tibia.i47732");
        assert_eq!(quantity(&harness,74).await?,1);
        assert_eq!(quantity(&harness,71).await?,0);
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?,2);
        let (next,result,_) = exchange(&owner,next,7,&placeholder,Some(refs(exalted,dragon)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        let exalted_receipt=receipt(&harness,7).await?;
        assert_eq!(exalted_receipt.0,"oteryn:soulpit.exalted");
        assert_eq!(exalted_receipt.2[0]["key"],"oteryn:item.tibia.i47386");
        assert_eq!(quantity(&harness,75).await?,1);
        assert_eq!(quantity(&harness,73).await?,1);

        // Seed an overloaded but otherwise valid carried tree. Prism's source capacity gate
        // runs before consuming either input; it cannot use future freed weight as capacity.
        let saved:Vec<(String,i64)>=sqlx::query_as("SELECT item_instance_id::text,quantity FROM game_item_instances WHERE lifecycle=1 AND definition_production_key<>$1 AND definition_production_key<>$2")
            .bind(bag).bind("oteryn:item.tibia.i3577").fetch_all(&harness.pool).await?;
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&harness.pool).await?;
        let mut tx=harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET level=2 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        for (item,_) in &saved {
            sqlx::query("UPDATE game_item_instances SET quantity=100 WHERE item_instance_id=$1::uuid").bind(item).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let (next,result,_) = exchange(&owner,next,8,&placeholder,Some(refs(prism,dragon)?)).await?;
        assert_eq!(result,UseDisposition::NotPossible);
        assert_eq!(quantity(&harness,74).await?,100);
        assert_eq!(quantity(&harness,73).await?,100);
        assert_eq!(receipt_count(&harness).await?,4);
        let mut tx=harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET level=$2 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(level).execute(&mut *tx).await?;
        for (item,quantity) in &saved {
            sqlx::query("UPDATE game_item_instances SET quantity=$2 WHERE item_instance_id=$1::uuid").bind(item).bind(quantity).execute(&mut *tx).await?;
        }
        tx.commit().await?;

        // Full carried container: addItem is attempted before burning inputs and falls to the
        // current pinned ground cell. One-unit inputs therefore do not pre-free carried slots.
        fill_backpack(&harness).await?;
        let (next,result,ground_payload) = exchange(&owner,next,9,&placeholder,Some(refs(orc,dragon)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        let ground=receipt(&harness,9).await?;
        assert_eq!(ground.1["delivery"]["kind"],"ground");
        assert_eq!(quantity(&harness,72).await?,0);
        assert_eq!(quantity(&harness,73).await?,0);
        let ground_item=ground.2[0]["item"].as_str().ok_or("ground output")?;
        let location:(Vec<u8>,String,Vec<u8>)=sqlx::query_as("SELECT spatial_position,content_revision,native_room_placement_context FROM game_item_ground_locations WHERE item_instance_id=$1::uuid").bind(ground_item).fetch_one(&harness.pool).await?;
        let mut expected_position=Vec::new();
        expected_position.extend_from_slice(&6_i32.to_be_bytes());
        expected_position.extend_from_slice(&0_i32.to_be_bytes());
        expected_position.extend_from_slice(&(-7_i16).to_be_bytes());
        assert_eq!(location.0,expected_position);
        assert!(location.1.starts_with("sha256:"));
        assert_eq!(location.2,pin.frame_binding().digest());
        // Duplicates retain the current ingress high-water and advance only the server
        // sequence. Rewinding connection continuity would be rejected before dispatch.
        let (next,replayed,_)=exchange(&owner,next,9,&ground_payload,None).await?;
        assert_eq!(replayed,UseDisposition::Committed);
        assert_eq!(receipt_count(&harness).await?,5);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=$1::uuid").bind(ground_item).fetch_one(&harness.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances WHERE item_instance_id=$1::uuid").bind(&output_id).fetch_one(&harness.pool).await?,1);

        // White Lion is in the output/entry catalogue, but its literal source name lacks
        // " soul core" and therefore cannot be an Exalted conversion target. Keep the
        // current fence and this command slot so later deterministic draws are unchanged.
        let (next,result,_)=exchange(&owner,next,10,&placeholder,Some(refs(exalted,white_lion)?)).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        assert_eq!(receipt_count(&harness).await?,5);
        assert_eq!(quantity(&harness,75).await?,1);
        assert_eq!(quantity(&harness,76).await?,2);

        // A full bag still permits addItem to merge into a compatible, not-yet-full input.
        // This fixture's command11 draw is Walking Dread52809: two debits and one credit to
        // the same instance leave one unit, without a fresh Item identity or ground output.
        fill_backpack(&harness).await?;
        let merge_core="oteryn:item.tibia.i52809";
        let mut tx=harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_instances SET definition_production_key=$2,quantity=2 WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(76).as_slice()).bind(merge_core).execute(&mut *tx).await?;
        tx.commit().await?;
        let item_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances").fetch_one(&harness.pool).await?;
        let (next,result,merge_payload)=exchange(&owner,next,11,&placeholder,Some(refs(merge_core,merge_core)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        let merged=receipt(&harness,11).await?;
        assert_eq!(merged.1["delivery"]["kind"],"stack");
        assert_eq!(merged.1["inputs"][0]["consume"],2);
        assert_eq!(merged.1["credit"]["before"]["quantity"],2);
        assert_eq!(merged.2[0]["key"],merge_core);
        assert_eq!(quantity(&harness,76).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances").fetch_one(&harness.pool).await?,item_count);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_ground_locations").fetch_one(&harness.pool).await?,1);
        let (next,result,_)=exchange(&owner,next,11,&merge_payload,None).await?;
        assert_eq!(result,UseDisposition::Committed);
        assert_eq!(quantity(&harness,76).await?,1);
        assert_eq!(receipt_count(&harness).await?,6);

        // ANSWER6542983549 keeps unbound source identities in the835-entry draw. This
        // command selects pirate gunner53143, which must refuse without redrawing or debit.
        replace_fixture_item(&harness,77,111,orc,2).await?;
        let (next,result,_)=exchange(&owner,next,12,&placeholder,Some(refs(orc,merge_core)?)).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        assert_eq!(quantity(&harness,111).await?,2);
        assert_eq!(quantity(&harness,76).await?,1);
        assert_eq!(receipt_count(&harness).await?,6);

        // The source's2% ominous branch preserves the target core while consuming the prism.
        // The seeded command13 roll is2; its ordinary selected core exists before this draw.
        replace_fixture_item(&harness,111,211,rat,1).await?;
        let (next,result,ominous_payload)=exchange(&owner,next,13,&placeholder,Some(refs(prism,rat)?)).await?;
        assert_eq!(result,UseDisposition::Committed);
        let ominous=receipt(&harness,13).await?;
        assert_eq!(ominous.0,"oteryn:soulpit.prism");
        assert_eq!(ominous.2[0]["key"],"oteryn:item.tibia.i49163");
        let inputs=ominous.1["inputs"].as_array().ok_or("ominous custody inputs")?;
        assert_eq!(inputs.len(),2);
        assert_eq!(inputs.iter().filter(|input|input["consume"]==0).count(),1);
        assert_eq!(inputs.iter().filter(|input|input["consume"]==1).count(),1);
        assert_eq!(quantity(&harness,74).await?,0);
        assert_eq!(quantity(&harness,211).await?,1);
        let (next,result,_)=exchange(&owner,next,13,&ominous_payload,None).await?;
        assert_eq!(result,UseDisposition::Committed);
        assert_eq!(quantity(&harness,74).await?,0);
        assert_eq!(quantity(&harness,211).await?,1);
        assert_eq!(receipt_count(&harness).await?,7);

        // ANSWER6543406061 preserves every source Bestiary candidate before Item admission.
        // Exalted on Rat(stars1) follows the donor's numeric-key fallback to stars0. This
        // command draws index0 of all12 candidates: race0 Mushroom/Pillar, neither with a
        // core. Shrinking the pool or redrawing would wrongly consume this valid input pair.
        replace_fixture_item(&harness,211,215,rat,1).await?;
        let item_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
            .fetch_one(&harness.pool).await?;
        let audit_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&harness.pool).await?;
        let (after,result,missing_core_payload)=exchange(&owner,next,14,&placeholder,Some(refs(exalted,rat)?)).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        assert_eq!(quantity(&harness,75).await?,1);
        assert_eq!(quantity(&harness,215).await?,1);
        assert_eq!(receipt_count(&harness).await?,7);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_instances")
            .fetch_one(&harness.pool).await?,item_count);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_ground_locations")
            .fetch_one(&harness.pool).await?,1);
        let (_,result,_)=exchange(&owner,after,14,&missing_core_payload,None).await?;
        assert_eq!(result,UseDisposition::NothingToUse);
        assert_eq!(quantity(&harness,75).await?,1);
        assert_eq!(quantity(&harness,215).await?,1);
        assert_eq!(receipt_count(&harness).await?,7);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&harness.pool).await?,audit_count);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn custody_snapshot(harness: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object( \
          'items',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_instances r), \
          'backpacks',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_container_slots r), \
          'carried',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_container_entries r), \
          'nested',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_nested_entries r), \
          'equipment',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_character_equipment_slots r), \
          'ground',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_ground_locations r), \
          'drop_channels',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_loop_drop_channels r), \
          'receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_craft_records r), \
          'audit',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text),'[]'::jsonb) FROM game_item_audit_outbox r))",
    ).fetch_one(&harness.pool).await?)
}

async fn quantity(harness:&Harness,seed:u8)->TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(seed).as_slice()).fetch_one(&harness.pool).await?)
}
async fn receipt_count(harness:&Harness)->TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await?)
}
async fn receipt(harness:&Harness,command:u64)->TestResult<(String,serde_json::Value,serde_json::Value)> {
    Ok(sqlx::query_as("SELECT recipe,soul_core_proof,outputs FROM game_item_craft_records WHERE command_id=$1::text::numeric")
        .bind(command.to_string()).fetch_one(&harness.pool).await?)
}
async fn fill_backpack(harness:&Harness)->TestResult {
    let mut tx=harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let (occupied,highest):(i64,i64)=sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::bigint FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(id(70).as_slice()).fetch_one(&mut *tx).await?;
    assert!(occupied<=20);
    for offset in occupied..20 {
        sqlx::query("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3600','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,game_character_uuid_v7() FROM i")
            .bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(highest+offset-occupied+1).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Change only fixture custody, before issuing the tested real command. Distinct source UUIDs
/// fix server entropy so rare source branches are reproducible without mocking the writer.
async fn replace_fixture_item(harness:&Harness,previous:u8,next:u8,key:&str,quantity:i64)->TestResult {
    let mut tx=harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(previous).as_slice()).bind(id(next).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($2,'hex')::uuid,definition_production_key=$3,quantity=$4 WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(previous).as_slice()).bind(id(next).as_slice()).bind(key).bind(quantity).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    current_items: Option<(std::num::NonZeroU32,std::num::NonZeroU32)>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    eprintln!("SOULPIT WIRE_BEGIN command={command_id} current_items={} scope_generation={:?}",
        current_items.is_some(), admitted.item_fence.as_ref().map(|fence| fence.scope_ownership_generation.get()));
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut target = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await
                .map_err(|error| format!("Soulpit command{command_id} snapshot read: {error}"))?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                        if let Some((source_definition,target_definition)) = current_items {
                            source = inventory.entries.iter().find(|entry| entry.item_definition_ref == source_definition).map(|entry| entry.handle);
                            target = inventory.entries.iter().find(|entry| entry.item_definition_ref == target_definition).map(|entry| entry.handle);
                        }
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit { break; }
        }
        eprintln!("SOULPIT WIRE_SNAPSHOT_READY command={command_id}");
        let actual = if current_items.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(admitted.continuity.selected_capabilities.as_slice(), payload).map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            intent.use_with_map_item = Some(MapUseTarget::Item(target.ok_or("current target handle")?));
            encode_item_use_intent(&intent).map_err(debug)?
        } else { payload.to_vec() };
        let frame = oteryn_protocol_oteryn::encode_client_command(1, &oteryn_protocol_oteryn::ClientCommandValue { command_id, command_type: 2, payload: &actual })?;
        client.write_u32(u32::try_from(frame.len())?).await
            .map_err(|error| format!("Soulpit command{command_id} command length write: {error}"))?;
        client.write_all(&frame).await
            .map_err(|error| format!("Soulpit command{command_id} command frame write: {error}"))?;
        client.shutdown().await
            .map_err(|error| format!("Soulpit command{command_id} command half-close: {error}"))?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await
                .map_err(|error| format!("Soulpit command{command_id} result read: {error}"))?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(result.payload).map_err(debug)?;
                    eprintln!("SOULPIT WIRE_RESULT command={command_id} disposition={disposition:?}");
                    return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        let label=match &end {
            ConnectionEnd::ProtocolViolation(_)=>"protocol_violation",
            ConnectionEnd::AdmissionRefused(_)=>"admission_refused",
            ConnectionEnd::ResumeUnavailable=>"resume_unavailable",
            ConnectionEnd::AdmittedThenClosed(_,_)=>"admitted_closed",
            ConnectionEnd::AdmittedThenDisconnected(_)=>"disconnected",
            ConnectionEnd::AdmittedThenControlLost(_)=>"control_lost",
            ConnectionEnd::AdmittedThenLoggedOut(_)=>"logged_out",
            ConnectionEnd::TransportFailed=>"transport_failed",
        };
        eprintln!("SOULPIT WIRE_SERVER_END command={command_id} end={label}");
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(std::time::Duration::from_secs(30), std::future::poll_fn(|context| {
        if end.is_none() && let Poll::Ready(value) = serving.as_mut().poll(context) {
            end = Some(value);
        }
        if result.is_none() && let Poll::Ready(value) = client_task.as_mut().poll(context) {
            result = Some(value);
        }
        if end.is_some() && result.is_some() { Poll::Ready(()) } else { Poll::Pending }
    })).await.map_err(|error| format!("Soulpit command{command_id} exchange timeout: {error}"))?;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else { return Err(format!("{end:?}").into()); };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}
