//! REWARD-CHEST-WIRE-1: USE on a carried reward bag → real composed owner → one atomic
//! consume/mint → durable rows; replay of the consumed bag and a capacity refusal.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_items::reward_bag::{reward_bag_conversion, reward_bag_draw};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget, encode_item_use_intent};

#[test]
fn reward_bag_use_wire_atomic_replay_and_capacity_refusal() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "reward bag e2e panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "reward_bag", true, false).await?;
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
        let covet = "oteryn:item.tibia.i43895";
        super::item_look::seed_main_backpack(&harness, &[(covet, 1)]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("reward bag offer selectable")?;
        let bag_definition = crate::gameplay_transport::item_look::typed_definition(index, reference(covet).ok_or("bag reference")?).ok_or("bag definition")?;
        let draw = reward_bag_draw(*fence.game_session_id.as_bytes(), 1, id(71));
        let expected = reward_bag_conversion(native, &bag_definition, draw).ok_or("admitted reward bag")?;
        let reward = expected.outputs[0].0.production_key.clone();
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let craft_count = async || sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records").fetch_one(&harness.pool).await;
        let (end, result, _) = exchange(&owner, admitted, 1, &payload, reference(covet)).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(craft_count().await?, 1);
        // The used bag is retired and its slot holds exactly the drawn reward.
        let retired: (i64, i16, i64) = sqlx::query_as("SELECT quantity,lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, (0, 2, 0));
        let durable = harness.root.read_character_backpack(&authority, fence.character_id).await?.ok_or("durable backpack")?;
        let keys: Vec<_> = durable.entries.iter().map(|e| (e.item.definition.production_key.as_str(), e.item.quantity)).collect();
        assert_eq!(keys, [(reward.as_str(), 1)]);
        let (recipe, outputs, hash_matches): (String, serde_json::Value, bool) = sqlx::query_as(
            "SELECT r.recipe,r.outputs,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(recipe, "oteryn:reward-bag.bag_you_covet");
        assert_eq!(outputs[0]["key"], reward.as_str());
        assert!(hash_matches);
        // Over capacity: a second bag stays carried and nothing is minted or recorded.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let levelled = sqlx::query("UPDATE game_character_progression_state SET level=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&mut *tx).await?;
        assert_eq!(levelled.rows_affected(), 1);
        let mut seeded = Vec::new();
        for (n, key) in std::iter::once(covet).chain(std::iter::repeat_n("oteryn:item.tibia.i43872", 7)).enumerate() {
            seeded.push(sqlx::query_scalar::<_, String>("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,'definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7() FROM i RETURNING item_instance_id::text")
                .bind(id(42).as_slice()).bind(key).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(100_i64 + i64::try_from(n)?).fetch_one(&mut *tx).await?);
        }
        tx.commit().await?;
        let mut next = end;
        next.continuity.next_command_id = 2;
        let (_, result, _) = exchange(&owner, next, 2, &payload, reference(covet)).await?;
        assert_eq!(result, UseDisposition::NotPossible);
        assert_eq!(craft_count().await?, 1);
        let kept: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=$1::uuid").bind(&seeded[0]).fetch_one(&harness.pool).await?;
        assert_eq!(kept, (1, 1));
        let bag_two: [u8; 16] = sqlx::query_scalar("SELECT uuid_send($1::uuid)").bind(&seeded[0]).fetch_one(&harness.pool).await?;
        // A restarted owner replays the consumed bag's receipt without redrawing or minting.
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        deps.initialize(&restarted).await;
        let replayed = restarted
            .use_item_completion(deps.actor, UseCommand { game_session_id: deps.admitted.game_session_id, command_id: 1, item_fence: Some(fence) }, ItemKey::Instance(id(71)))
            .await
            .ok_or("replay outcome")?;
        assert_eq!(replayed.disposition, UseDisposition::Committed);
        let refused = restarted
            .use_item_completion(deps.actor, UseCommand { game_session_id: deps.admitted.game_session_id, command_id: 2, item_fence: Some(fence) }, ItemKey::Instance(bag_two))
            .await
            .ok_or("refused outcome")?;
        assert_eq!(refused.disposition, UseDisposition::NotPossible);
        assert_eq!(craft_count().await?, 1);
        let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND item_instance_id::text<>ALL($2)").bind(&reward).bind(&seeded).fetch_one(&harness.pool).await?;
        assert_eq!(minted, 1);
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
