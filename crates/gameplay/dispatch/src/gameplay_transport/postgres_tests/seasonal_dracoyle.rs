//! W19E: real use/use-with routes, seeded rust and lottery outcomes, custody and replay.
use super::fixture::{Harness, debug, id, run};

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
fn srv7_santa_dracoyle_native_use_with_consumes_gem_and_schedules_seven_days() -> TestResult {
    run_scenario("oteryn:item.tibia.i9040", true, true)
}

#[test]
fn srv7_dracoyle_actual_enchantment_rotate_preserves_absolute_deadline_and_expires_once()
-> TestResult {
    run_scenario("oteryn:item.tibia.i9035", true, true)
}

fn run_scenario(key: &'static str, rust: bool, success: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(key, rust, success).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "rust lottery e2e panic")??;
    Ok(())
}
fn scenario(key: &str, rust: bool, success: bool) -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let harness =
            Harness::create_with_admission_seed(admin, "rust_lottery", true, false).await?;
        super::reset();
        prepare_current_session(&harness).await?;
        eprintln!("W19E_PG_BEGIN rust={rust} success={success}");
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
        let tool_key = "oteryn:item.tibia.i677";
        let tool_quantity = if success { 2 } else { 1 };
        let entries = if rust {
            vec![(tool_key, tool_quantity), (key, 1)]
        } else {
            vec![(key, 1)]
        };
        super::item_look::seed_main_backpack(&harness, &entries).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[
                4,
                6,
                11,
                15,
                18,
                oteryn_protocol_oteryn::world_rotate::CAPABILITY_WORLD_ROTATE_V1,
            ],
        )
        .ok_or("offers")?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::world_rotate::CAPABILITY_WORLD_ROTATE_V1)
        );

        let seeded_source = if rust { id(72) } else { id(71) };
        let command_id = 1;
        let source = seeded_source;
        let output = native
            .item_policy_for_source_id(if key == "oteryn:item.tibia.i9035" {
                9034
            } else {
                9039
            })
            .ok_or("output policy")?
            .record();
        let expected = oteryn_gameplay_items::item_craft::CookingConversion {
            recipe: "oteryn:seasonal.dracoyle".into(),
            outputs: vec![(
                crate::durability::item_mint::TypedDefinitionRef {
                    family: output.production_definition.family.clone(),
                    production_key: output.production_definition.production_key.clone(),
                    revision_ref: output.production_definition.revision_ref.clone(),
                },
                1,
            )],
        };
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(source.as_slice()).bind(seeded_source.as_slice()).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid")
            .bind(source.as_slice()).bind(seeded_source.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: rust.then_some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("target")?,
            )),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (end, result, actual) = super::rust_lottery::exchange(
            &owner,
            admitted,
            command_id,
            &payload,
            reference(if rust { tool_key } else { key }),
            if rust { reference(key) } else { None },
        )
        .await?;
        assert_eq!(
            result,
            UseDisposition::Committed,
            "rust={rust} success={success}"
        );
        let retired: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retired, (0, 2));
        let durable = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await?
            .ok_or("backpack")?;
        let mut keys: Vec<_> = durable
            .entries
            .iter()
            .map(|e| (e.item.definition.production_key.as_str(), e.item.quantity))
            .collect();
        let mut expected_keys: Vec<_> = expected
            .outputs
            .iter()
            .map(|(d, n)| (d.production_key.as_str(), *n))
            .collect();
        if rust && success {
            expected_keys.push((tool_key, 1));
        }
        keys.sort_unstable();
        expected_keys.sort_unstable();
        assert_eq!(keys, expected_keys);
        let (recipe, outputs, before, hash_matches, extra, source_count, consume_source): (String, serde_json::Value, serde_json::Value, bool, serde_json::Value, i64, bool) = sqlx::query_as("SELECT r.recipe,r.outputs,r.before_secondary,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256,r.extra_inputs,r.source_count,r.consume_source FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)").fetch_one(&harness.pool).await?;
        assert_eq!(recipe, expected.recipe);
        assert_eq!(
            outputs.as_array().ok_or("outputs")?.len(),
            usize::from(success)
        );
        assert!(hash_matches);
        let deadline:(i64,i64,String)=sqlx::query_as("SELECT duration_millis,expires_at_unix_ms-r.occurred_at,target_production_key FROM game_spell_item_temporal_schedules s JOIN game_item_consume_transform_records r ON r.transaction_id=s.source_transaction_id").fetch_one(&harness.pool).await?;
        assert_eq!(deadline, (604800000, 604800000, key.to_owned()));
        if rust {
            let proof = before;
            assert_eq!(
                proof["definition_production_key"], key,
                "the source statue is the primary consumed operand"
            );
            assert_eq!(proof["quantity"], 1);
            assert_eq!(source_count, 1);
            assert!(consume_source);
            let costs = extra.as_array().ok_or("exact emerald input receipt")?;
            assert_eq!(costs.len(), 1);
            assert_eq!(costs[0]["before"]["definition_production_key"], tool_key);
            assert_eq!(costs[0]["before"]["quantity"], tool_quantity);
            assert_eq!(costs[0]["count"], 1);
            assert_eq!(costs[0]["source_kind"], 1);
            assert!(sqlx::query_scalar::<_,bool>("SELECT source_item_instance_id=encode($1,'hex')::uuid AND (extra_inputs->0->>'item')::uuid=encode($2,'hex')::uuid FROM game_item_consume_transform_records").bind(id(72).as_slice()).bind(id(71).as_slice()).fetch_one(&harness.pool).await?);
        }
        // Receipt replay with the exact original request must not consume another flask.
        let replay = if rust {
            owner
                .use_consume_transform(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                        end.continuity.selected_capabilities.as_slice(),
                        &actual,
                    )
                    .map_err(debug)?,
                    None,
                    None,
                )
                .await
        } else {
            owner
                .use_item_completion(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    ItemKey::Instance(source),
                )
                .await
        }
        .ok_or("replay")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records"
            )
            .fetch_one(&harness.pool)
            .await?,
            1
        );
        if rust {
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?, tool_quantity - 1);
        }
        // Re-enter the real connection with the post-use inventory, then replay the wire command.
        let (end, replayed, _) =
            super::rust_lottery::exchange(&owner, end, command_id, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        if key == "oteryn:item.tibia.i9035" {
            let original: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_spell_item_temporal_schedules s")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                original["definition_production_key"],
                "oteryn:item.tibia.i9034"
            );
            assert_eq!(original["target_production_key"], "oteryn:item.tibia.i9035");
            let observed =
                reference("oteryn:item.tibia.i9034").ok_or("charged source reference")?;
            let rotate_payload = encode_item_use_intent(&UseIntent {
                target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("rotate handle")?),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            })
            .map_err(debug)?;
            let (rotated_session, rotated, _) = super::rust_lottery::exchange_rotate(
                &owner,
                end,
                2,
                &rotate_payload,
                Some(observed),
                observed,
            )
            .await?;
            assert_eq!(rotated, UseDisposition::Committed);
            let after: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_spell_item_temporal_schedules s")
                    .fetch_one(&harness.pool)
                    .await?;
            for field in [
                "item_instance_id",
                "source_transaction_id",
                "source_ordinal",
                "duration_millis",
                "expires_at_unix_ms",
                "content_digest",
            ] {
                assert_eq!(
                    after[field], original[field],
                    "Rotate must preserve {field}"
                );
            }
            assert_eq!(
                after["definition_production_key"],
                "oteryn:item.tibia.i9039"
            );
            assert_eq!(after["target_production_key"], "oteryn:item.tibia.i9040");
            let item = after["item_instance_id"].as_str().ok_or("charged UUID")?;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=$1::uuid"
                )
                .bind(item)
                .fetch_one(&harness.pool)
                .await?,
                0,
                "no parallel clock"
            );
            // Preservation above uses the actual produced future seven-day schedule.
            // This explicit controlled predecessor advances only the fixture clock to
            // overdue; no claim is made that seven real days elapsed in the test.
            let mut clock = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *clock)
                .await?;
            sqlx::query("UPDATE game_spell_item_temporal_schedules SET expires_at_unix_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1 WHERE item_instance_id=$1::uuid").bind(item).execute(&mut *clock).await?;
            clock.commit().await?;
            let permit = owner.spell_lane_permit().await.ok_or("expiry lane")?;
            assert_eq!(
                owner
                    .root
                    .drain_spell_item_deadlines(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.runtime_scope,
                        fence.scope_ownership_generation.get()
                    )
                    .await
                    .map_err(debug)?,
                1
            );
            assert_eq!(
                owner
                    .root
                    .drain_spell_item_deadlines(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.runtime_scope,
                        fence.scope_ownership_generation.get()
                    )
                    .await
                    .map_err(debug)?,
                0
            );
            let actual:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=$1::uuid").bind(item).fetch_one(&harness.pool).await?;
            assert_eq!(actual, "oteryn:item.tibia.i9040");
            drop(permit);
            let settled: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_spell_item_temporal_schedules s")
                    .fetch_one(&harness.pool)
                    .await?;
            // Advancing the controlled DB expiry predecessor does not advance
            // the native actor's independent monotonic action clock. Honor its
            // actual remaining delay before the next fresh Rotate.
            let remaining = owner
                .states
                .world_gate
                .lock()
                .map_err(|_| "world action gate")?
                .equip_delay(deps.actor, 0, owner.clock_origin.elapsed());
            if let Some(remaining) = remaining {
                tokio::time::sleep(remaining).await;
            }
            let inactive = reference("oteryn:item.tibia.i9040").ok_or("inactive reference")?;
            let (_, result, _) = super::rust_lottery::exchange_rotate(
                &owner,
                rotated_session,
                3,
                &rotate_payload,
                Some(inactive),
                inactive,
            )
            .await?;
            assert_eq!(result, UseDisposition::Committed);
            let actual:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=$1::uuid").bind(item).fetch_one(&harness.pool).await?;
            assert_eq!(actual, "oteryn:item.tibia.i9035");
            let historical: serde_json::Value =
                sqlx::query_scalar("SELECT to_jsonb(s) FROM game_spell_item_temporal_schedules s")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                historical, settled,
                "inactive Rotate must not restart historical absolute clock"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=$1::uuid"
                )
                .bind(item)
                .fetch_one(&harness.pool)
                .await?,
                0
            );
        }
        let elapsed = started.elapsed();
        assert!(elapsed > std::time::Duration::ZERO);
        eprintln!(
            "W19E_PG_EXECUTED rust={rust} success={success} command={command_id} elapsed={elapsed:?}"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Uses the real native wire before House scope admission, returning the actual
/// newly minted charged UUID. Caller seeds statue/gem through its existing fixture.
pub(super) async fn animate_fixture_source(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    statue: [u8; 16],
    gem: [u8; 16],
) -> TestResult<(
    crate::gameplay_transport::connection::AdmittedSession,
    [u8; 16],
)> {
    let fence = admitted.item_fence.ok_or("actual enchant Item fence")?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native enchant content")?;
    let index = native.item_index().ok_or("native Item index")?;
    let before = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?
        .ok_or("source backpack")?;
    assert!(
        before
            .entries
            .iter()
            .any(|entry| entry.item.item_instance_id == statue
                && entry.item.definition.production_key == "oteryn:item.tibia.i9035")
    );
    assert!(
        before
            .entries
            .iter()
            .any(|entry| entry.item.item_instance_id == gem
                && entry.item.definition.production_key == "oteryn:item.tibia.i677")
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            std::num::NonZeroU64::new(2).ok_or("target handle")?,
        )),
        use_with_position: None,
    })
    .map_err(debug)?;
    let source = index
        .definition_ref("Item", "oteryn:item.tibia.i677", "definition-r1")
        .ok_or("gem reference")?;
    let target = index
        .definition_ref("Item", "oteryn:item.tibia.i9035", "definition-r1")
        .ok_or("inactive reference")?;
    let (next, result, _) =
        super::rust_lottery::exchange(owner, admitted, 1, &payload, Some(source), Some(target))
            .await?;
    assert_eq!(result, UseDisposition::Committed);
    let after = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?
        .ok_or("charged backpack")?;
    assert!(
        !after
            .entries
            .iter()
            .any(|entry| entry.item.item_instance_id == statue)
    );
    let charged = after
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i9034")
        .ok_or("actual charged output")?
        .item
        .item_instance_id;
    assert_ne!(charged, statue);
    Ok((next, charged))
}
