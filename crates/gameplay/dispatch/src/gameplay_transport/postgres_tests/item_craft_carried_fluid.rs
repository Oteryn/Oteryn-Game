//! I11: carried flour + a filled carried vessel through composed dispatch and PostgreSQL.
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

#[derive(Clone, Copy)]
enum Case {
    Water,
    Milk,
    Empty,
    Full,
    Keg,
    KegLast,
    KegMove,
    KegFull,
    GroundFull,
    Ground,
    GroundLast,
    Nested,
    NestedLast,
}

impl Case {
    fn keg(self) -> bool {
        matches!(
            self,
            Self::Keg
                | Self::KegLast
                | Self::KegMove
                | Self::KegFull
                | Self::GroundFull
                | Self::Ground
                | Self::GroundLast
                | Self::Nested
                | Self::NestedLast
        )
    }
    fn last(self) -> bool {
        matches!(
            self,
            Self::KegLast | Self::KegFull | Self::GroundFull | Self::GroundLast | Self::NestedLast
        )
    }
    fn ground(self) -> bool {
        matches!(self, Self::Ground | Self::GroundLast | Self::GroundFull)
    }
    fn moves(self) -> bool {
        matches!(self, Self::KegMove)
    }
    fn nested(self) -> bool {
        matches!(self, Self::Nested | Self::NestedLast)
    }
}
#[test]
fn srx1_dispenser_full_parent_conserves_flasks_and_charges_postgres() -> TestResult {
    postgres_case(Case::KegFull, "srx1_keg_full_parent")?;
    postgres_case(Case::GroundFull, "srx1_cask_full_parent")
}

#[test]
fn srx1_used_keg_moves_preserve_charge_row_postgres() -> TestResult {
    postgres_case(Case::KegMove, "srx1_used_keg_move")
}

#[test]
fn srx1_dispenser_ground_and_nested_charge_custody_postgres() -> TestResult {
    for (case, name) in [
        (Case::Ground, "srx1_cask_ground"),
        (Case::GroundLast, "srx1_cask_ground_last"),
        (Case::Nested, "srx1_keg_nested"),
        (Case::NestedLast, "srx1_keg_nested_last"),
    ] {
        postgres_case(case, name)?;
    }
    Ok(())
}

#[test]
fn item_craft_carried_flour_on_water_vessel_commits_atomically_postgres() -> TestResult {
    postgres_case(Case::Water, "item_craft_carried_water")
}

#[test]
fn item_craft_carried_flour_on_milk_vessel_makes_cake_dough_postgres() -> TestResult {
    postgres_case(Case::Milk, "item_craft_carried_milk")
}

#[test]
fn item_craft_carried_flour_on_empty_vessel_is_nothing_to_use_postgres() -> TestResult {
    postgres_case(Case::Empty, "item_craft_carried_empty")
}

#[test]
fn item_craft_carried_flour_on_water_vessel_full_bag_is_no_room_postgres() -> TestResult {
    postgres_case(Case::Full, "item_craft_carried_full")
}

#[test]
fn srx1_keg_fills_flasks_and_debits_charges_atomically_postgres() -> TestResult {
    postgres_case(Case::Keg, "srx1_keg")?;
    postgres_case(Case::KegLast, "srx1_keg_last")
}

fn postgres_case(case: Case, name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case, name).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "carried craft e2e panic")??;
    Ok(())
}

