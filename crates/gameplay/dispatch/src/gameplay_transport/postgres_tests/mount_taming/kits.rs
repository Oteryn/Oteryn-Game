//! Machine targets, seven glider pieces and exact receipt replay through ordinary USE.
use super::super::fixture::{Harness as FreshHarness, debug, id, run as run_fixture};
use super::super::{Dependencies, prepare_current_session};
use super::*;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseTarget, encode_item_use_intent};
use std::num::{NonZeroU32, NonZeroU64};

const SOURCES: [&str; 4] = [
    "oteryn:item.tibia.i12305",
    "oteryn:item.tibia.i12801",
    "oteryn:item.tibia.i37397",
    "oteryn:item.tibia.i21897",
];
const TARGETS: [&str; 3] = [
    "oteryn:item.tibia.i12319",
    "oteryn:item.tibia.i12800",
    "oteryn:item.tibia.i37398",
];
const COMPONENTS: [&str; 6] = [
    "oteryn:item.tibia.i21898",
    "oteryn:item.tibia.i21906",
    "oteryn:item.tibia.i21902",
    "oteryn:item.tibia.i21899",
    "oteryn:item.tibia.i21905",
    "oteryn:item.tibia.i21901",
];

pub(super) async fn seed(h: &FreshHarness, index: &ItemDefinitionIndex) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &SOURCES.map(|key| (key, 1))).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (ordinal, key) in TARGETS[2..]
        .iter()
        .chain(COMPONENTS[..5].iter())
        .enumerate()
    {
        let item = crate::operator_files::uuid_v7().map_err(debug)?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(key).bind(index.revision(key).ok_or_else(||format!("kit profile {key}"))?).bind(id(82).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(item.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice())
            .bind(i64::try_from(ordinal + 5)?).bind(id(82).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn grant_count(h: &FreshHarness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn quantity(h: &FreshHarness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn run(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &FreshHarness,
    mut admitted: AdmittedSession,
    index: &ItemDefinitionIndex,
) -> TestResult {
    let session = admitted.game_session_id;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 14, 15, 18],
        )
        .ok_or("kit selected caps")?;
    let reference = |key: &str| -> TestResult<NonZeroU32> {
        index
            .definition_ref("Item", key, index.revision(key).ok_or("kit revision")?)
            .ok_or_else(|| "kit definition".into())
    };
    let payload = |target: bool| -> TestResult<Vec<u8>> {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(NonZeroU64::new(1).ok_or("source handle")?),
            use_with: None,
            use_with_map_item: if target {
                Some(MapUseTarget::Item(
                    NonZeroU64::new(2).ok_or("target handle")?,
                ))
            } else {
                None
            },
            use_with_position: None,
        })
        .map_err(debug)
    };
    for (i, key) in SOURCES.iter().enumerate() {
        let source = reference(key)?;
        let target = if i == 2 {
            Some(reference(TARGETS[2])?)
        } else {
            None
        };
        let actual_payload = if i < 2 {
            let map = owner.bundle_map().ok_or("kit bundle")?;
            let pos = crate::map::overlay::TilePos {
                x: u16::try_from(6 + i)?,
                y: 0,
                floor: -7,
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(
                &crate::map::view::placement_key(pos, 1)
                    .ok_or("kit machine placement")?
                    .to_be_bytes(),
            );
            encode_item_use_intent(&UseIntent {
                target: UseTarget::Item(NonZeroU64::new(1).ok_or("kit handle")?),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Object(
                    oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )),
                use_with_position: None,
            })
            .map_err(debug)?
        } else {
            payload(target.is_some())?
        };
        let before = quantity(h, key).await?;
        // The key itself is never an inoperative machine target. Glider construction
        // requires every component; it cannot be completed by a blueprint alone.
        if i < 3 {
            let command = admitted.continuity.next_command_id;
            let (next, result, _) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                command,
                &payload(true)?,
                Some(source),
                Some(source),
            )
            .await?;
            admitted = next;
            assert_eq!(result, UseDisposition::NothingToUse);
            assert_eq!(quantity(h, key).await?, before);
        }
        if i == 3 {
            let command = admitted.continuity.next_command_id;
            let (next, result, _) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                command,
                &payload(false)?,
                Some(source),
                None,
            )
            .await?;
            admitted = next;
            assert_eq!(result, UseDisposition::RequirementNotMet);
            assert_eq!(quantity(h, key).await?, before);
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let piece = crate::operator_files::uuid_v7().map_err(debug)?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
                .bind(piece.as_slice()).bind(id(42).as_slice()).bind(COMPONENTS[5])
                .bind(index.revision(COMPONENTS[5]).ok_or("hinge revision")?).bind(id(82).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,15,encode($5,'hex')::uuid)")
                .bind(piece.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(82).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let item: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 LIMIT 1")
            .bind(key).fetch_one(&h.pool).await?;
        let item: [u8; 16] = item.try_into().map_err(|_| "kit item id")?;
        let mut command = admitted.continuity.next_command_id;
        let deadline = command.checked_add(100).ok_or("kit command bound")?;
        while oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw(
            *session.as_bytes(),
            command,
            item,
            41,
            100,
        )
        .ok_or("kit roll")?
            >= if i < 2 { 61 } else { 100 }
        {
            if command >= deadline {
                return Err("deterministic kit success draw missing".into());
            }
            // Wrong targets advance the real command stream without consuming a key.
            let (next, result, _) = super::super::rust_lottery::exchange(
                owner,
                admitted,
                command,
                &payload(true)?,
                Some(source),
                Some(source),
            )
            .await?;
            admitted = next;
            assert_eq!(result, UseDisposition::NothingToUse);
            command = admitted.continuity.next_command_id;
        }
        let (next, result, actual) = super::super::rust_lottery::exchange(
            owner,
            admitted,
            command,
            &actual_payload,
            Some(source),
            target,
        )
        .await?;
        admitted = next;
        assert_eq!(result, UseDisposition::Committed, "kit {key}");
        assert_eq!(quantity(h, key).await?, before - 1);
        assert_eq!(grant_count(h).await?, i64::try_from(i + 1)?);
        let (next, result, _) =
            super::super::rust_lottery::exchange(owner, admitted, command, &actual, None, None)
                .await?;
        admitted = next;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(quantity(h, key).await?, before - 1);
        if i == 2 {
            assert_eq!(quantity(h, TARGETS[2]).await?, 0);
        }
        if i < 2 {
            let map = owner.bundle_map().ok_or("machine after use")?;
            let facts = map.facts.pinned();
            let tile = crate::map::view::compose_tile(
                &map.overlay,
                &facts,
                crate::map::overlay::TilePos {
                    x: u16::try_from(6 + i)?,
                    y: 0,
                    floor: -7,
                },
            )
            .map_err(debug)?
            .ok_or("machine tile")?;
            assert!(
                tile.entries
                    .iter()
                    .any(|e| e.facts.appearance_id == [12319, 12800][i])
            );
        }
    }
    for key in COMPONENTS {
        assert_eq!(quantity(h, key).await?, 0);
    }
    Ok(())
}

pub(super) fn map(
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    let index = native.item_index().ok_or("kit item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (
            TARGETS[0],
            reference(TARGETS[0]).ok_or("tin machine")?.get() - 1,
        ),
        (
            TARGETS[1],
            reference(TARGETS[1]).ok_or("wheel machine")?.get() - 1,
        ),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let sector = bundle.sectors.first_mut().ok_or("machine sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("entrance template")?
        .clone();
    entrance.x = 6;
    entrance.y = 1;
    sector.tiles.push(entrance);
    let bytes =
        oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
            .map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 1,
                floor: -7,
            },
        },
        world,
        channel,
        |key| {
            let policy = native.item_policy(key, "definition-r1").map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy
                    .and_then(|p| p.attributes.blocks_projectile)
                    .unwrap_or(true),
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
}

pub(super) fn case() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "mount kit fixture thread")??;
    Ok(())
}

