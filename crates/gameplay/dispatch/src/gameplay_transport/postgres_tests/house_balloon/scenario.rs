use super::*;

pub(super) async fn scenario(h: &Harness, case: Case) -> TestResult {
    super::super::reset();
    let (before_key, after_key) = match case {
        Case::Decoration(before, after) => (
            format!("oteryn:item.tibia.i{before}"),
            format!(
                "oteryn:item.tibia.i{}",
                if after == 0 { before } else { after }
            ),
        ),
        Case::CarpetTile { .. } => (
            "oteryn:item.tibia.i22736".to_owned(),
            "oteryn:item.tibia.i22737".to_owned(),
        ),
        Case::TimedRotation | Case::TimedNearExpiry | Case::TimedWrap => (
            "oteryn:item.tibia.i35949".to_owned(),
            "oteryn:item.tibia.i35969".to_owned(),
        ),
        Case::BedKit(kit) => (
            format!("oteryn:item.tibia.i{kit}"),
            format!("oteryn:item.tibia.i{kit}"),
        ),
        Case::ContainerCapacity => (
            "oteryn:item.tibia.i10210".to_owned(),
            "oteryn:item.tibia.i10210".to_owned(),
        ),
        Case::Container => (
            "oteryn:item.tibia.i34276".to_owned(),
            "oteryn:item.tibia.i34276".to_owned(),
        ),
        Case::NativeNoop(source) => (
            format!("oteryn:item.tibia.i{source}"),
            format!("oteryn:item.tibia.i{source}"),
        ),
        _ => (BEFORE.to_owned(), AFTER.to_owned()),
    };
    let other_key = match case {
        Case::CarpetTile { other, .. } => format!("oteryn:item.tibia.i{other}"),
        _ => String::new(),
    };
    let before_key = before_key.as_str();
    let after_key = after_key.as_str();
    let placed = matches!(
        case,
        Case::Placed
            | Case::Decoration(_, _)
            | Case::NativeNoop(_)
            | Case::StaticWindow
            | Case::StaticAqueduct
            | Case::TimedRotation
            | Case::TimedNearExpiry
            | Case::TimedWrap
            | Case::Container
            | Case::ContainerCapacity
            | Case::CarpetTile { .. }
    );
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = if case == Case::StaticAqueduct {
        super::super::house_wrap::static_timed_decoration_bundle(
            base.admitted.world_id,
            base.admitted.channel_id,
        )?
    } else if matches!(case, Case::BedKit(_)) {
        super::super::house_wrap::static_bed_decoration_bundle(
            base.admitted.world_id,
            base.admitted.channel_id,
        )?
    } else if case == Case::StaticWindow {
        super::super::house_wrap::static_decoration_bundle(
            base.admitted.world_id,
            base.admitted.channel_id,
        )?
    } else {
        super::super::house_wrap::wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?
    };
    let (controller, native_pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            base.admitted.world_id,
            None,
        );
    // Cell collision and gameplay must use the same activated artifact. Native
    // House exit independently checks this generation before admitting a tile.
    let room = native_pin.qualified_room().clone();
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let index = native.item_index().ok_or("Item index")?;
    let before_ref = index
        .definition_ref("Item", before_key, "definition-r1")
        .ok_or("balloon input")?;
    let after_ref = index
        .definition_ref("Item", after_key, "definition-r1")
        .ok_or("balloon output")?;
    let inner_ref = index
        .definition_ref("Item", INNER, "definition-r1")
        .ok_or("inner bag")?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        [2; 32],
        [3; 32],
        [4; 32],
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, room, base.spells)?;
    let (required, offers): (&[u32], &'static [OfferedCapability]) =
        if placed || matches!(case, Case::BedKit(_)) {
            (&[11, 18], MAP_OFFERS)
        } else {
            (&[], INVENTORY_OFFERS)
        };
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        required, crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id), 1, "policy-1", offers,
    ).map_err(debug)?;
    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    // Historical fixture only: a distinct owner Character, then a real guest ACL
    // entry. The production access function and all write guards remain enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($1,'hex')::uuid,r.account_id,r.world_id,r.lifecycle,r.character_revision,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,'Balloon House Owner',r.sex FROM game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid")
        .bind(id(101).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    // Timed wrapping exercises the actual owner-only wrap contract; every
    // ordinary decoration case continues to enter as a separately authorized guest.
    let house_owner = if case == Case::TimedWrap {
        id(41)
    } else {
        id(101)
    };
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(house_owner.as_slice())
    .execute(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    if case != Case::TimedWrap {
        sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let role: String = sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        role,
        if case == Case::TimedWrap {
            "OWNER"
        } else {
            "GUEST"
        }
    );
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("House")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    let capabilities: &[u32] = match case {
        Case::NativeNoop(_)
        | Case::Decoration(2358, _)
        | Case::Decoration(34278, _)
        | Case::StaticAqueduct
        | Case::TimedRotation
        | Case::TimedNearExpiry
        | Case::TimedWrap => &[4, 6, 11, 15, 18, 49, 86, 445],
        Case::Container => &[4, 6, 11, 12, 14, 15, 18, 49, 86],
        Case::ContainerCapacity => &[4, 6, 11, 12, 14, 15, 18, 49, 86, 446],
        Case::BedKit(_) => &[4, 6, 11, 15, 18, 49, 86],
        Case::Placed | Case::Decoration(_, _) | Case::StaticWindow | Case::CarpetTile { .. } => {
            &[4, 6, 11, 15, 18, 49, 86]
        }
        Case::Nested => &[4, 6, 12, 14, 15, 49, 86],
        Case::Carried | Case::Foreign => &[4, 6, 15, 49, 86],
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), capabilities)
            .ok_or("House USE offers")?;
    let contents = if matches!(case, Case::CarpetTile { .. }) {
        vec![(before_key, 1), (other_key.as_str(), 1)]
    } else if matches!(case, Case::Container | Case::ContainerCapacity) {
        vec![(before_key, 1), (INNER, 1), ("oteryn:item.tibia.i3031", 10)]
    } else if case == Case::Nested {
        vec![(before_key, 1), (INNER, 1)]
    } else {
        vec![(before_key, 1)]
    };
    if matches!(
        case,
        Case::Nested | Case::Container | Case::ContainerCapacity
    ) {
        // Capability 12 reads the existing equipment owner even with no worn items.
        // Its independent activation fact must agree with the live runtime pin.
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(active.identity().server_artifact_digest().as_slice())
            .bind([2_u8; 32].as_slice()).bind([3_u8; 32].as_slice())
            .execute(&h.pool).await?;
    }
    super::super::item_look::seed_main_backpack(h, &contents).await?;
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let before = item(h).await?;
    if !matches!(
        case,
        Case::Decoration(_, _)
            | Case::NativeNoop(_)
            | Case::Container
            | Case::ContainerCapacity
            | Case::StaticAqueduct
            | Case::TimedRotation
            | Case::TimedNearExpiry
            | Case::TimedWrap
            | Case::BedKit(_)
            | Case::CarpetTile { .. }
    ) {
        let (next, refused, _) = super::super::rust_lottery::exchange(
            &owner,
            admitted,
            1,
            &body,
            Some(before_ref),
            None,
        )
        .await?;
        admitted = next;
        assert_ne!(
            refused,
            UseDisposition::Committed,
            "a carried balloon outside a House cannot transform"
        );
        assert_eq!(item(h).await?, before);
        assert_eq!(receipts(h).await?, 0);
    }
    let mut transfer = vec![8, 1, 18, HOUSE.len() as u8];
    transfer.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 2, &transfer)
        .await
        .ok_or("House transfer")?;
    let mut admitted = inside.ok_or("guest House admission")?;
    assert_eq!(
        admitted.continuity.next_command_id, 1,
        "the actual House handoff creates a new session"
    );
    let actor_position = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(
                admitted.runtime_actor.ok_or("House actor")?,
                admitted.game_session_id,
            )
            .map_err(debug)?;
        [
            state.position.x,
            state.position.y,
            i32::from(state.position.floor),
        ]
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for at in [POSITION, actor_position] {
        let position: Vec<u8> = at.iter().flat_map(|v| v.to_be_bytes()).collect();
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(HOUSE).bind(position).execute(&mut *tx).await?;
    }
    if placed || case == Case::Nested {
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(71).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    if placed {
        let position: Vec<u8> = POSITION.iter().flat_map(|v| v.to_be_bytes()).collect();
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
            .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
            .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(house_owner.as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    } else if case == Case::Nested {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    }
    if matches!(case, Case::CarpetTile { .. }) {
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(72).as_slice())
        .execute(&mut *tx)
        .await?;
        let position: Vec<u8> = POSITION
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect();
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,2,encode($5,'hex')::uuid)")
            .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(position).bind(id(75).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
            .bind(id(72).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(101).as_slice()).bind(id(75).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    if case == Case::Foreign {
        // Observe the real current handle, then move its historical fixture to a
        // distinct Character before sending USE. A visible handle is no custody grant.
        let (_, refused, _, deltas) = exchange(
            &owner,
            admitted,
            1,
            &body,
            HouseExchange {
                current: Some(before_ref),
                after: None,
                nested: None,
                foreign_after_snapshot: Some(h),
            },
        )
        .await?;
        assert_eq!(refused, UseDisposition::StaleState);
        assert_eq!(deltas, 0);
        assert_eq!(item(h).await?, before);
        assert_eq!(receipts(h).await?, 0);
        let other: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(other, id(101));
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED house_balloon outside_house_and_changed_character_custody_refuse"
        );
        return Ok(());
    }
    if case == Case::TimedNearExpiry {
        return super::timed::near_expiry_scenario(&owner, h, admitted).await;
    }
    if case == Case::TimedWrap {
        return super::timed::wrap_scenario(&owner, h, admitted).await;
    }
    if case == Case::TimedRotation {
        return super::timed::scenario(&owner, h, admitted).await;
    }
    if case == Case::StaticAqueduct {
        return super::super::house_base_decorations::aqueduct_scenario(&owner, h, admitted).await;
    }
    if let Case::BedKit(kit) = case {
        return super::super::house_base_decorations::bed_scenario(&owner, h, admitted, kit).await;
    }
    if case == Case::ContainerCapacity {
        return super::container::capacity_scenario(&owner, h, admitted).await;
    }
    if case == Case::Container {
        return super::container::scenario(&owner, h, admitted).await;
    }
    if matches!(
        case,
        Case::NativeNoop(_) | Case::Decoration(2358, _) | Case::Decoration(34278, _)
    ) {
        return super::rotation::scenario(&owner, h, admitted, case, &before).await;
    }
    if matches!(
        case,
        Case::Decoration(_, _) | Case::NativeNoop(_) | Case::CarpetTile { .. }
    ) {
        return super::decorations::scenario(&owner, h, admitted, case, &body, &before).await;
    }
    if case == Case::StaticWindow {
        return super::super::house_base_decorations::window_scenario(&owner, h, admitted).await;
    }
    let original_custody = custody(h).await?;
    if case == Case::Nested {
        assert_house_equipment_observation(&owner, admitted).await?;
    }
    let nested = (case == Case::Nested).then_some(inner_ref);
    let command = if nested.is_some() { 2 } else { 1 };
    let (next, result, original, delta_count) = exchange(
        &owner,
        admitted,
        command,
        &body,
        HouseExchange {
            current: Some(before_ref),
            after: Some(after_ref),
            nested,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(delta_count, 1);
    let mut after = before.clone();
    after["definition_production_key"] = AFTER.into();
    after["state_revision"] = serde_json::json!(2);
    let stored = item(h).await?;
    after["last_transaction_id"] = stored["last_transaction_id"].clone();
    assert_eq!(
        stored, after,
        "exact instance/quantity/minted lineage is retained"
    );
    assert_eq!(
        custody(h).await?,
        original_custody,
        "the actual carried/House placement and reclaim provenance are retained"
    );
    assert_eq!(receipts(h).await?, 1);
    let at = if placed { POSITION } else { actor_position };
    let mut expected = oteryn_gameplay_items::novelty_items::plan_ground(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: BEFORE.into(),
            revision_ref: "definition-r1".into(),
        },
        *admitted.game_session_id.as_bytes(),
        command,
        id(71),
        oteryn_gameplay_items::novelty_items::GroundEffect {
            x: at[0],
            y: at[1],
            floor: i16::try_from(at[2])?,
        },
        true,
    )
    .ok_or("source House balloon plan")?;
    expected.effects["house_after"] =
        serde_json::json!({"key": AFTER, "revision": "definition-r1"});
    let retained: Value = sqlx::query_scalar(
        "SELECT effects FROM game_item_consume_transform_records WHERE source_kind=5",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        retained, expected.effects,
        "exact donor cues remain durable while presentation is unavailable"
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_audit_outbox a JOIN game_item_consume_transform_records r USING(transaction_id) WHERE r.source_kind=5")
        .fetch_one(&h.pool).await?, 1);

    // Recreate the command directory; the historical raw wire bytes contain a
    // now-obsolete handle and must replay the receipt before resolving it.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        required, crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id), 1, "policy-1", offers,
    ).map_err(debug)?;
    admitted.continuity.next_command_id = 1;
    // For a nested source, OPEN1 is executed again before the original USE2.
    // The fresh directory never invents a high-water mark or a retained result.
    let (next, result, replay, delta_count) = exchange(
        &owner,
        admitted,
        command,
        &original,
        HouseExchange {
            current: None,
            after: None,
            nested,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(replay, original);
    assert_eq!(
        delta_count, 0,
        "replay neither transforms again nor publishes a second mutation"
    );
    assert_eq!(item(h).await?, after);
    assert_eq!(custody(h).await?, original_custody);
    assert_eq!(receipts(h).await?, 1);
    let reverse = if nested.is_some() { 4 } else { 2 };
    let (_, result, _, deltas) = exchange(
        &owner,
        admitted,
        reverse,
        &body,
        HouseExchange {
            current: Some(after_ref),
            after: Some(before_ref),
            nested,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(deltas, 1);
    assert_eq!(item(h).await?["definition_production_key"], BEFORE);
    assert_eq!(item(h).await?["state_revision"], 3);
    assert_eq!(custody(h).await?, original_custody);
    assert_eq!(receipts(h).await?, 2);
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        if let Some(publisher) = states.presentations.as_mut() {
            assert!(
                publisher
                    .take_for_current_publisher(&runtime)
                    .map_err(debug)?
                    .is_empty(),
                "ANSWER A: House balloon cues are retained, never enqueued without a client consumer"
            );
        }
    }
    eprintln!(
        "W19 POSTGRES_CASE_COMPLETED house_balloon case={case:?} exact_custody_replay_and_reverse"
    );
    Ok(())
}

// A named House retains its Channel carrier actor ref. Capability 12 must
// nevertheless observe that actor through the actual House equipment owner.
async fn assert_house_equipment_observation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("House equipment actor")?;
    let fence = admitted
        .item_fence
        .ok_or("House equipment admitted fence")?;
    assert!(
        matches!(
            actor.runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ),
        "the real House handoff retains its Channel carrier actor ref"
    );
    {
        let runtime = owner.runtime.lock().await;
        let house = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        assert!(
            matches!(
                house.scope,
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            ),
            "the actual House resident has an independent named-House scope"
        );
        assert_eq!(house.scope, fence.runtime_scope);
        assert_eq!(house.generation, fence.scope_ownership_generation);
        assert_eq!(
            house.lease.character_id().as_bytes(),
            fence.character_id.as_bytes()
        );
        assert_eq!(house.lease.generation(), fence.character_lease_generation);
    }
    let projected = owner
        .observe_character_equipment(actor, admitted.game_session_id)
        .await
        .ok_or("House equipment projection under the actual resident is unavailable")?;
    assert!(projected.is_empty(), "the valid fixture has no worn items");
    Ok(())
}

pub(super) async fn item(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn custody(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('container',(SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid),'nested',(SELECT to_jsonb(n) FROM game_item_nested_entries n WHERE item_instance_id=encode($1,'hex')::uuid),'equipment',(SELECT to_jsonb(s) FROM game_character_equipment_slots s WHERE item_instance_id=encode($1,'hex')::uuid),'house',(SELECT to_jsonb(h) FROM game_item_house_interior_locations h WHERE item_instance_id=encode($1,'hex')::uuid),'provenance',(SELECT to_jsonb(p) FROM game_item_house_reclaim_provenance p WHERE item_instance_id=encode($1,'hex')::uuid))")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&h.pool)
            .await?,
    )
}