fn scenario(case: Case, name: &str) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, name, true, false).await?;
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
        let flour = if case.keg() {
            "oteryn:item.tibia.i283"
        } else {
            "oteryn:item.tibia.i3603"
        };
        let vessel = if case.ground() {
            "oteryn:item.tibia.i25880"
        } else if case.keg() {
            "oteryn:item.tibia.i25904"
        } else {
            "oteryn:item.tibia.i2874"
        };
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
        let bundle_room = super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let action_room = &bundle_room;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            action_room.frame_binding().digest(),
            action_room.map_revision_digest(),
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
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(action_room));
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

        let mut entries = vec![(flour, 2), (vessel, 1)];
        if case.nested() {
            entries.push((bag, 1));
        }
        if case.moves() {
            entries.push((flour, 2));
            entries.push((bag, 1));
        }
        if matches!(case, Case::KegFull | Case::GroundFull) {
            while entries.len() < 20 {
                entries.push(("oteryn:item.tibia.i3031", 1));
            }
        }
        if matches!(case, Case::Full) {
            let policy = native
                .item_policy(bag, "definition-r1")
                .ok_or("bag policy")?
                .record();
            let crate::content::ReferenceItemField::Known(container) = &policy.semantics.container
            else {
                return Err("bag container facts".into());
            };
            let crate::content::ReferenceItemField::Known(capacity) = &container.capacity else {
                return Err("backpack capacity".into());
            };
            assert_eq!(*capacity, 20);
            for _ in 0..usize::from(*capacity - 3) {
                entries.push(("oteryn:item.tibia.i3031", 1));
            }
        }
        super::item_look::seed_main_backpack(&harness, &entries).await?;
        if case.moves() {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
                .bind(id(73).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if case.ground() || case.nested() {
            let ground = if case.ground() {
                let runtime = owner.runtime.lock().await;
                Some(
                    crate::durability::item_loop_move::LoopGround::from_native_owner(
                        action_room,
                        &runtime,
                        MovementLocalPosition {
                            x: 6,
                            y: 0,
                            floor: 7,
                        },
                        &crate::map::overlay::map_revision(&base),
                    )
                    .map_err(debug)?,
                )
            } else {
                None
            };
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).execute(&mut *tx).await?;
            if let Some(ground) = ground {
                let reference = [
                    crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                    id(79).as_slice(),
                ]
                .concat();
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,nextval('game_item_ground_stack_ordinal_seq'))")
                    .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(ground.spatial_position).bind(reference)
                    .bind(ground.map_revision).bind(ground.content_revision).bind(ground.placement_context).execute(&mut *tx).await?;
            } else {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
                    .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).execute(&mut *tx).await?;
            }
            if matches!(case, Case::GroundFull) {
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($3,'hex')::uuid)").bind(id(95).as_slice()).bind(id(42).as_slice()).bind(id(96).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1000,encode($5,'hex')::uuid)").bind(id(95).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(97).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        if case.last() {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,1,1)")
                .bind(id(72).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        if !matches!(case, Case::Empty) && !case.keg() {
            let subtype = if matches!(case, Case::Milk) {
                9_i16
            } else {
                1_i16
            };
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_fluid_states(item_instance_id,world_id,fluid_subtype,last_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,game_character_uuid_v7())")
                .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(subtype).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let initial_instances: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
            .fetch_one(&harness.pool)
            .await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("vessel")?,
            )),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let direct = case.ground() || case.nested();
        let use_command = crate::gameplay_transport::connection::UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        let (continued, result, actual) = if direct {
            let result = owner
                .use_craft(
                    deps.actor,
                    use_command,
                    intent.clone(),
                    Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                        71,
                    ))),
                    Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                        72,
                    ))),
                )
                .await
                .ok_or("dispenser production caller")?;
            (admitted, result.disposition, payload.clone())
        } else {
            exchange_carried(
                &owner,
                admitted,
                1,
                &payload,
                reference(flour),
                reference(vessel),
            )
            .await?
        };

        let flour_state: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        let vessel_state: (i16, i64) = sqlx::query_as("SELECT lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
        let craft_records: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_craft_records")
            .fetch_one(&harness.pool)
            .await?;
        match case {
            Case::GroundFull => {
                assert_eq!(
                    result,
                    UseDisposition::NoRoom,
                    "capacity must be checked before any dispenser debit/retirement"
                );
                assert_eq!(flour_state, (2, 1));
                assert_eq!(vessel_state, (1, 0));
                assert_eq!(craft_records, 0);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_consume_transform_records"
                    )
                    .fetch_one(&harness.pool)
                    .await?,
                    0
                );
                assert_eq!(sqlx::query_as::<_,(i32,String)>("SELECT charges,state_revision::text FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,(1,"1".into()));
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                        .fetch_one(&harness.pool)
                        .await?,
                    initial_instances
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_timed_state_writes"
                    )
                    .fetch_one(&harness.pool)
                    .await?,
                    0
                );
            }
            Case::Keg
            | Case::KegLast
            | Case::KegMove
            | Case::KegFull
            | Case::Ground
            | Case::GroundLast
            | Case::Nested
            | Case::NestedLast => {
                assert_eq!(result, UseDisposition::Committed);
                let last = case.last();
                assert_eq!(flour_state, if last { (1, 1) } else { (0, 2) });
                assert_eq!(
                    vessel_state,
                    if last {
                        (2, 0)
                    } else {
                        (1, i64::from(!direct))
                    }
                );
                let potion_stack: (i64, i64) = sqlx::query_as("SELECT coalesce(sum(i.quantity),0)::bigint,count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i236' AND i.lifecycle=1")
                    .bind(id(70).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(potion_stack, (if last { 1 } else { 2 }, 1));
                let charges: i32 = sqlx::query_scalar("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
                assert_eq!(
                    charges,
                    if last {
                        1
                    } else if case.ground() {
                        998
                    } else {
                        498
                    }
                );
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'")
                    .fetch_one(&harness.pool).await?;
                assert_eq!(receipts, 1);
                if matches!(case, Case::KegFull) {
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&harness.pool).await?,20,"the exhausted carried keg frees the final committed slot");
                }
                if direct {
                    let replay = owner
                        .use_craft(deps.actor, use_command, intent.clone(), None, None)
                        .await
                        .ok_or("dispenser durable replay")?;
                    assert_eq!(replay.disposition, UseDisposition::Committed);
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'").fetch_one(&harness.pool).await?,1);
                    let custody_sql = if case.ground() {
                        "SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid"
                    } else {
                        "SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
                    };
                    let custody: i64 = sqlx::query_scalar(custody_sql)
                        .bind(id(72).as_slice())
                        .fetch_one(&harness.pool)
                        .await?;
                    assert_eq!(custody, i64::from(!last));
                } else {
                    let (after_replay, replay, _) =
                        exchange_carried(&owner, continued, 1, &actual, None, None).await?;
                    assert_eq!(replay, UseDisposition::Committed);
                    let replay_receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'")
                    .fetch_one(&harness.pool).await?;
                    assert_eq!(replay_receipts, 1);
                    if case.moves() {
                        let plain = encode_item_use_intent(&UseIntent {
                            target: UseTarget::Item(
                                std::num::NonZeroU64::new(1).ok_or("plain source")?,
                            ),
                            use_with: None,
                            use_with_map_item: None,
                            use_with_position: None,
                        })
                        .map_err(debug)?;
                        let (_, plain_use, _) = exchange_carried(
                            &owner,
                            after_replay,
                            2,
                            &plain,
                            reference(vessel),
                            None,
                        )
                        .await?;
                        assert_eq!(
                            plain_use,
                            UseDisposition::NothingToUse,
                            "a dispenser consumes charges only when filling a matching flask"
                        );
                        let explicit =
                            oteryn_protocol_oteryn::timed_item::encode_timed_item_intent(
                                oteryn_protocol_oteryn::timed_item::TimedItemIntent {
                                    action: oteryn_protocol_oteryn::timed_item::TimedAction::Use,
                                    target: oteryn_protocol_oteryn::item_view::ItemHandle::new(1)
                                        .ok_or("timed handle")?,
                                    expected_revision: 1,
                                    npc: None,
                                },
                            )
                            .map_err(debug)?;
                        let explicit = owner
                            .timed_item_command(
                                deps.actor,
                                crate::gameplay_transport::connection::UseCommand {
                                    game_session_id: fence.game_session_id,
                                    command_id: 900,
                                    item_fence: Some(fence),
                                },
                                &explicit,
                                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                                    72,
                                ))),
                            )
                            .await
                            .ok_or("timed dispenser refusal")?;
                        assert_eq!(
                            oteryn_protocol_oteryn::timed_item::decode_timed_item_result(&explicit)
                                .map_err(debug)?
                                .disposition,
                            oteryn_protocol_oteryn::timed_item::TimedDisposition::Rejected
                        );

                        assert_eq!(sqlx::query_scalar::<_,i32>("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,498);
                        let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
                        let permit = owner.spell_lane_permit().await.ok_or("move lane")?;
                        let weights =
                            crate::gameplay_transport::item_ref_admission::NativeItemWeights(
                                native,
                            );
                        let moved=owner.root.commit_item_loop_move(&permit,owner.character,owner.holder,&fence,
                        crate::foundation::CommandRef::new(fence.game_session_id,crate::foundation::CommandId::new(901).map_err(debug)?),&weights,
                        crate::durability::item_loop_move::LoopMoveRequest{item:id(72),count:None,pickup_cell:None,
                            destination:crate::durability::item_loop_move::LoopDestination::Container(id(74))}).await.map_err(debug)?;
                        assert!(
                            moved.is_ok(),
                            "a used keg must move through the real loop: {moved:?}"
                        );
                        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,before);
                        drop(permit);
                        let command = crate::gameplay_transport::connection::UseCommand {
                            game_session_id: fence.game_session_id,
                            command_id: 902,
                            item_fence: Some(fence),
                        };
                        let result = owner
                            .use_craft(
                                deps.actor,
                                command,
                                intent.clone(),
                                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                                    73,
                                ))),
                                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                                    72,
                                ))),
                            )
                            .await
                            .ok_or("refill after move")?;
                        assert_eq!(result.disposition, UseDisposition::Committed);
                        assert_eq!(sqlx::query_scalar::<_,i32>("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,496);
                        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,2,"movement must not checkpoint or debit charges");
                        let frozen:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
                        let permit = owner.spell_lane_permit().await.ok_or("storage lane")?;
                        let ground = {
                            let runtime = owner.runtime.lock().await;
                            crate::durability::item_loop_move::LoopGround::from_native_owner(
                                action_room,
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
                        for (number, pickup_cell, destination) in [
                            (
                                903,
                                None,
                                crate::durability::item_loop_move::LoopDestination::Ground(
                                    ground.clone(),
                                ),
                            ),
                            (
                                904,
                                Some(ground.spatial_position.clone()),
                                crate::durability::item_loop_move::LoopDestination::MainBackpack,
                            ),
                        ] {
                            let moved = owner
                                .root
                                .commit_item_loop_move(
                                    &permit,
                                    owner.character,
                                    owner.holder,
                                    &fence,
                                    crate::foundation::CommandRef::new(
                                        fence.game_session_id,
                                        crate::foundation::CommandId::new(number).map_err(debug)?,
                                    ),
                                    &weights,
                                    crate::durability::item_loop_move::LoopMoveRequest {
                                        item: id(72),
                                        count: None,
                                        pickup_cell,
                                        destination,
                                    },
                                )
                                .await
                                .map_err(debug)?;
                            assert!(
                                moved.is_ok(),
                                "frozen dispenser Ground roundtrip {number}: {moved:?}"
                            );
                            assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,frozen);
                        }
                        for (number, deposit) in [(905, true), (906, false)] {
                            let moved = owner
                                .root
                                .commit_depot_move(
                                    &permit,
                                    owner.character,
                                    owner.holder,
                                    &fence,
                                    crate::foundation::CommandRef::new(
                                        fence.game_session_id,
                                        crate::foundation::CommandId::new(number).map_err(debug)?,
                                    ),
                                    &weights,
                                    crate::durability::item_loop_move::DepotMove {
                                        town: 1,
                                        box_number: 1,
                                        deposit,
                                        request_digest: [if deposit { 5 } else { 6 }; 32],
                                        premium: crate::domain::premium::PremiumGate::CLOSED,
                                    },
                                    id(72),
                                )
                                .await
                                .map_err(debug)?;
                            assert!(
                                moved.is_ok(),
                                "frozen dispenser depot roundtrip {number}: {moved:?}"
                            );
                            assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,frozen);
                        }
                        // Materialize the corpse through its production owner, including
                        // the D133 top-damage/exclusivity receipt. Restore only the child's
                        // historical placement; TRANSFER pickup/replay remains fully guarded.
                        use crate::durability::item_mint::{
                            CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause,
                            ItemMintOutcome, ItemMintRequest,
                        };
                        let mut corpse = owner
                            .root
                            .freeze_item_mint(
                                owner.character,
                                owner.holder,
                                ItemMintRequest {
                                    cause: ItemMintCause::for_test(
                                        owner.world_id,
                                        owner.channel_id,
                                        fence.scope_ownership_generation,
                                        99,
                                        1,
                                        crate::durability::item_mint::TypedDefinitionRef {
                                            family: "LootTable".into(),
                                            production_key: "fixture:loot.srx1-storage".into(),
                                            revision_ref: "definition-r1".into(),
                                        },
                                        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                                        0,
                                    ),
                                    item: crate::durability::item_mint::TypedDefinitionRef {
                                        family: "Item".into(),
                                        production_key: "oteryn:item.tibia.i5973".into(),
                                        revision_ref: "definition-r1".into(),
                                    },
                                    quantity: 1,
                                    ground: GroundPlacement {
                                        spatial_position: ground.spatial_position.clone(),
                                        corpse_ref: id(78).to_vec(),
                                        map_revision: ground.map_revision.clone(),
                                        content_revision: ground.content_revision.clone(),
                                        native_room_placement_context: ground
                                            .placement_context
                                            .clone(),
                                    },
                                    content_revision: "content-1".into(),
                                    ruleset_revision: "ruleset-1".into(),
                                    sim_revision: "sim-1".into(),
                                },
                            )
                            .await
                            .map_err(debug)?;
                        let corpse = match owner
                            .root
                            .commit_corpse_mint(
                                &permit,
                                owner.character,
                                owner.holder,
                                &mut corpse,
                                *fence.character_id.as_bytes(),
                            )
                            .await
                            .map_err(debug)?
                        {
                            ItemMintOutcome::Committed(receipt) => receipt.item_instance_id,
                            other => {
                                return Err(format!("corpse materialization: {other:?}").into());
                            }
                        };
                        let mut tx = harness.pool.begin().await?;
                        sqlx::query("SET LOCAL session_replication_role=replica")
                            .execute(&mut *tx)
                            .await?;
                        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).execute(&mut *tx).await?;
                        sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                        .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(corpse.as_slice()).bind(id(79).as_slice()).execute(&mut *tx).await?;
                        tx.commit().await?;
                        let definition =
                            |key: &str| crate::durability::item_mint::TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: "definition-r1".into(),
                            };
                        let facts = |key: &str| {
                            crate::durability::item_loop_move::ItemLoopContent::loop_facts(
                                &weights,
                                &definition(key),
                            )
                            .ok_or("native transfer facts")
                        };
                        let mut pickup=owner.root.freeze_item_transfer(owner.character,owner.holder,fence,
                        crate::durability::item_transfer::ItemTransferRequest{
                            weight_generation_digest:crate::durability::item_transfer::ItemWeightLookup::generation_digest(&weights),
                            command:crate::foundation::CommandRef::new(fence.game_session_id,crate::foundation::CommandId::new(907).map_err(debug)?),
                            source_item_instance_id:id(72),destination:crate::durability::item_transfer::ItemTransferDestination::MainBackpack,
                            item:facts(vessel)?,backpack:Some(facts(bag)?),content_revision:"content-1".into(),ruleset_revision:"ruleset-1".into(),sim_revision:"sim-1".into()},&weights).await.map_err(|error|format!("corpse freeze: {error:?}"))?;
                        for _ in 0..2 {
                            let _ = owner
                                .root
                                .commit_item_transfer(
                                    &permit,
                                    owner.character,
                                    owner.holder,
                                    fence,
                                    &mut pickup,
                                    &weights,
                                )
                                .await
                                .map_err(|error| format!("corpse commit: {error:?}"))?;
                        }
                        assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,frozen);
                        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_corpse_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,0);
                        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?,2);
                        drop(permit);
                        drop(owner);
                        drop(authority);
                        drop(seal);
                        return harness.cleanup().await;
                    }
                    // A fresh owner has no ingress result cache: this must resolve the
                    // immutable item-action receipt even though input handles changed.
                    let mut restarted = deps.owner(&harness, &authority);
                    restarted.active_generation = Some(active);
                    restarted.movement_cells = (&cells).into();
                    restarted.rooms =
                        crate::gameplay_transport::SeamRooms::new(None, Some(action_room));
                    deps.initialize(&restarted).await;
                    let (_, recovered, _) =
                        exchange_carried(&restarted, admitted, 1, &actual, None, None).await?;
                    assert_eq!(recovered, UseDisposition::Committed);
                    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'").fetch_one(&harness.pool).await?, 1);
                    assert_eq!(sqlx::query_scalar::<_, i32>("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?, if last {1} else if case.ground() {998} else {498});
                }
            }
            Case::Water | Case::Milk => {
                assert_eq!(result, UseDisposition::Committed);
                assert_eq!(flour_state, (1, 1));
                assert_eq!(vessel_state, (2, 0));
                assert_eq!(craft_records, 1);
                assert_eq!(
                    live_backpack_item_count(&harness, vessel, Some(72)).await?,
                    1
                );
                let output = if matches!(case, Case::Water) {
                    "oteryn:item.tibia.i3604"
                } else {
                    "oteryn:item.tibia.i6276"
                };
                assert_eq!(live_backpack_item_count(&harness, output, None).await?, 1);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                        .fetch_one(&harness.pool)
                        .await?,
                    initial_instances + 2
                );
            }
            Case::Empty => {
                assert_eq!(result, UseDisposition::NothingToUse);
                assert_eq!(flour_state, (2, 1));
                assert_eq!(vessel_state, (1, 1));
                assert_eq!(craft_records, 0);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                        .fetch_one(&harness.pool)
                        .await?,
                    initial_instances
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fluid_states")
                        .fetch_one(&harness.pool)
                        .await?,
                    0
                );
            }
            Case::Full => {
                assert_eq!(result, UseDisposition::NoRoom);
                assert_eq!(flour_state, (2, 1));
                assert_eq!(vessel_state, (1, 1));
                assert_eq!(craft_records, 0);
                assert_eq!(sqlx::query_scalar::<_, i16>("SELECT fluid_subtype FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?, 1);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                        .fetch_one(&harness.pool)
                        .await?,
                    initial_instances
                );
            }
        }
        drop(owner);
        if matches!(case, Case::Water) {
            // Reconstruct the owner and replay the original command on the original admission.
            let mut restarted = deps.owner(&harness, &authority);
            restarted.active_generation = Some(active);
            restarted.movement_cells = (&cells).into();
            restarted.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(action_room));
            deps.initialize(&restarted).await;
            let output = "oteryn:item.tibia.i3604";
            let (_, replay, _) =
                exchange_carried(&restarted, admitted, 1, &actual, None, None).await?;
            assert_eq!(replay, UseDisposition::Committed);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_craft_records")
                    .fetch_one(&harness.pool)
                    .await?,
                1
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
                    .fetch_one(&harness.pool)
                    .await?,
                initial_instances + 2
            );
            assert_eq!(
                live_backpack_item_count(&harness, vessel, Some(72)).await?,
                1
            );
            assert_eq!(live_backpack_item_count(&harness, output, None).await?, 1);
            assert_eq!(sqlx::query_as::<_, (i64, i16)>("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?, (1, 1));
            assert_eq!(sqlx::query_as::<_, (i16, i64)>("SELECT lifecycle,(SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&harness.pool).await?, (2, 0));
            drop(restarted);
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn live_backpack_item_count(
    harness: &Harness,
    key: &str,
    excluded_seed: Option<u8>,
) -> TestResult<i64> {
    let count = if let Some(seed) = excluded_seed {
        sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.quantity=1 AND i.lifecycle=1 AND i.item_instance_id<>encode($3,'hex')::uuid")
            .bind(id(70).as_slice())
            .bind(key)
            .bind(id(seed).as_slice())
            .fetch_one(&harness.pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 AND i.quantity=1 AND i.lifecycle=1")
            .bind(id(70).as_slice())
            .bind(key)
            .fetch_one(&harness.pool)
            .await?
    };
    Ok(count)
}

async fn exchange_carried(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    payload: &[u8],
    source_definition: Option<std::num::NonZeroU32>,
    vessel_definition: Option<std::num::NonZeroU32>,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_task = async {
        let mut source = None;
        let mut vessel = None;
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
                        let inventory =
                            oteryn_protocol_oteryn::item_view::decode_character_inventory(
                                domain.payload,
                            )
                            .map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == source_definition)
                            .map(|entry| entry.handle);
                        vessel = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == vessel_definition)
                            .map(|entry| entry.handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if source_definition.is_some() || vessel_definition.is_some() {
            let mut intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            intent.target = UseTarget::Item(source.ok_or("current source handle")?);
            if intent.use_with_map_item.is_some() {
                intent.use_with_map_item =
                    Some(MapUseTarget::Item(vessel.ok_or("current vessel handle")?));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &actual,
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
                    let disposition =
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(debug)?;
                    return Ok::<_, Box<dyn std::error::Error>>((disposition, actual));
                }
            }
        }
    };
    use std::future::Future;
    use std::task::Poll;
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut serving = std::pin::pin!(serving);
    let mut client_task = std::pin::pin!(client_task);
    let mut end = None;
    let mut result = None;
    // This direct connection has no node task to replenish a retired DB holder.
    let mut maintenance = std::pin::pin!(super::item_craft::maintain_ready(owner));
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|context| {
            if let Poll::Ready(result) = maintenance.as_mut().poll(context) {
                return Poll::Ready(result);
            }
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(context)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(context)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }),
    )
    .await??;
    let end = end.ok_or("server did not finish")?;
    let ConnectionEnd::AdmittedThenDisconnected(admitted) = end else {
        return Err(format!("{end:?}").into());
    };
    let (result, actual) = result.ok_or("client did not finish")??;
    Ok((admitted, result, actual))
}