fn scenario() -> TestResult {
    scenario_mode(false)
}

pub(super) fn carried_furniture_case() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_mode(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "carried Rotate fixture thread")??;
    Ok(())
}

fn scenario_mode(furniture: bool) -> TestResult {
    run_fixture(async |admin| {
        let started = std::time::Instant::now();
        let h = FreshHarness::create_with_admission_seed(
            admin,
            if furniture {
                "srv7_carried_rotate"
            } else {
                "srv7_mount_kits"
            },
            true,
            false,
        )
        .await?;
        super::super::reset();
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = crate::content::native_gameplay::test_support::cosmetics_fixture::activated_with_optional_cosmetics(
            deps.admitted.world_id, None, Some(&super::cosmetics()?));
        let active = controller.active().ok_or("kit generation")?;
        let native = active.native_gameplay().ok_or("kit native")?;
        let index = native.item_index().ok_or("kit index")?;
        let map = map(native, deps.admitted.world_id, deps.admitted.channel_id)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = super::super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = crate::foundation::ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            map.frame_binding_digest(deps.admitted.world_id),
            map.map_revision_digest(),
            (6, 1, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            12,
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
                crate::foundation::MovementLocalPosition {
                    x: 6,
                    y: 1,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.rooms = crate::gameplay_transport::seam_rooms::SeamRooms::new(None, Some(&room));
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("kit current fence")?;
        let facts =
            match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
                &h.root,
                &authority,
                fence.character_id,
            )
            .await
            {
                crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
                    facts,
                    ..
                }
                | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
                    facts,
                    ..
                } => facts,
                crate::gameplay_transport::spell_character_facts::CastFactsLoad::Unavailable => {
                    return Err("kit durable class facts unavailable".into());
                }
            };
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        deps.admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice()).bind(map.frame_binding_digest(deps.admitted.world_id).as_slice())
            .execute(&h.pool).await?;
        if furniture {
            seed_rotate(&h).await?;
        } else {
            seed(&h, index).await?;
        }
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        assert!(
            owner
                .observe_character_inventory(deps.actor, admitted.game_session_id)
                .await
                .is_some(),
            "a complete fresh-admission journal is necessary for the real inventory snapshot"
        );
        if furniture {
            run_rotate(&owner, &h, admitted, index).await?;
        } else {
            run(&owner, &h, admitted, index).await?;
        }
        eprintln!(
            "SRV7_NATIVE_ITEM_PG_EXECUTED furniture={furniture} elapsed={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn seed_rotate(h: &FreshHarness) -> TestResult {
    super::super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i9034", 1),
            ("oteryn:item.tibia.i2472", 1),
        ],
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(90).as_slice()).bind(id(42).as_slice()).bind(id(82).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($1,'hex')::uuid,world_id,item_instance_id,1,encode($2,'hex')::uuid FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i2472' AND lifecycle=1")
        .bind(id(90).as_slice()).bind(id(82).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn run_rotate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &FreshHarness,
    mut admitted: AdmittedSession,
    index: &ItemDefinitionIndex,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 14, 15, 18, 384],
        )
        .ok_or("Rotate capabilities")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let child_before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(n)) FROM game_item_instances i JOIN game_item_nested_entries n USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?;
    for (source_key, after_key) in [
        ("oteryn:item.tibia.i9034", "oteryn:item.tibia.i9039"),
        ("oteryn:item.tibia.i2472", "oteryn:item.tibia.i2481"),
    ] {
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let observed = index
            .definition_ref("Item", source_key, "definition-r1")
            .ok_or("Rotate source definition")?;
        let before: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1")
            .bind(source_key).fetch_one(&h.pool).await?;
        let command = admitted.continuity.next_command_id;
        let wrong = index
            .definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1")
            .ok_or("wrong observed definition")?;
        let (next, disposition, _) = super::super::rust_lottery::exchange_rotate(
            owner,
            admitted,
            command,
            &payload,
            Some(observed),
            wrong,
        )
        .await?;
        admitted = next;
        assert_eq!(disposition, UseDisposition::StaleState);
        // Refused actions retain the real 200ms World action interval.
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        let command = admitted.continuity.next_command_id;
        let (next, disposition, actual) = super::super::rust_lottery::exchange_rotate(
            owner,
            admitted,
            command,
            &payload,
            Some(observed),
            observed,
        )
        .await?;
        admitted = next;
        assert_eq!(
            disposition,
            UseDisposition::Committed,
            "carried Rotate {source_key}"
        );
        let after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(e)) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=($1->'item'->>'item_instance_id')::uuid")
            .bind(&before).fetch_one(&h.pool).await?;
        assert_eq!(after["item"]["definition_production_key"], after_key);
        assert_eq!(after["custody"], before["custody"]);
        let mut unchanged = after["item"].clone();
        for field in [
            "definition_production_key",
            "state_revision",
            "last_transaction_id",
        ] {
            unchanged[field] = before["item"][field].clone();
        }
        assert_eq!(unchanged, before["item"]);
        assert_eq!(
            after["item"]["state_revision"].as_i64(),
            before["item"]["state_revision"].as_i64().map(|r| r + 1)
        );
        let (next, disposition, _) = super::super::rust_lottery::exchange_rotate(
            owner, admitted, command, &actual, None, observed,
        )
        .await?;
        admitted = next;
        assert_eq!(disposition, UseDisposition::Committed);
        let replay: serde_json::Value = sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE i.item_instance_id=($1->'item'->>'item_instance_id')::uuid")
            .bind(&before).fetch_one(&h.pool).await?;
        assert_eq!(replay, after["item"]);
    }
    let child_after: serde_json::Value = sqlx::query_scalar("SELECT jsonb_build_object('item',to_jsonb(i),'custody',to_jsonb(n)) FROM game_item_instances i JOIN game_item_nested_entries n USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(child_before, child_after);
    let house_metadata: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_house_interior_locations)+(SELECT count(*) FROM game_item_house_reclaim_provenance)").fetch_one(&h.pool).await?;
    assert_eq!(house_metadata, 0);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE effects->>'kind'='furniture_rotation'").fetch_one(&h.pool).await?;
    assert_eq!(receipts, 2);
    Ok(())
}
