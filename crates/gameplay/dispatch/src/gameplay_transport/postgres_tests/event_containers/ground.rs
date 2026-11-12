//! SP04-owned bags must not gain a W19 ground route, receipt or presentation.
use super::*;
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
fn w19_ground_event_container_anniversary_defers_to_sp04() -> TestResult {
    run_case(Case::Anniversary)
}

#[test]
fn w19_ground_event_container_special_defers_to_sp04() -> TestResult {
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
        owner.movement_cells = (&cells).into();
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
        assert!(oteryn_gameplay_items::reward_bag::is_reward_bag(key));
        assert!(!oteryn_gameplay_items::event_items::is_event_reward(key));
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
            UseDisposition::StaleState,
            "SP04 bags have no W19 ground route"
        );
        assert_eq!(
            durable_state(&harness).await?,
            before_refusal,
            "refusal changes no source, custody, reward, receipt or audit row"
        );
        admitted = next;

        // Free the former W19 destination capacity. SP04 still has no ground route.
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
        let before = durable_state(&harness).await?;
        let (next, outcome, actual) = super::super::rust_lottery::exchange(
            &owner,
            admitted,
            2,
            &payload,
            reference(key),
            None,
        )
        .await?;
        assert_eq!(outcome, UseDisposition::StaleState);
        assert_eq!(durable_state(&harness).await?, before);
        assert_eq!(source_row(&harness).await?, source_before);
        let retained: Value = sqlx::query_scalar("SELECT to_jsonb(g) FROM game_item_ground_locations g WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source.as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(retained, source_location);
        admitted = next;
        let (_, replayed, same) =
            super::super::rust_lottery::exchange(&owner, admitted, 2, &actual, None, None).await?;
        assert_eq!(replayed, UseDisposition::StaleState);
        assert_eq!(same, actual);
        assert_eq!(durable_state(&harness).await?, before);
        drop(owner);
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = (&cells).into();
        restarted.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
        deps.initialize(&restarted).await;
        let request = UseCommand {
            game_session_id: fence.game_session_id,
            command_id: 3,
            item_fence: Some(fence),
        };
        assert!(
            restarted
                .use_event_item(
                    deps.actor,
                    request,
                    intent.clone(),
                    Some(ItemKey::Instance(source)),
                    false
                )
                .await
                .map_err(|_| "restarted W19 ground probe")?
                .is_none()
        );
        assert!(
            restarted
                .use_reward_bag(deps.actor, request, ItemKey::Instance(source))
                .await
                .is_none()
        );
        assert_eq!(durable_state(&harness).await?, before);
        let counts: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_item_consume_transform_records), (SELECT count(*) FROM game_item_craft_records), (SELECT count(*) FROM game_item_audit_outbox)",
        ).fetch_one(&harness.pool).await?;
        assert_eq!(counts, (0, 0, 0), "no ground receipts or audit rows");
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED mode=POSTGRES ground_reward case={case:?} sp04_owner=true refusal=true replay=true"
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

pub(super) async fn durable_state(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT coalesce(jsonb_agg(to_jsonb(i) ORDER BY item_instance_id),'[]') FROM game_item_instances i),'ground',(SELECT coalesce(jsonb_agg(to_jsonb(g) ORDER BY item_instance_id),'[]') FROM game_item_ground_locations g),'carried',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]') FROM game_item_container_entries e),'nested',(SELECT coalesce(jsonb_agg(to_jsonb(e) ORDER BY item_instance_id),'[]') FROM game_item_nested_entries e),'receipts',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY command_id),'[]') FROM game_item_consume_transform_records r),'craft',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY command_id),'[]') FROM game_item_craft_records r),'audit',(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY event_id),'[]') FROM game_item_audit_outbox a))")
        .fetch_one(&h.pool).await?)
}
