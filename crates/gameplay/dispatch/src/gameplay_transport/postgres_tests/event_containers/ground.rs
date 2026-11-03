//! Ground reward receipts use actual map handles and the shared consume-transform owner.
use super::*;
use oteryn_gameplay_items::novelty_items::{GroundEffect, NoveltyPlan};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Anniversary,
    Special,
}
impl Case {
    fn key(self) -> &'static str {
        match self {
            Self::Anniversary => "oteryn:item.tibia.i39710",
            Self::Special => "oteryn:item.tibia.i37457",
        }
    }
}

#[test]
fn w19_ground_event_container_anniversary_atomic_inventory_bundle_and_replay() -> TestResult {
    run_case(Case::Anniversary)
}

#[test]
fn w19_ground_event_container_special_exact_cell_limit_and_replay() -> TestResult {
    run_case(Case::Special)
}

fn run_case(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("W19 POSTGRES_CASE_COMPLETED status=SKIPPED no_admin_url ground_reward");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Ground reward PG requires PostgreSQL"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Ground reward scenario thread")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let key = case.key();
        let harness =
            Harness::create_with_admission_seed(admin, "w19_ground_reward", true, false).await?;
        super::super::reset();
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
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let bundle_room = super::super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle_room.frame_binding().digest(),
            bundle_room.map_revision_digest(),
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
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
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
        super::super::item_look::seed_main_backpack(&harness, &[(key, 2)]).await?;
        let source = id(71);
        let used = crate::gameplay_transport::item_look::typed_definition(
            index,
            reference(key).ok_or("reward source reference")?,
        )
        .ok_or("reward source definition")?;
        let target = {
            let runtime = owner.runtime.lock().await;
            crate::durability::item_loop_move::LoopGround::from_native_owner(
                &bundle_room,
                &runtime,
                MovementLocalPosition {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
                &crate::map::overlay::map_revision(&base),
            )
            .map_err(debug)?
        };
        // Historical fixture data: real qualified Ground placement and distinct UUIDv7 roots.
        // The source gets the last sequence ordinal so it is a visible top-level map item.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let mut fillers = Vec::new();
        let source_ref = [
            crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
            id(79).as_slice(),
        ]
        .concat();
        for ordinal in 0..if case == Case::Special { 63 } else { 7 } {
            let item: String = sqlx::query_scalar(
                "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3588','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text",
            ).bind(id(42).as_slice()).fetch_one(&mut *tx).await?;
            if case == Case::Special {
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,nextval('game_item_ground_stack_ordinal_seq'))")
                    .bind(&item).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(&source_ref)
                    .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,game_character_uuid_v7())")
                    .bind(&item).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(i64::from(100 + ordinal)).execute(&mut *tx).await?;
            }
            fillers.push(item);
        }
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(source.as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,nextval('game_item_ground_stack_ordinal_seq'))")
            .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(&source_ref)
            .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).execute(&mut *tx).await?;
        tx.commit().await?;
        let source_before = source_row(&harness).await?;
        let source_location: Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        if case == Case::Special {
            assert_eq!(ground_count(&harness).await?, 64);
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 12, 15, 18])
                .ok_or("Ground reward capabilities")?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let before_refusal = durable_state(&harness).await?;
        let (next, refused, _) = super::super::rust_lottery::exchange(
            &owner,
            admitted,
            1,
            &payload,
            reference(key),
            None,
        )
        .await?;
        assert_eq!(
            refused,
            UseDisposition::NoRoom,
            "the actual inventory/Ground destination cannot admit an extra reward"
        );
        assert_eq!(
            durable_state(&harness).await?,
            before_refusal,
            "refusal changes no source, custody, reward, receipt or audit row"
        );
        admitted = next;

        // Free exactly one real Ground root for Special. Anniversary needs all14
        // inventory slots, so remove its seven historical slot fillers.
        let removed = if case == Case::Special {
            &fillers[..1]
        } else {
            fillers.as_slice()
        };
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id::text=ANY($1)")
            .bind(removed)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id::text=ANY($1)")
            .bind(removed)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_instances WHERE item_instance_id::text=ANY($1)")
            .bind(removed)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        if case == Case::Special {
            assert_eq!(ground_count(&harness).await?, 63);
        }
        let expected = oteryn_gameplay_items::event_items::plan(
            native,
            &used,
            *fence.game_session_id.as_bytes(),
            2,
            source,
            Some(GroundEffect {
                x: 6,
                y: 0,
                floor: 7,
            }),
        )
        .ok_or("admitted Ground reward plan")?;
        assert_eq!(
            expected.outputs.len(),
            if case == Case::Anniversary { 14 } else { 1 }
        );
        let (next, outcome, actual) = super::super::rust_lottery::exchange(
            &owner,
            admitted,
            2,
            &payload,
            reference(key),
            None,
        )
        .await?;
        assert_eq!(outcome, UseDisposition::Committed);
        admitted = next;
        verify_receipt(
            &harness,
            case,
            2,
            &expected,
            &actual,
            &source_before,
            &source_location,
        )
        .await?;
        let first_after = source_row(&harness).await?;
        assert_eq!(
            first_after["quantity"],
            if case == Case::Anniversary { 0 } else { 1 }
        );
        assert_eq!(
            first_after["lifecycle"],
            if case == Case::Anniversary { 2 } else { 1 }
        );
        assert_eq!(
            first_after["minted_transaction_id"],
            source_before["minted_transaction_id"]
        );
        assert_eq!(
            first_after["definition_production_key"],
            source_before["definition_production_key"]
        );
        // The transport retains only its latest terminal result. Replay this
        // command before accepting the next; durable replay of both receipts is
        // checked again below after the owner restarts.
        let first_settled = durable_state(&harness).await?;
        let (next, replayed, same) =
            super::super::rust_lottery::exchange(&owner, admitted, 2, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::Committed);
        assert_eq!(same, actual);
        admitted = next;
        assert_eq!(durable_state(&harness).await?, first_settled);
        let mut replays = vec![(2, actual)];
        if case == Case::Anniversary {
            assert_eq!(ground_count(&harness).await?, 0);
            let children: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_nested_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i39693') + (SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key='oteryn:item.tibia.i39693')")
                .fetch_one(&harness.pool).await?;
            assert_eq!(children, 0, "birthday backpack39693 is minted empty");
        } else {
            assert_eq!(ground_count(&harness).await?, 64);
            let retained: Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(source.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                retained, source_location,
                "partial debit retains exact Ground custody"
            );
            let expected = oteryn_gameplay_items::event_items::plan(
                native,
                &used,
                *fence.game_session_id.as_bytes(),
                3,
                source,
                Some(GroundEffect {
                    x: 6,
                    y: 0,
                    floor: 7,
                }),
            )
            .ok_or("second Special plan")?;
            // At64 roots qty1 must retire its actual source before reserving the
            // one replacement root. A blanket full-cell rejection is incorrect.
            let (next, outcome, actual) = super::super::rust_lottery::exchange(
                &owner,
                admitted,
                3,
                &payload,
                reference(key),
                None,
            )
            .await?;
            assert_eq!(
                outcome,
                UseDisposition::Committed,
                "retiring source frees its one actual root"
            );
            admitted = next;
            verify_receipt(
                &harness,
                case,
                3,
                &expected,
                &actual,
                &first_after,
                &source_location,
            )
            .await?;
            assert_eq!(ground_count(&harness).await?, 64);
            let source = source_row(&harness).await?;
            assert_eq!(source["quantity"], 0);
            assert_eq!(source["lifecycle"], 2);
            let second_settled = durable_state(&harness).await?;
            let (next, replayed, same) =
                super::super::rust_lottery::exchange(&owner, admitted, 3, &actual, None, None)
                    .await?;
            assert_eq!(replayed, UseDisposition::Committed);
            assert_eq!(same, actual);
            admitted = next;
            assert_eq!(durable_state(&harness).await?, second_settled);
            replays.push((3, actual));
        }
        let expected_receipts = if case == Case::Anniversary { 1 } else { 2 };
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_consume_transform_records"
            )
            .fetch_one(&harness.pool)
            .await?,
            expected_receipts
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        let settled = durable_state(&harness).await?;
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &cells;
        restarted.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
        deps.initialize(&restarted).await;
        for (command, raw) in &replays {
            let original = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                raw,
            )
            .map_err(debug)?;
            let request = UseCommand {
                game_session_id: fence.game_session_id,
                command_id: *command,
                item_fence: Some(fence),
            };
            let replayed = restarted
                .use_event_item(deps.actor, request, original.clone(), None, false)
                .await
                .map_err(|_| "Ground reward restart replay unavailable")?
                .ok_or("Ground reward immutable receipt")?;
            assert_eq!(replayed.disposition, UseDisposition::Committed);
            let mut changed = original;
            changed.target = UseTarget::Item(std::num::NonZeroU64::MAX);
            let refused = restarted
                .use_event_item(deps.actor, request, changed, None, false)
                .await
                .map_err(|_| "Ground reward conflict unavailable")?
                .ok_or("Ground reward intent conflict")?;
            assert_eq!(refused.disposition, UseDisposition::Rejected);
        }
        assert_eq!(
            durable_state(&harness).await?,
            settled,
            "restart/raw-intent replay preserves exact receipts, output UUIDs and custody"
        );
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED ground_reward case={case:?} source_kind=3 real_map_handle=true refusal=true replay=true"
        );
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn source_row(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}

