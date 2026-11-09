//! W18: real named-House cupboards use the existing container observer and item loop.
use super::*;
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;

pub(super) async fn scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    let actor = admitted.runtime_actor.ok_or("House container actor")?;
    let fence = admitted.item_fence.ok_or("House container fence")?;
    // Historical carried inventory, using the existing fixture seam. Every
    // subsequent House admission/edge change uses the real native loop owner;
    // no contained-MINT receipt or audit event is fabricated for this setup.
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    for number in 80_u8..185 {
        let parent = if number < 85 {
            72
        } else {
            80 + (number - 85) / 20
        };
        let ordinal = if number < 85 {
            i64::from(number - 80) + 1
        } else {
            i64::from((number - 85) % 20) + 1
        };
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,encode($3,'hex')::uuid)")
            .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(id(201).as_slice()).execute(&mut *fixture).await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
            .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(id(parent).as_slice()).bind(ordinal).bind(id(202).as_slice()).execute(&mut *fixture).await?;
    }
    fixture.commit().await?;
    let cupboard = ItemKey::Instance(id(71));
    let bag = ItemKey::Instance(id(72));
    let gold = ItemKey::Instance(id(73));
    const FOREIGN_HOUSE: &str = "oteryn:content.house.admiral_s_avenue_2";
    let mut foreign = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *foreign)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)",
    )
    .bind(id(206).as_slice())
    .execute(&mut *foreign)
    .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Other Container Owner',sex FROM game_character_roots WHERE character_id=encode($3,'hex')::uuid")
        .bind(id(205).as_slice()).bind(id(206).as_slice()).bind(id(101).as_slice()).execute(&mut *foreign).await?;
    foreign.commit().await?;
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(FOREIGN_HOUSE)
    .bind(id(205).as_slice())
    .execute(&h.pool)
    .await?;
    let mut foreign = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *foreign)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i34276','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(185).as_slice()).bind(id(42).as_slice()).bind(id(207).as_slice()).execute(&mut *foreign).await?;
    // Deliberately identical coordinates: the named House, not position alone,
    // determines custody even when an attacker supplies the other InstanceId.
    let at: Vec<u8> = POSITION
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect();
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(185).as_slice()).bind(id(42).as_slice()).bind(FOREIGN_HOUSE).bind(at).bind(id(208).as_slice()).execute(&mut *foreign).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(185).as_slice()).bind(id(42).as_slice()).bind(FOREIGN_HOUSE).bind(id(205).as_slice()).bind(id(208).as_slice()).execute(&mut *foreign).await?;
    foreign.commit().await?;
    let foreign_box = ItemKey::Instance(id(185));
    assert!(
        !matches!(
            owner
                .observe_container(actor, Some(fence), foreign_box)
                .await,
            Some(ContainerObservation::Container { .. })
        ),
        "same coordinates cannot grant another House's container graph"
    );
    assert_ne!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 99,
                    item_fence: Some(fence)
                },
                LoopMove {
                    item: bag,
                    count: None,
                    target: LoopTarget::Container(foreign_box)
                }
            )
            .await,
        LoopMoveResult::Committed,
        "cross-House insertion cannot borrow the current House's ACL or range"
    );
    let observe = |target| owner.observe_container(actor, Some(fence), target);
    let Some(ContainerObservation::Container {
        capacity, entries, ..
    }) = observe(cupboard).await
    else {
        return Err(
            "current empty House cupboard must open through the real container observer".into(),
        );
    };
    assert_eq!(capacity, 8);
    assert!(entries.is_empty());
    let lineage: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let move_item = |command_id, item, target| {
        owner.move_loop_item(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            LoopMove {
                item,
                count: None,
                target,
            },
        )
    };
    assert_eq!(
        move_item(100, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(cupboard).await else {
        return Err("full cupboard remains observable".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, bag);
    assert_eq!(
        move_item(101, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Refused(crate::durability::item_loop_move::LoopMoveRefusal::Overweight),
        "pickup must include the full existing106-item subtree's weight"
    );
    // Unload the five populated branches within the same House, allowing the
    // original small bag pickup checks without changing their source identities.
    for number in 80_u8..85 {
        assert_eq!(
            move_item(
                102 + u64::from(number - 80),
                ItemKey::Instance(id(number)),
                LoopTarget::Container(cupboard)
            )
            .await,
            LoopMoveResult::Committed
        );
    }
    assert_eq!(
        move_item(107, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container {
        parent, entries, ..
    }) = observe(bag).await
    else {
        return Err("nested House bag must open from its real House graph".into());
    };
    assert_eq!(parent, Some(cupboard));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    assert_eq!(entries[0].count.get(), 10);
    let subjects: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(reclaim_subject_character_id) FROM game_house_nested_reclaim_provenance ORDER BY item_instance_id")
        .fetch_all(&h.pool).await?;
    assert_eq!(subjects.len(), 107);
    assert!(
        subjects.iter().all(|subject| *subject == id(41)),
        "cupboard owner cannot acquire any item of the guest's inserted tree"
    );
    // Native reentry/reconnect projection must retain a populated cupboard.
    // House nested entries differ from the legacy carried-container table;
    // the source renderer reads the actual root, never a synthetic item.
    let house = crate::durability::house_scope_handoff::HouseId::new(owner.world_id, HOUSE)
        .ok_or("House identity for populated native snapshot")?;
    let mut snapshot_tx = h.pool.begin().await?;
    let snapshot = crate::durability::house_interior::load_house_interior_in_transaction(
        &mut snapshot_tx,
        &house,
        fence.scope_ownership_generation.get(),
        owner.holder,
    )
    .await
    .map_err(|error| format!("populated native House snapshot refused: {error:?}"))?;
    assert!(
        snapshot
            .items
            .iter()
            .any(|item| item.item_instance_id == id(71)),
        "populated cupboard survives the native House projection used on reentry"
    );
    snapshot_tx.rollback().await?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        move_item(107, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    let replay_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts, replay_receipts,
        "same command replays without a second move"
    );
    assert_eq!(
        move_item(108, gold, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(109, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(cupboard).await else {
        return Err("emptied cupboard remains observable".into());
    };
    assert_eq!(
        entries.len(),
        5,
        "the five other guest branches remain in House custody"
    );
    let after_lineage: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        lineage, after_lineage,
        "container moves never remint their existing contents"
    );
    let remaining_provenance: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_nested_reclaim_provenance")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        remaining_provenance, 105,
        "taking the small subtree out retires only its own House provenance"
    );
    // A populated bag moves as its existing subtree, without new item identities.
    assert_eq!(
        move_item(110, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(111, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(112, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = observe(bag).await else {
        return Err("populated bag returns to carried custody".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    let mut stale = fence;
    stale.scope_ownership_generation = crate::foundation::ScopeOwnershipGeneration::new(
        fence.scope_ownership_generation.get() + 1,
    )
    .map_err(debug)?;
    assert!(
        !matches!(
            owner.observe_container(actor, Some(stale), cupboard).await,
            Some(ContainerObservation::Container { .. })
        ),
        "stale House generation cannot observe contents"
    );
    assert_ne!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 113,
                    item_fence: Some(stale)
                },
                LoopMove {
                    item: gold,
                    count: None,
                    target: LoopTarget::Container(cupboard)
                }
            )
            .await,
        LoopMoveResult::Committed,
        "stale House generation cannot insert contents"
    );
    // Leave through the native transfer owner before the existing disposition
    // runner drains a historical overdue notice (same fixture seam as HOUSE-1b).
    assert_eq!(
        move_item(114, bag, LoopTarget::Container(cupboard)).await,
        LoopMoveResult::Committed
    );
    for number in 80_u8..85 {
        assert_eq!(
            move_item(
                115 + u64::from(number - 80),
                ItemKey::Instance(id(number)),
                LoopTarget::Container(bag)
            )
            .await,
            LoopMoveResult::Committed
        );
    }
    let mut exit = vec![8, 2, 18, u8::try_from(HOUSE.len())?];
    exit.extend_from_slice(HOUSE.as_bytes());
    let exit_context = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        (
            owner.movement_cells.scope().generation_digest
                == runtime.content_pin().server_artifact_digest(),
            (state.position.x, state.position.y, state.position.floor),
            (
                state.origin_position.x,
                state.origin_position.y,
                state.origin_position.floor,
            ),
        )
    };
    match owner.transfer_house(admitted, 120, &exit).await {
        Some((Some(_outside), _)) => {}
        outcome => {
            let kind = if outcome.is_none() {
                "uncertain"
            } else {
                "refused"
            };
            return Err(format!("actual House exit {kind}: same_content_generation={}, actor_position={:?}, origin_position={:?}, entrance={:?}",
                exit_context.0,exit_context.1,exit_context.2,
                crate::gameplay_transport::house_query::entrance(HOUSE).map(|p|(p.x,p.y,p.floor))).into());
        }
    }
    sqlx::query("INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at) SELECT world_id,house_key,owner_character_id,ownership_revision,1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400001,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1 FROM game_house_ownership WHERE house_key=$1")
        .bind(HOUSE).execute(&h.pool).await?;
    for _ in 0..3 {
        owner
            .drain_source_house_dispositions()
            .await
            .map_err(debug)?;
    }
    let delivered: Vec<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT uuid_send(item_instance_id),uuid_send(character_id) FROM game_item_character_inbox_locations WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) ORDER BY item_instance_id")
        .bind(id(71).as_slice()).bind(id(72).as_slice()).bind(id(73).as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        delivered,
        vec![
            (id(71).to_vec(), id(101).to_vec()),
            (id(72).to_vec(), id(101).to_vec())
        ],
        "T135 wraps the cupboard and sends the unbound bag to the seller's depot inbox"
    );
    let wrapped:String=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(wrapped,"oteryn:item.tibia.i23398","T135 cupboard becomes a kit after collecting its contents");
    let retained_parent:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(n.parent_item_instance_id) FROM game_item_nested_entries n JOIN game_item_character_inbox_locations root ON root.item_instance_id=n.parent_item_instance_id WHERE n.item_instance_id=encode($1,'hex')::uuid AND root.character_id=encode($2,'hex')::uuid")
        .bind(id(73).as_slice()).bind(id(101).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        retained_parent,
        id(72),
        "same-subject contents retain their exact bag edge through Inbox delivery"
    );
    let counts: Vec<i32> = sqlx::query_scalar(
        "SELECT tree_count FROM game_house_disposition_census ORDER BY item_instance_id",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(
        counts,
        vec![1, 107],
        "the107-item subtree remains one whole transfer across the old100-item boundary"
    );
    let remaining_provenance: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_house_nested_reclaim_provenance")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining_provenance, 0);
    eprintln!(
        "W18_PG house_container_open_insert_nested_remove_replay_stale elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn capacity_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
) -> TestResult {
    let started = std::time::Instant::now();
    assert!(
        admitted.continuity.selected_capabilities.contains(
            oteryn_protocol_oteryn::container_tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2
        )
    );
    let actor = admitted.runtime_actor.ok_or("House rack actor")?;
    let fence = admitted.item_fence.ok_or("House rack fence")?;
    let rack = ItemKey::Instance(id(71));
    let bag = ItemKey::Instance(id(72));
    let gold = ItemKey::Instance(id(73));
    assert!(
        !matches!(
            owner.observe_container(actor, Some(fence), rack).await,
            Some(ContainerObservation::Container { .. })
        ),
        "legacy observer cannot truncate a25-slot rack to20"
    );
    let Some(ContainerObservation::Container {
        capacity, entries, ..
    }) = owner
        .observe_container_with_house_capacity(actor, Some(fence), rack, true)
        .await
    else {
        return Err("negotiated House rack must open through the native observer".into());
    };
    assert_eq!(capacity, 25);
    assert!(entries.is_empty());
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let request = |item, target| LoopMove {
        item,
        count: None,
        target,
    };
    assert_ne!(
        owner
            .move_loop_item(
                actor,
                command(200),
                request(bag, LoopTarget::Container(rack))
            )
            .await,
        LoopMoveResult::Committed,
        "legacy writer cannot silently clamp source capacity25"
    );
    let move_item = |command_id, item, target| {
        owner.move_loop_item_with_house_capacity(
            actor,
            command(command_id),
            request(item, target),
            true,
        )
    };
    assert_eq!(
        move_item(201, bag, LoopTarget::Container(rack)).await,
        LoopMoveResult::Committed
    );
    assert_ne!(
        owner
            .move_loop_item(
                actor,
                command(201),
                request(bag, LoopTarget::Container(rack))
            )
            .await,
        LoopMoveResult::Committed,
        "same command cannot replay under a different authenticated capacity mode"
    );
    let attested:(i16,i16)=sqlx::query_as("SELECT a.capacity,p.capacity_limit FROM game_house_container_roots a JOIN game_house_container_loop_proofs p ON p.transaction_id=a.admitted_transaction_id WHERE a.item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        attested,
        (25, 25),
        "SQL retains exact donor capacity and negotiated ceiling"
    );
    assert_eq!(
        move_item(202, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container {
        parent, entries, ..
    }) = owner
        .observe_container_with_house_capacity(actor, Some(fence), bag, true)
        .await
    else {
        return Err("nested native rack contents must remain readable".into());
    };
    assert_eq!(parent, Some(rack));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    assert_eq!(
        move_item(202, gold, LoopTarget::Container(bag)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        move_item(203, bag, LoopTarget::MainBackpack).await,
        LoopMoveResult::Committed
    );
    let Some(ContainerObservation::Container { entries, .. }) = owner
        .observe_container_with_house_capacity(actor, Some(fence), bag, true)
        .await
    else {
        return Err("rack pickup must preserve the populated bag".into());
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, gold);
    eprintln!(
        "W18_PG House rack native25 legacy20 nested_insert_read_pickup_replay elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}
