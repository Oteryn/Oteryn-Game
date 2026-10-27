//! SOULPIT: admitted USE -> existing craft owner -> fenced atomic two-core fusion.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

#[test]
fn soulpit_fusion_entire_admitted_catalogue_and_special_item_refusals() -> TestResult {
    let deps = Dependencies::new()?;
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        deps.admitted.world_id,
        None,
    );
    let native = controller
        .active()
        .ok_or("active")?
        .native_gameplay()
        .ok_or("native")?;
    assert!(oteryn_gameplay_items::soul_core::soul_core_fusion_available(native));
    let without_cores =
        crate::content::native_gameplay::test_support::activated_with_item_keys(None);
    let incomplete = without_cores.active().ok_or("active without cores")?;
    assert!(
        !oteryn_gameplay_items::soul_core::soul_core_fusion_available(
            incomplete.native_gameplay().ok_or("native without cores")?
        )
    );
    for active in [None, Some(incomplete)] {
        for weather_bound in [false, true] {
            let offered =
                crate::gameplay_transport::world_required_capabilities::offered_for_scope_with(
                    true,
                    active,
                    weather_bound,
                );
            assert!(!offered.iter().any(|capability| capability.id == 114));
            let selected = SelectedCapabilities::select(offered, &[4, 6, 11, 15, 18, 114])
                .ok_or("selection without core profiles")?;
            assert!(!selected.contains(114));
        }
    }
    let item = |id| crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    let ids: Vec<u32> =
        serde_json::from_str(include_str!("../../../../items/src/soul_core_ids.json"))?;
    let mut reached = std::collections::BTreeSet::new();
    for (draw, id) in ids.iter().enumerate() {
        let plan = oteryn_gameplay_items::soul_core::soul_core_fusion(
            native,
            &item(47384),
            &item(47398),
            u64::try_from(draw)?,
        )
        .ok_or("core output closure")?;
        assert_eq!(plan.outputs, [(item(*id), 1)]);
        reached.insert(*id);
    }
    assert_eq!(reached.len(), 680);
    for special in [2854, 37110, 47899, 49163, 49164] {
        assert!(
            oteryn_gameplay_items::soul_core::soul_core_fusion(
                native,
                &item(special),
                &item(47398),
                0
            )
            .is_none()
        );
        assert!(
            oteryn_gameplay_items::soul_core::soul_core_fusion(
                native,
                &item(47384),
                &item(special),
                0
            )
            .is_none()
        );
    }
    let mut wrong_family = item(47384);
    wrong_family.family = "Creature".into();
    assert!(
        oteryn_gameplay_items::soul_core::soul_core_fusion(native, &wrong_family, &item(47398), 0)
            .is_none()
    );
    Ok(())
}

#[test]
fn soulpit_fusion_wire_atomic_replay_restart_and_refusals() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "soulpit e2e panic")??;
    Ok(())
}

#[test]
fn soulpit_transformations_wire_difficulties_ominous_replay_and_restart() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "soulpit transformations e2e panic")??;
    Ok(())
}

#[path = "soulpit/custody.rs"]
mod custody_cases;

#[test]
fn soulpit_fusion_custody_wire_nested_equipment_ground_and_recovery() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "soulpit custody panic")??;
    Ok(())
}

