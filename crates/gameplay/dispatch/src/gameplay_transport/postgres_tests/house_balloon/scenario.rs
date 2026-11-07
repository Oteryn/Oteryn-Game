use super::*;

pub(super) async fn scenario(
    h: &Harness,
    case: Case,
    before_key: &str,
    after_key: &str,
) -> TestResult {
    let house_key = if case == Case::StaticBase {
        super::static_base::HOUSE
    } else {
        HOUSE
    };
    let fixture_entry = if case == Case::StaticBase {
        super::static_base::ENTRY
    } else {
        [32309, 32805, 7]
    };
    let fixture_position = if case == Case::StaticBase {
        super::static_base::NEAR_TORCH
    } else if case == Case::ArtistEasel {
        super::artist_easel::PLACE
    } else {
        POSITION
    };
    super::super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let (controller, native_pin) = if case == Case::ArtistEasel
        && std::env::var_os("OTERYN_SRV7_ARTIST_WRAP_CAPTURE").is_some()
    {
        super::super::raid_live::hare_production_activation::activate(
            base.admitted.world_id,
            "OTERYN_SRV7_ARTIST_WRAP_CLIENT_ARTIFACT",
        )?
    } else if matches!(
        case,
        Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
    ) {
        super::super::death_ml_loss_composed::production_generation_with_pin(
            base.admitted.world_id,
            "content-1",
        )?
    } else {
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            base.admitted.world_id,
            None,
        )
    };
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let index = native.item_index().ok_or("Item index")?;
    let world = if case == Case::ArtistEasel {
        super::artist_easel::world(base.admitted.world_id, base.admitted.channel_id, native)?
    } else if case == Case::StaticBase {
        super::static_base::world(base.admitted.world_id, base.admitted.channel_id, native)?
    } else if case == Case::GiftDragonDeathHigh {
        super::gift_death::world(base.admitted.world_id, base.admitted.channel_id, native)?
    } else if case == Case::GiftPandaFar {
        super::gifts::far_world(base.admitted.world_id, base.admitted.channel_id)?
    } else {
        super::super::house_wrap::wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?
    };
    let cells = world
        .movement_cells(native_pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = if matches!(
        case,
        Case::ArtistEasel | Case::StaticBase | Case::GiftDragonDeathHigh
    ) {
        native_pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                world.frame_binding_digest(base.admitted.world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: if matches!(case, Case::StaticBase | Case::GiftDragonDeathHigh) {
                        fixture_entry[0]
                    } else {
                        33572
                    },
                    y: if matches!(case, Case::StaticBase | Case::GiftDragonDeathHigh) {
                        fixture_entry[1]
                    } else {
                        31866
                    },
                    floor: if matches!(case, Case::StaticBase | Case::GiftDragonDeathHigh) {
                        i16::try_from(fixture_entry[2])?
                    } else {
                        12
                    },
                },
            )
            .ok_or("actual easel native room")?
    } else {
        base.room.clone()
    };
    let before_ref = index
        .definition_ref("Item", before_key, "definition-r1")
        .ok_or("balloon input")?;
    let after_ref = index
        .definition_ref("Item", after_key, "definition-r1")
        .ok_or("balloon output")?;
    let inner_ref = index
        .definition_ref("Item", INNER, "definition-r1")
        .ok_or("inner bag")?;
    let start = if case == Case::ArtistEasel {
        (33572, 31866, 12)
    } else if case == Case::StaticBase {
        (
            fixture_entry[0],
            fixture_entry[1],
            i16::try_from(fixture_entry[2])?,
        )
    } else {
        (32309, 32805, 7)
    };
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        if matches!(
            case,
            Case::ArtistEasel | Case::StaticBase | Case::GiftDragonDeathHigh
        ) {
            room.compiled().client_digest()
        } else {
            [2; 32]
        },
        if matches!(
            case,
            Case::ArtistEasel | Case::StaticBase | Case::GiftDragonDeathHigh
        ) {
            room.frame_binding().digest()
        } else {
            [3; 32]
        },
        if matches!(
            case,
            Case::ArtistEasel | Case::StaticBase | Case::GiftDragonDeathHigh
        ) {
            room.map_revision_digest()
        } else {
            [4; 32]
        },
        start,
    );
    let deps = Dependencies::with_content(pin, room, base.spells)?;
    let (required, offers): (&[u32], &'static [OfferedCapability]) = if case.placed()
        || matches!(
            case,
            Case::ArtistEasel | Case::NativeMove | Case::StaticBase
        ) {
        (&[11, 18], MAP_OFFERS)
    } else {
        (&[], INVENTORY_OFFERS)
    };
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        required, crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id), 1, "policy-1", offers,
    ).map_err(debug)?;
    if !case.gift() {
        deps.initialize(&owner).await;
    }
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: start.0,
                    y: start.1,
                    floor: start.2,
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
    if case.gift() {
        let current = owner
            .current_quest_fence(deps.admitted.game_session_id)
            .await
            .map_err(|()| "House gift actual class fence")?
            .ok_or("House gift actual class fence")?;
        let facts =
            match crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
                &h.root,
                &authority,
                current.character_id,
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
                    return Err("House gift actual durable class facts".into());
                }
            };
        let runtime = owner.runtime.lock().await;
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
    if case == Case::GiftDragonDeathHigh {
        super::gift_death::prepare_high_level(h, &owner, &deps, native).await?;
    }
    let historical_owner = if case == Case::GiftDragonDeathHigh {
        id(131)
    } else {
        id(101)
    };
    let nonpickupable_owner_case = case == Case::Placed && before_key == "oteryn:item.tibia.i2976";
    let house_owner = if nonpickupable_owner_case {
        id(41)
    } else {
        historical_owner
    };
    // Historical fixture only: a distinct owner Character, then a real guest ACL
    // entry. The production access function and all write guards remain enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($1,'hex')::uuid,r.account_id,r.world_id,r.lifecycle,r.character_revision,r.profile_revision,r.ruleset_revision,r.content_revision,r.starter_template_revision,'Balloon House Owner',r.sex FROM game_character_roots r WHERE r.character_id=encode($2,'hex')::uuid")
        .bind(historical_owner.as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    sqlx::query(include_str!(
        "../../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(house_key)
    .bind(house_owner.as_slice())
    .execute(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    if !nonpickupable_owner_case {
        sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(house_key).bind(id(41).as_slice()).execute(&mut *tx).await?;
    }
    if case == Case::GiftDragonDeathHigh {
        sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)").bind(id(42).as_slice()).bind(house_key).bind(id(101).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let role: String = sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
    )
    .bind(id(42).as_slice())
    .bind(house_key)
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        role,
        if nonpickupable_owner_case {
            "OWNER"
        } else {
            "GUEST"
        }
    );
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(house_key).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, house_key).ok_or("House")?,
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
        Case::GiftDragonDeathHigh => &[4, 6, 7, 11, 12, 14, 15, 18, 49, 86, 288, 350],
        Case::GiftDragon => &[4, 6, 7, 11, 15, 18, 49, 86, 288, 350],
        Case::GiftSantaDoll
        | Case::GiftPanda
        | Case::GiftPandaFar
        | Case::GiftDragonDeath
        | Case::GiftDragonDeathCrash
        | Case::GiftBankBreak
        | Case::GiftBankKeep => &[4, 6, 7, 11, 15, 18, 49, 86, 350],
        Case::Placed => &[4, 6, 11, 15, 18, 49, 86],
        Case::Cupcake => &[4, 6, 11, 15, 18, 49, 86],
        Case::DracoyleWest | Case::DracoyleEast => &[4, 6, 11, 15, 18, 49, 86, 350],
        Case::Cue => &[4, 6, 11, 15, 18, 49, 86, 350],
        Case::GoldenFull | Case::GoldenLocked => &[4, 6, 7, 11, 15, 18, 49, 86, 350, 386],
        Case::NativeMove => &[4, 6, 11, 15, 18, 49, 86],
        Case::Voice => &[4, 6, 7, 11, 15, 18, 49, 86, 288, 350],
        Case::StaticBase => &[4, 6, 11, 15, 18, 49, 86],
        Case::ArtistEasel => &[4, 6, 11, 15, 18, 49, 86, 207, 350, 387, 388],
        Case::AbsoluteRotate | Case::AbsoluteLegacyPlace | Case::AbsoluteHistory => {
            &[4, 6, 11, 15, 18, 49, 86, 388, 350]
        }
        Case::Rotate => &[4, 6, 11, 15, 18, 49, 86, 388],
        Case::Nested => &[4, 6, 12, 14, 15, 49, 86],
        Case::Carried | Case::CarriedCandle | Case::Foreign => &[4, 6, 15, 49, 86],
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), capabilities)
            .ok_or("House USE offers")?;
    if case == Case::Placed {
        assert!(
            admitted.continuity.selected_capabilities.contains(11)
                && admitted.continuity.selected_capabilities.contains(18),
            "placed furniture fixture must negotiate both required map capabilities"
        );
    }
    let contents = if matches!(case, Case::ArtistEasel | Case::StaticBase) {
        vec![]
    } else if case.absolute_source() {
        vec![
            ("oteryn:item.tibia.i9035", 1),
            (
                "oteryn:item.tibia.i677",
                if case == Case::AbsoluteHistory { 2 } else { 1 },
            ),
        ]
    } else if matches!(case, Case::DracoyleWest | Case::DracoyleEast) {
        vec![(before_key, 1), ("oteryn:item.tibia.i677", 2)]
    } else if case == Case::GiftDragonDeathHigh {
        vec![(before_key, 1), ("oteryn:item.tibia.i2914", 1)]
    } else if case == Case::Nested {
        vec![(before_key, 1), (INNER, 1)]
    } else {
        vec![(before_key, if case == Case::Cupcake { 2 } else { 1 })]
    };
    if matches!(
        case,
        Case::Nested
            | Case::Placed
            | Case::AbsoluteRotate
            | Case::AbsoluteLegacyPlace
            | Case::AbsoluteHistory
            | Case::ArtistEasel
            | Case::StaticBase
            | Case::GiftDragonDeathHigh
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
            .bind(if matches!(case,Case::ArtistEasel|Case::StaticBase|Case::GiftDragonDeathHigh) { deps.room.compiled().client_digest() } else { [2_u8;32] }.as_slice()).bind(if matches!(case,Case::ArtistEasel|Case::StaticBase|Case::GiftDragonDeathHigh) { deps.room.frame_binding().digest() } else { [3_u8;32] }.as_slice())
            .execute(&h.pool).await?;
    }
    super::super::item_look::seed_main_backpack(h, &contents).await?;
    if case == Case::GiftDragonDeathHigh {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i2854',definition_revision_ref='definition-r1' WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }

    let mut source_uuid = id(71);
    if case == Case::ArtistEasel {
        source_uuid = super::artist_easel::acquire(h, &owner, &mut admitted, before_ref).await?;
        // Fixture staging between two real source regions. No travel-route claim.
        owner
            .runtime
            .lock()
            .await
            .stage_pinned_player_test_position(
                deps.actor,
                admitted.game_session_id,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
    }

    if case.absolute_source() {
        let (updated, charged) = super::super::seasonal_dracoyle::animate_fixture_source(
            &owner,
            admitted,
            id(71),
            id(72),
        )
        .await?;
        admitted = updated;
        source_uuid = charged;
    }
    let body = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    let before = if case == Case::StaticBase {
        Value::Null
    } else {
        sqlx::query_scalar::<_,Value>("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source_uuid.as_slice()).fetch_one(&h.pool).await
            .map_err(|error|format!("actual acquired source before House setup: {error}"))?
    };
    if !case.gift()
        && !(case == Case::Placed && before_key == "oteryn:item.tibia.i2976")
        && !matches!(
            case,
            Case::Cupcake
                | Case::Cue
                | Case::GoldenFull
                | Case::GoldenLocked
                | Case::Voice
                | Case::NativeMove
                | Case::AbsoluteRotate
                | Case::AbsoluteLegacyPlace
                | Case::AbsoluteHistory
                | Case::ArtistEasel
                | Case::StaticBase
                | Case::DracoyleWest
                | Case::DracoyleEast
        )
    {
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
    if matches!(
        case,
        Case::GiftDragonDeath | Case::GiftDragonDeathCrash | Case::GiftDragonDeathHigh
    ) {
        if case == Case::GiftDragonDeathHigh {
            assert_eq!(
                owner
                    .home_town(
                        admitted.game_session_id,
                        Some((deps.actor, super::gift_death::TOWN))
                    )
                    .await,
                Some(super::gift_death::TOWN)
            );
            assert_eq!(
                owner.home_town(admitted.game_session_id, None).await,
                Some(super::gift_death::TOWN)
            );
        }
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner
                .progression_binding(admitted.game_session_id)
                .is_some(),
            "lethal gift carries a real progression binding into House custody"
        );
    }
    let mut transfer = vec![8, 1, 18, house_key.len() as u8];
    transfer.extend(house_key.as_bytes());
    let (inside, _) = owner
        .transfer_house(
            admitted,
            if matches!(
                case,
                Case::AbsoluteRotate
                    | Case::AbsoluteLegacyPlace
                    | Case::AbsoluteHistory
                    | Case::ArtistEasel
            ) {
                2
            } else if case.gift()
                || (case == Case::Placed && before_key == "oteryn:item.tibia.i2976")
                || matches!(
                    case,
                    Case::Cupcake
                        | Case::Cue
                        | Case::GoldenFull
                        | Case::GoldenLocked
                        | Case::Voice
                        | Case::NativeMove
                        | Case::StaticBase
                        | Case::DracoyleWest
                        | Case::DracoyleEast
                )
            {
                1
            } else {
                2
            },
            &transfer,
        )
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
    let gift_position = if case == Case::GiftPandaFar {
        [32313, 32804, 7]
    } else {
        fixture_position
    };
    for at in [gift_position, actor_position] {
        let position: Vec<u8> = at.iter().flat_map(|v| v.to_be_bytes()).collect();
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(house_key).bind(position).execute(&mut *tx).await?;
    }
    if (case.placed() && !case.absolute_source()) || case == Case::Nested {
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(source_uuid.as_slice())
        .execute(&mut *tx)
        .await?;
    }
    if case.placed() && !case.absolute_source() {
        let position: Vec<u8> = gift_position.iter().flat_map(|v| v.to_be_bytes()).collect();
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
            .bind(source_uuid.as_slice()).bind(id(42).as_slice()).bind(house_key).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
            .bind(source_uuid.as_slice()).bind(id(42).as_slice()).bind(house_key).bind(house_owner.as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    } else if case == Case::Nested {
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
            .bind(source_uuid.as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
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
                rotate: case == Case::Rotate,
                current: Some(before_ref),
                after: None,
                nested: None,
                capture: None,
                foreign_after_snapshot: Some(h),
            },
        )
        .await?;
        assert_eq!(refused, UseDisposition::StaleState);
        assert_eq!(deltas, 0);
        assert_eq!(item(h).await?, before);
        assert_eq!(receipts(h).await?, 0);
        let other: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source_uuid.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(other, id(101));
        eprintln!(
            "W19 POSTGRES_CASE_COMPLETED house_balloon outside_house_and_changed_character_custody_refuse"
        );
        return Ok(());
    }
    if case == Case::StaticBase {
        return super::static_base::exercise(h, &owner, admitted, before_ref, after_ref).await;
    }
    if case == Case::ArtistEasel {
        return super::artist_easel::exercise(
            h,
            &owner,
            admitted,
            source_uuid,
            before_ref,
            after_ref,
        )
        .await;
    }
    if case == Case::CarriedCandle {
        return super::carried_candle::exercise(h, &owner, admitted, before_ref, after_ref).await;
    }
    if case == Case::AbsoluteLegacyPlace {
        return super::legacy_clock_place::exercise(h, &owner, admitted, source_uuid).await;
    }
    if case.absolute_source() {
        return super::absolute_rotate::exercise(
            h,
            &owner,
            admitted,
            source_uuid,
            before_ref,
            after_ref,
            case == Case::AbsoluteHistory,
        )
        .await;
    }
    if matches!(case, Case::DracoyleWest | Case::DracoyleEast) {
        return super::dracoyle::exercise(h, &owner, admitted, before_ref, after_key).await;
    }
    if case.gift() {
        return super::gifts::exercise(h, &owner, &deps, admitted, before_ref, case).await;
    }
    if case == Case::Placed && before_key == "oteryn:item.tibia.i2976" {
        return super::native_move::refuse_nonpickupable_take(
            h,
            &owner,
            admitted,
            source_uuid,
            before_ref,
        )
        .await;
    }
    if case == Case::NativeMove {
        return super::native_move::exercise(h, &owner, admitted, source_uuid, before_ref).await;
    }
    if case == Case::Voice {
        return super::decoration_voices::exercise(
            h, &owner, admitted, before_ref, after_ref, before_key,
        )
        .await;
    }
    if matches!(case, Case::GoldenFull | Case::GoldenLocked) {
        return super::golden_display::exercise(
            h,
            &owner,
            admitted,
            source_uuid,
            before_ref,
            after_ref,
            case == Case::GoldenFull,
        )
        .await;
    }
    if case == Case::Cue {
        return super::cues::exercise(h, &owner, admitted, before_ref, before_key).await;
    }
    if case == Case::Cupcake {
        let item_id = before_key
            .strip_prefix("oteryn:item.tibia.i")
            .ok_or("Cupcake key")?
            .parse()?;
        return super::cupcakes::exercise(h, &owner, admitted, before_ref, item_id).await;
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
            rotate: case == Case::Rotate,
            current: Some(before_ref),
            after: Some(after_ref),
            nested,
            capture: None,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(delta_count, 1);
    let mut after = before.clone();
    after["definition_production_key"] = after_key.into();
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
    let at = if case.placed() {
        fixture_position
    } else {
        actor_position
    };
    let mut expected = oteryn_gameplay_items::novelty_items::plan_ground(
        native,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: before_key.into(),
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
    .or_else(|| {
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: before_key.into(),
            revision_ref: "definition-r1".into(),
        };
        if case == Case::Rotate {
            oteryn_gameplay_items::furniture::plan_rotate(native, &definition)
        } else {
            oteryn_gameplay_items::furniture::plan_house(native, &definition)
        }
    })
    .ok_or("source House item plan")?;
    if before_key == "oteryn:item.tibia.i2662" {
        expected.effects["house_temporal"] = serde_json::json!({"before":null,"duration":6000,
            "target":"oteryn:item.tibia.i2662"});
    }
    expected.effects["house_after"] =
        serde_json::json!({"key": after_key, "revision": "definition-r1"});
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
    owner.movement_cells = &cells;
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
            rotate: case == Case::Rotate,
            current: None,
            after: None,
            nested,
            capture: None,
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
    if before_key == "oteryn:item.tibia.i2662" {
        let deadline:i64=sqlx::query_scalar("SELECT deadline_at FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source_uuid.as_slice()).fetch_one(&h.pool).await?;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                interval.tick().await;
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                )
                .fetch_one(&h.pool)
                .await?;
                if now >= deadline {
                    break;
                }
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await??;
        owner
            .expire_house_furniture_on_tick(
                admitted.runtime_actor.ok_or("clock House actor")?,
                admitted.game_session_id,
            )
            .await
            .ok_or("clock House tick")?;
        assert_eq!(item(h).await?["definition_production_key"], before_key);
        assert_eq!(item(h).await?["state_revision"], 3);
        assert_eq!(custody(h).await?, original_custody);
        assert_eq!(receipts(h).await?, 1, "expiry is not a second player USE");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_house_decoration_decay_receipts"
            )
            .fetch_one(&h.pool)
            .await?,
            1
        );
        let next_deadline:i64=sqlx::query_scalar("SELECT deadline_at FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(source_uuid.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            next_deadline,
            deadline + 54_000,
            "source clock cycle retains original elapsed time"
        );
        return Ok(());
    }
    let reverse = if nested.is_some() { 4 } else { 2 };
    let (_, result, _, deltas) = exchange(
        &owner,
        admitted,
        reverse,
        &body,
        HouseExchange {
            rotate: case == Case::Rotate,
            current: Some(after_ref),
            after: Some(before_ref),
            nested,
            capture: None,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(deltas, 1);
    assert_eq!(item(h).await?["definition_production_key"], before_key);
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

async fn item(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}
async fn custody(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('container',(SELECT to_jsonb(e) FROM game_item_container_entries e WHERE item_instance_id=encode($1,'hex')::uuid),'nested',(SELECT to_jsonb(n) FROM game_item_nested_entries n WHERE item_instance_id=encode($1,'hex')::uuid),'equipment',(SELECT to_jsonb(s) FROM game_character_equipment_slots s WHERE item_instance_id=encode($1,'hex')::uuid),'house',(SELECT to_jsonb(h) FROM game_item_house_interior_locations h WHERE item_instance_id=encode($1,'hex')::uuid),'provenance',(SELECT to_jsonb(p) FROM game_item_house_reclaim_provenance p WHERE item_instance_id=encode($1,'hex')::uuid))")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?)
}
async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records")
            .fetch_one(&h.pool)
            .await?,
    )
}