async fn ground_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1")
        .fetch_one(&h.pool).await?)
}

async fn durable_state(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY item_instance_id),'[]') FROM game_item_instances i),'ground',(SELECT coalesce(jsonb_agg(to_jsonb(g) ORDER BY item_instance_id),'[]') FROM game_item_ground_locations g),'carried',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]') FROM game_item_container_entries e),'nested',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]') FROM game_item_nested_entries e),'receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY command_id),'[]') FROM game_item_consume_transform_records r),'craft',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY command_id),'[]') FROM game_item_craft_records r),'audit',(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY event_id),'[]') FROM game_item_audit_outbox a))")
        .fetch_one(&h.pool).await?)
}

async fn verify_receipt(
    h: &Harness,
    case: Case,
    command: u64,
    expected: &NoveltyPlan,
    raw: &[u8],
    source_before: &Value,
    source_location: &Value,
) -> TestResult {
    let record: Value = sqlx::query_scalar("SELECT to_jsonb(r) FROM game_item_consume_transform_records r WHERE command_id=$1::text::numeric")
        .bind(command.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(record["source_kind"], 3);
    assert_eq!(record["recipe"], expected.recipe);
    assert_eq!(
        record["source_count"],
        if case == Case::Anniversary { 2 } else { 1 }
    );
    assert_eq!(&record["before_item"], source_before);
    assert_eq!(&record["source_location"], source_location);
    assert_eq!(record["effects"], expected.effects);
    assert_eq!(
        record["parent_item_instance_id"].is_null(),
        case == Case::Special
    );
    let (source, intent, hash_ok): (Vec<u8>, Vec<u8>, bool) = sqlx::query_as("SELECT uuid_send(r.source_item_instance_id),r.intent,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id) WHERE r.command_id=$1::text::numeric")
        .bind(command.to_string()).fetch_one(&h.pool).await?;
    assert_eq!(source.as_slice(), id(71).as_slice());
    assert_eq!(intent, raw);
    assert!(hash_ok);
    let outputs = record["outputs"]
        .as_array()
        .ok_or("reward receipt outputs")?;
    assert_eq!(outputs.len(), expected.outputs.len());
    let mut expected_items: Vec<_> = expected
        .outputs
        .iter()
        .map(|(definition, count)| (definition.production_key.clone(), i64::from(*count)))
        .collect();
    let mut actual_items = Vec::new();
    for output in outputs {
        let uuid = output["item"].as_str().ok_or("output UUID")?;
        let (item, ground, carried): (Value, Option<Value>, Option<Value>) = sqlx::query_as(
            "SELECT to_jsonb(i),(SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id),(SELECT to_jsonb(e) FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=$1::uuid",
        ).bind(uuid).fetch_one(&h.pool).await?;
        assert_eq!(item["minted_transaction_id"], record["transaction_id"]);
        assert_eq!(item["definition_production_key"], output["key"]);
        assert_eq!(item["definition_revision_ref"], output["revision"]);
        assert_eq!(item["quantity"], output["count"]);
        assert_eq!(item["lifecycle"], 1);
        assert_ne!(item["item_instance_id"], record["source_item_instance_id"]);
        let ordinal: i64 = output["ordinal"]
            .as_str()
            .ok_or("output ordinal")?
            .parse()?;
        if case == Case::Special {
            assert!(carried.is_none());
            let ground = ground.ok_or("actual Ground reward custody")?;
            for field in [
                "world_id",
                "channel_id",
                "runtime_scope_ownership_generation",
                "spatial_position",
                "corpse_ref",
                "map_revision",
                "content_revision",
                "native_room_placement_context",
            ] {
                assert_eq!(
                    ground[field], source_location[field],
                    "Ground output field {field}"
                );
            }
            assert_eq!(ground["stack_ordinal"].as_i64(), Some(ordinal));
            assert!(
                ordinal
                    > source_location["stack_ordinal"]
                        .as_i64()
                        .ok_or("source ordinal")?
            );
        } else {
            assert!(ground.is_none());
            let carried = carried.ok_or("actual inventory reward custody")?;
            assert_eq!(carried["character_id"], record["character_id"]);
            assert_eq!(
                carried["parent_item_instance_id"],
                record["parent_item_instance_id"]
            );
            assert_eq!(carried["placement_ordinal"].as_i64(), Some(ordinal));
        }
        actual_items.push((
            item["definition_production_key"]
                .as_str()
                .ok_or("reward key")?
                .to_owned(),
            item["quantity"].as_i64().ok_or("reward count")?,
        ));
    }
    expected_items.sort();
    actual_items.sort();
    assert_eq!(actual_items, expected_items);
    Ok(())
}