fn scenario(transformations: bool, custody: bool) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "soulpit", true, false).await?;
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
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
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
        if custody {
            custody_cases::run(&harness, &deps, &authority, &mut owner, fence, native).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if transformations {
            transformation_cases(&harness, &deps, &authority, &mut owner, fence, native).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let orc = "oteryn:item.tibia.i47384";
        let rat = "oteryn:item.tibia.i47398";
        super::item_look::seed_main_backpack(&harness, &[(orc, 2), (rat, 1)]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 114])
                .ok_or("soulpit offers")?;
        assert!(admitted.continuity.selected_capabilities.contains(114));
        // Newest-first initial inventory handles: bag=1, rat=2, orc=3.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("target")?,
            )),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let receipt_count = async || {
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await
        };
        // Legacy sessions cannot start fusion and keep both inputs.
        let mut legacy = admitted;
        legacy.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("legacy")?;
        let (_, refused, _) = exchange(&owner, legacy, 1, &payload, None)
            .await
            .map_err(|e| format!("legacy fusion: {e}"))?;
        assert_ne!(refused, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 0);
        // A fresh owner forgets the legacy connection's ingress result. Reusing its
        // CommandId on the same owner would replay that refusal before routing USE.
        drop(owner);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let (end, result, actual) = exchange(&owner, admitted, 1, &payload, None)
            .await
            .map_err(|e| format!("first fusion: {e}"))?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 1);
        let inputs: Vec<(i64, i16)> = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id=encode($2,'hex')::uuid ORDER BY item_instance_id")
            .bind(id(71).as_slice()).bind(id(72).as_slice()).fetch_all(&harness.pool).await?;
        assert_eq!(inputs, [(1, 1), (0, 2)]);
        let (recipe, before, secondary, outputs, hash_matches): (String, serde_json::Value, serde_json::Value, serde_json::Value, bool) = sqlx::query_as(
            "SELECT r.recipe,r.before_item,r.secondary_before,r.outputs,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_craft_records r JOIN game_item_audit_outbox a USING(event_id)",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(recipe, "oteryn:soulpit.fuse");
        assert_eq!(before["quantity"], 2);
        assert_eq!(secondary["quantity"], 1);
        assert_eq!(outputs.as_array().ok_or("outputs")?.len(), 1);
        assert_eq!(outputs[0]["count"], 1);
        assert!(hash_matches);
        let durable = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack")?;
        assert_eq!(durable.entries.len(), 2);
        assert!(
            durable
                .entries
                .iter()
                .any(|e| e.item.definition.production_key
                    == outputs[0]["key"].as_str().unwrap_or(""))
        );
        // A restarted owner returns the original receipt before resolving consumed handles.
        drop(owner);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let (replayed, replay, _) = exchange(&owner, admitted, 1, &actual, None)
            .await
            .map_err(|e| format!("replay: {e}"))?;
        assert_eq!(replay, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 1);
        // Reusing the command for different bytes never redraws or mutates either item.
        let different = encode_item_use_intent(&UseIntent {
            use_with_map_item: Some(MapUseTarget::Item(
                std::num::NonZeroU64::new(1).ok_or("bag")?,
            )),
            ..intent.clone()
        })
        .map_err(debug)?;
        // The session ingress intentionally replays its terminal verdict regardless
        // of replacement bytes; exercise the durable owner's independent conflict check.
        let conflict = owner
            .use_craft(
                deps.actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 1,
                    item_fence: Some(fence),
                },
                oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                    admitted.continuity.selected_capabilities.as_slice(),
                    &different,
                )
                .map_err(debug)?,
                None,
                None,
            )
            .await
            .ok_or("conflict result")?
            .disposition;
        assert_eq!(conflict, UseDisposition::Rejected);
        assert_eq!(receipt_count().await?, 1);
        // A lone remaining core cannot be fused with itself.
        assert_eq!(end.continuity.next_command_id, 2);
        let next = replayed;
        let (next, lone) = exchange_pair(
            &owner,
            next,
            reference(orc).ok_or("orc")?,
            reference(orc).ok_or("orc")?,
        )
        .await?;
        assert_eq!(lone, UseDisposition::NotPossible);
        assert_eq!(receipt_count().await?, 1);
        // Two units in one stack retire together and leave one fresh output.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET quantity=2 WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (next, fused) = exchange_pair(
            &owner,
            next,
            reference(orc).ok_or("orc")?,
            reference(orc).ok_or("orc")?,
        )
        .await?;
        assert_eq!(fused, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 2);
        let (quantity, lifecycle): (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!((quantity, lifecycle), (0, 2));
        let units: i16 = sqlx::query_scalar(
            "SELECT source_units FROM game_item_craft_records WHERE command_id=3",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(units, 2);
        // A plain item and a core cannot fuse. Both are still retained after refusal.
        let (_, rejected) = exchange_pair(
            &owner,
            next,
            reference(bag).ok_or("bag")?,
            reference(outputs[0]["key"].as_str().ok_or("output key")?)
                .ok_or("output definition")?,
        )
        .await?;
        assert_ne!(rejected, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 2);
        // Full bag: partial stacks free no slot, while consuming two last units frees two.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        for (seed, key, ordinal) in [(91, orc, 100), (92, rat, 101)] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',2,1,game_character_uuid_v7())")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7())")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(ordinal).execute(&mut *tx).await?;
        }
        for n in 0..16 {
            sqlx::query("WITH i AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3600','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,game_character_uuid_v7() FROM i")
                .bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(102_i64+n).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let full_command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 5,
            item_fence: Some(fence),
        };
        let result = owner
            .use_craft(
                deps.actor,
                full_command,
                intent.clone(),
                Some(ItemKey::Instance(id(91))),
                Some(ItemKey::Instance(id(92))),
            )
            .await
            .ok_or("full result")?;
        assert_eq!(result.disposition, UseDisposition::NoRoom);
        assert_eq!(receipt_count().await?, 2);
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET quantity=1 WHERE item_instance_id=encode($1,'hex')::uuid OR item_instance_id=encode($2,'hex')::uuid")
            .bind(id(91).as_slice()).bind(id(92).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let result = owner
            .use_craft(
                deps.actor,
                UseCommand {
                    command_id: 6,
                    ..full_command
                },
                intent.clone(),
                Some(ItemKey::Instance(id(91))),
                Some(ItemKey::Instance(id(92))),
            )
            .await
            .ok_or("last units result")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        assert_eq!(receipt_count().await?, 3);
        let final_bag = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("final backpack")?;
        assert_eq!(final_bag.entries.len(), 19);
        let core = final_bag
            .entries
            .iter()
            .find(|entry| entry.item.definition.production_key != "oteryn:item.tibia.i3600")
            .ok_or("remaining core")?
            .item
            .item_instance_id;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET quantity=2 WHERE item_instance_id=encode($1,'hex')::uuid").bind(core.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        // An output trigger proves execution reached mint after both planned decrements.
        // Its nontransactional sequence records the attempt even though the craft rolls back.
        sqlx::raw_sql("CREATE SEQUENCE soulpit_mint_attempt; CREATE FUNCTION soulpit_reject_output() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('soulpit_mint_attempt'); RAISE EXCEPTION 'soulpit injected output failure'; END $$; CREATE TRIGGER soulpit_reject_output BEFORE INSERT ON game_item_instances FOR EACH ROW EXECUTE FUNCTION soulpit_reject_output()")
            .execute(&harness.pool).await?;
        let failed = owner
            .use_craft(
                deps.actor,
                UseCommand {
                    command_id: 7,
                    ..full_command
                },
                intent.clone(),
                Some(ItemKey::Instance(core)),
                Some(ItemKey::Instance(core)),
            )
            .await;
        assert!(failed.is_none());
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT is_called FROM soulpit_mint_attempt")
                .fetch_one(&harness.pool)
                .await?
        );
        assert_eq!(receipt_count().await?, 3);
        let retained: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(core.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retained, (2, 1));
        sqlx::raw_sql("DROP TRIGGER soulpit_reject_output ON game_item_instances; DROP FUNCTION soulpit_reject_output(); DROP SEQUENCE soulpit_mint_attempt").execute(&harness.pool).await?;
        // Change only the current lease, keeping valid live core inputs and all other facts.
        let refused = owner
            .use_craft(
                deps.actor,
                UseCommand {
                    command_id: 8,
                    item_fence: Some(
                        crate::durability::item_transfer::CurrentCharacterItemFence {
                            character_lease_generation: fence.character_lease_generation + 1,
                            ..fence
                        },
                    ),
                    ..full_command
                },
                intent,
                Some(ItemKey::Instance(core)),
                Some(ItemKey::Instance(core)),
            )
            .await;
        assert!(
            refused.is_none(),
            "stale lease closes the item owner before PREPARE"
        );
        assert_eq!(receipt_count().await?, 3);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// Resolve both handles from this connection's real inventory snapshot. Handles are
// session observations and are deliberately not stable across reconnect snapshots.
async fn exchange_pair(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    source_definition: std::num::NonZeroU32,
    target_definition: std::num::NonZeroU32,
) -> TestResult<(
    crate::gameplay_transport::connection::AdmittedSession,
    UseDisposition,
)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::AsyncWriteExt;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exchanging = async move {
        let mut source = None;
        let mut target = None;
        let mut bags = Vec::new();
        let bag_definition = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.item_index())
            .and_then(|index| {
                index.definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
            });
        let mut record = |entry: oteryn_protocol_oteryn::item_view::ItemEntry| {
            if entry.item_definition_ref == source_definition {
                source = Some(entry.handle);
            }
            if entry.item_definition_ref == target_definition {
                target = Some(entry.handle);
            }
            if Some(entry.item_definition_ref) == bag_definition {
                bags.push(entry.handle);
            }
        };
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                    {
                        let inventory = oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload, admitted.continuity.selected_capabilities.contains(12)).map_err(debug)?;
                        for entry in inventory
                            .main_backpack
                            .into_iter()
                            .chain(inventory.entries)
                            .chain(inventory.equipment.into_iter().map(|entry| entry.item))
                        {
                            record(entry);
                        }
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        for tile in map.tiles {
                            for item in tile.items {
                                if let (
                                    oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                        reference,
                                    ),
                                    oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle),
                                ) = (item.definition, item.origin)
                                {
                                    record(oteryn_protocol_oteryn::item_view::ItemEntry {
                                        handle,
                                        item_definition_ref: reference,
                                        count: std::num::NonZeroU32::new(item.count)
                                            .ok_or("count")?,
                                        sub_type: item.sub_type,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        // Open carried bags through command 21, then use only handles delivered in domain 14.
        let mut opened = std::collections::BTreeSet::new();
        let mut next_command = admitted.continuity.next_command_id;
        while (source.is_none() || target.is_none()) && !bags.is_empty() {
            let bag = bags.remove(0);
            if !opened.insert(bag) {
                continue;
            }
            let payload = oteryn_protocol_oteryn::container_tree::encode_container_view_intent(
                &oteryn_protocol_oteryn::container_tree::ContainerViewIntent::Open {
                    handle: bag,
                    replace_view: None,
                },
            )
            .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: next_command,
                    command_type: 21,
                    payload: &payload,
                },
            )?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            next_command += 1;
            loop {
                let raw = super::item_look::read_frame(&mut client).await?;
                let envelope = crate::foundation::decode_wire_envelope(&raw)?;
                if envelope.message_type() == crate::foundation::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                    if delta.domain_id
                        == oteryn_protocol_oteryn::container_tree::STATE_DOMAIN_CONTAINER_VIEWS
                    {
                        let views = oteryn_protocol_oteryn::container_tree::decode_container_views(
                            delta.payload,
                        )
                        .map_err(debug)?;
                        for entry in views.views.into_iter().flat_map(|view| view.entries) {
                            if entry.item_definition_ref == source_definition {
                                source = Some(entry.handle);
                            }
                            if entry.item_definition_ref == target_definition {
                                target = Some(entry.handle);
                            }
                            if Some(entry.item_definition_ref) == bag_definition
                                && !opened.contains(&entry.handle)
                            {
                                bags.push(entry.handle);
                            }
                        }
                        break;
                    }
                }
            }
        }
        let command_id = next_command;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(source.ok_or("live source core handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(target.ok_or("live target core handle")?)),
            use_with_position: None,
        })
        .map_err(debug)?;
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &payload,
            },
        )?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        client.shutdown().await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&raw)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if result.command_id == command_id {
                    return oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                        .map_err(debug);
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(serving);
    let mut exchanging = std::pin::pin!(exchanging);
    let mut end = None;
    let mut result: Option<TestResult<UseDisposition>> = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = exchanging.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("server did not finish")?;
    let result = result
        .ok_or("client did not finish")?
        .map_err(|error| format!("{error}; connection ended: {end:?}"))?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    Ok((admitted, result))
}

async fn transformation_cases(
    harness: &Harness,
    deps: &Dependencies,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult {
    let species: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../items/src/soul_core_metadata.json"
    ))?;
    let reference = |key: &str| {
        native
            .item_index()
            .and_then(|i| i.definition_ref("Item", key, "definition-r1"))
    };
    super::item_look::seed_main_backpack(harness, &[]).await?;
    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 114])
            .ok_or("transformation capabilities")?;
    sqlx::raw_sql(
        "CREATE TABLE soulpit_test_entropy (draw bytea NOT NULL); \
         INSERT INTO soulpit_test_entropy VALUES (decode('0000000000000000ffffffffffffffff', 'hex')); \
         GRANT SELECT ON soulpit_test_entropy TO oteryn_game_runtime; \
         CREATE OR REPLACE FUNCTION game_soulpit_entropy() RETURNS bytea LANGUAGE SQL VOLATILE \
         AS $$ SELECT draw FROM soulpit_test_entropy $$",
    )
    .execute(&harness.pool)
    .await?;
    // Fixed server entropy is a database fixture, never a client-controlled draw.
    let mut cases = vec![
        (49164, "easy", false, "medium", None),
        (49164, "challenging", false, "challenging", None),
        (37110, "medium", false, "easy", None),
        (37110, "harmless", false, "harmless", None),
        (49164, "easy", true, "easy", None),
        (49164, "easy", false, "medium", Some(47658_u64)),
        (49164, "medium", false, "hard", Some(47660)),
        (49164, "medium", false, "hard", Some(47662)),
    ]
    .into_iter()
    .map(|(tool, difficulty, ominous, desired, core)| {
        (tool, difficulty, ominous, desired, core, 0_u64, None)
    })
    .collect::<Vec<_>>();
    let tiers = [
        "harmless",
        "trivial",
        "easy",
        "medium",
        "hard",
        "challenging",
    ];
    // Every newly qualified Item is exercised as both tools' input and an exact
    // output through serve_admitted. No synthetic Creature identity is registered.
    for core in [
        47654_u64, 47691, 47697, 47773, 48089, 49007, 49033, 49034, 49035, 49036, 49037, 49040,
        49041,
    ] {
        let own = species
            .iter()
            .find(|s| s["core"].as_u64() == Some(core))
            .ok_or("qualified core metadata")?;
        let difficulty = own["difficulty"].as_str().ok_or("difficulty")?;
        let tier = tiers
            .iter()
            .position(|name| *name == difficulty)
            .ok_or("tier")?;
        cases.push((
            49164,
            difficulty,
            false,
            tiers[(tier + 1).min(5)],
            Some(core),
            0,
            None,
        ));
        cases.push((
            37110,
            difficulty,
            false,
            tiers[tier.saturating_sub(1)],
            Some(core),
            0,
            None,
        ));
        let candidates = species
            .iter()
            .filter(|s| s["difficulty"] == difficulty)
            .collect::<Vec<_>>();
        let position = candidates
            .iter()
            .position(|s| s["core"].as_u64() == Some(core))
            .ok_or("candidate")?;
        let draw = u64::try_from(((position as u128 * 2 + 1) << 63) / candidates.len() as u128)?;
        let (tool, input) = if tier == 0 {
            (37110, "trivial")
        } else {
            (49164, tiers[tier - 1])
        };
        cases.push((tool, input, false, difficulty, None, draw, Some(core)));
    }
    for (index, (tool, difficulty, ominous, desired, exact_core, draw, exact_output)) in
        cases.into_iter().enumerate()
    {
        let command = u64::try_from(index)? + 1;
        let own = species
            .iter()
            .find(|s| {
                s["difficulty"] == difficulty
                    && exact_core.is_none_or(|core| s["core"].as_u64() == Some(core))
            })
            .ok_or("test difficulty")?;
        let core = own["core"].as_u64().ok_or("core id")?;
        let tool_key = format!("oteryn:item.tibia.i{tool}");
        let core_key = format!("oteryn:item.tibia.i{core}");
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        // Each command starts with the same two physical instances and a clean main bag.
        sqlx::query("DELETE FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).execute(&mut *tx).await?;
        for (seed, key, ordinal) in [(71, &tool_key, 1_i64), (72, &core_key, 2)] {
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,game_character_uuid_v7()) ON CONFLICT(item_instance_id) DO UPDATE SET definition_production_key=excluded.definition_production_key,quantity=1,lifecycle=1,last_transaction_id=NULL")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7())")
                .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(ordinal).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let ominous_draw = if ominous { 0 } else { u64::MAX };
        sqlx::query("UPDATE soulpit_test_entropy SET draw=$1")
            .bind([draw.to_be_bytes(), ominous_draw.to_be_bytes()].concat())
            .execute(&harness.pool)
            .await?;
        let (next, result) = exchange_pair(
            owner,
            admitted,
            reference(&tool_key).ok_or("tool ref")?,
            reference(&core_key).ok_or("core ref")?,
        )
        .await?;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "{tool} {difficulty} ominous={ominous}"
        );
        admitted = next;
        let (recipe, outputs): (String, serde_json::Value) = sqlx::query_as(
            "SELECT recipe,outputs FROM game_item_craft_records WHERE command_id=$1::text::numeric",
        )
        .bind(command.to_string())
        .fetch_one(&harness.pool)
        .await?;
        let expected_recipe = if ominous {
            "oteryn:soulpit.prism-ominous"
        } else if tool == 49164 {
            "oteryn:soulpit.prism"
        } else {
            "oteryn:soulpit.exalted"
        };
        assert_eq!(recipe, expected_recipe);
        if ominous {
            assert_eq!(outputs[0]["key"], "oteryn:item.tibia.i49163");
            assert_eq!(outputs[1]["key"], core_key);
        } else {
            let output = outputs[0]["key"].as_str().ok_or("output key")?;
            if let Some(core) = exact_output {
                assert_eq!(output, format!("oteryn:item.tibia.i{core}"));
            }
            assert!(species.iter().any(|s| s["difficulty"] == desired
                && output == format!("oteryn:item.tibia.i{}", s["core"])));
        }
        let backpack = harness
            .root
            .read_character_backpack(authority, fence.character_id)
            .await?
            .ok_or("transformed backpack")?;
        assert_eq!(backpack.entries.len(), if ominous { 2 } else { 1 });
        let retired: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid) AND quantity=0 AND lifecycle=2")
            .bind(id(71).as_slice()).bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, 2);
        // Durable replay uses the exact original bounded intent even after both handles retire.
        let intent: Vec<u8> = sqlx::query_scalar(
            "SELECT intent FROM game_item_craft_records WHERE command_id=$1::text::numeric",
        )
        .bind(command.to_string())
        .fetch_one(&harness.pool)
        .await?;
        let mut restarted = deps.owner(harness, authority);
        restarted.active_generation = owner.active_generation;
        restarted.movement_cells = owner.movement_cells;
        deps.initialize(&restarted).await;
        let replay = restarted
            .use_craft(
                deps.actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command,
                    item_fence: Some(fence),
                },
                oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                    admitted.continuity.selected_capabilities.as_slice(),
                    &intent,
                )
                .map_err(debug)?,
                None,
                None,
            )
            .await
            .ok_or("transform replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        let retained: serde_json::Value = sqlx::query_scalar(
            "SELECT outputs FROM game_item_craft_records WHERE command_id=$1::text::numeric",
        )
        .bind(command.to_string())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(retained, outputs);
    }
    Ok(())
}

#[test]
fn soulpit_reference_drop_probability_and_transformation_boundaries() -> TestResult {
    let deps = Dependencies::new()?;
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        deps.admitted.world_id,
        None,
    );
    let native = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .ok_or("native")?;
    let item = |id| crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    };
    // Midpoints of each of the 100 equal integer-percent buckets avoid edge rounding.
    let bucket = |n: u64| u64::try_from((u128::from(n) * 2 + 1) * (1_u128 << 63) / 100);
    let mut cores = 0;
    let mut prisms = 0;
    let mut ominous = 0;
    for n in 0..100 {
        let draw = bucket(n)?;
        let drops = oteryn_gameplay_items::soul_core::soulpit_acquisition(
            native,
            "oteryn:creature.rat",
            "definition-r1",
            [draw, 0, 0, draw],
        )
        .ok_or("acquisition")?;
        cores += usize::from(drops.contains(&item(47398)));
        prisms += usize::from(drops.contains(&item(49164)));
        let plan = oteryn_gameplay_items::soul_core::soulpit_transformation(
            native,
            &item(49164),
            &item(47398),
            0,
            draw,
        )
        .ok_or("prism")?;
        ominous += usize::from(plan.recipe == "oteryn:soulpit.prism-ominous");
    }
    assert_eq!((cores, prisms, ominous), (4, 3, 2));
    let same = oteryn_gameplay_items::soul_core::soulpit_acquisition(
        native,
        "oteryn:creature.rat",
        "definition-r1",
        [0, bucket(14)?, 0, u64::MAX],
    )
    .ok_or("same")?;
    assert_eq!(same, [item(47398)]);
    let alternate = oteryn_gameplay_items::soul_core::soulpit_acquisition(
        native,
        "oteryn:creature.rat",
        "definition-r1",
        [0, bucket(15)?, 0, u64::MAX],
    )
    .ok_or("class")?;
    let species: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../items/src/soulpit_species.json"))?;
    assert_eq!(alternate.len(), 1);
    assert!(species.iter().any(|s| s["class"] == "Mammal"
        && alternate[0].production_key == format!("oteryn:item.tibia.i{}", s["core"])));
    assert!(
        oteryn_gameplay_items::soul_core::soulpit_acquisition(
            native,
            "oteryn:creature.rat",
            "stale",
            [0; 4]
        )
        .is_none()
    );
    assert!(
        oteryn_gameplay_items::soul_core::soulpit_transformation(
            native,
            &item(49163),
            &item(47398),
            0,
            0
        )
        .is_none()
    );
    assert!(
        oteryn_gameplay_items::soul_core::soulpit_transformation(
            native,
            &item(49164),
            &item(49163),
            0,
            0
        )
        .is_none()
    );
    Ok(())
}
