//! Production Item USE selects the same shared native owner and scope barrier.
use super::*;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterLease, MovementLocalPosition,
};
use crate::gameplay_transport::SeamRooms;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, UseCommand};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

#[test]
fn soulpit_native_item_use_entry_return_and_animus_query_postgres() -> TestResult {
    scenario(false, 32)
}

#[test]
fn soulpit_production_owner_spawns_native_wave_and_timeout_returns_without_mastery_postgres()
-> TestResult {
    scenario(true, 32)
}

#[test]
fn soulpit_native_entry_refuses_insufficient_wave_capacity_without_burning_core_postgres()
-> TestResult {
    scenario(false, 11)
}

fn scenario(drive_native: bool, capacity: usize) -> TestResult { scenario_inner(drive_native,capacity,None) }
pub(in super::super) fn soulseal_scenario(mode:super::super::soulseal_fights::Mode)->TestResult {
    scenario_inner(false,if mode==super::super::soulseal_fights::Mode::Capacity {11} else {32},Some(mode))
}
fn scenario_inner(drive_native: bool, capacity: usize, soulseals:Option<super::super::soulseal_fights::Mode>) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("soulpit-native-use", move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_native_use", true, false)
                .await?;
            super::super::reset();
            let evidence = prepare_source(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut deps = Dependencies::new()?;
            let world = deps.admitted.world_id;
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                );
            let active = controller.active().ok_or("active generation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let core = native
                .creature_profiles()
                .records
                .iter()
                .find_map(|record| {
                    let crate::content::ProjectV2AuthoringProfileData::Creature(profile) =
                        &record.profile.data
                    else {
                        return None;
                    };
                    let core = profile.details.as_ref()?.soul_core_item.as_ref()?;
                    let policy = native.item_policy(&core.key, &core.revision)?.record();
                    (core.revision == "definition-r1"
                        && policy.admission.materializable
                        && policy
                            .production_binding
                            .external_id
                            .parse::<u32>()
                            .ok()
                            .is_some_and(crate::domain::soulpit::is_soul_core))
                    .then(|| core.key.clone())
                })
                .ok_or("native core")?;
            let reference = |key: &str| {
                native
                    .item_index()
                    .and_then(|index| index.definition_ref("Item", key, "definition-r1"))
                    .ok_or("item reference")
            };
            let (bytes, pins) = bundle(reference("oteryn:item.tibia.i47367")?.get() - 1)?;
            let base = crate::map::load(&bytes, &pins).map_err(debug)?;
            let map = crate::map::boot::boot(
                &bytes,
                &crate::map::boot::BootPins {
                    map_revision: crate::map::overlay::map_revision(&base),
                    bundle: pins,
                    start: tile(32375, 31158),
                },
                world,
                deps.admitted.channel_id,
                |key| {
                    Some(crate::map::facts::ItemDefinition {
                        reference: reference(key).ok()?,
                        solid: Some(false),
                        blocks_projectile: false,
                        pickupable: false,
                        door: false,
                    })
                },
            )
            .map_err(debug)?;
            let cells = map
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            let room = pin
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    map.frame_binding_digest(world),
                    map.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 32375,
                        y: 31158,
                        floor: 8,
                    },
                )
                .ok_or("bundle Soulpit room")?;
            let template = controller
                .qualify_soulpit_activity_template(&room, "content-1")
                .map_err(debug)?;
            let content_pin = ChannelContentPin::from_activation(
                world,
                1,
                active.identity().server_artifact_digest(),
                active.identity().client_artifact_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (32375, 31158, 8),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                world,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                capacity,
                content_pin,
            )
            .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    deps.admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            deps.admitted.runtime_actor = Some(deps.actor);
            let source = item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(deps.admitted.game_session_id)
                    .await?
                    .0,
            )
            .ok_or("source fence")?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = Some(source);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[4, 6, 11, 15, 18, 72, 117, 168],
            )
            .ok_or("Soulpit capabilities")?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            owner.rooms = SeamRooms::new(None, Some(&room));
            deps.initialize(&owner).await;
            owner
                .register_current_runtime()
                .await
                .map_err(|_| "register source")?;
            owner
                .activity_runtimes
                .remember(admitted)
                .map_err(|_| "remember source")?;
            owner
                .activity_runtimes
                .register_soulpit_template(&template)
                .map_err(|_| "register Soulpit template")?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice())
            .bind(active.identity().client_artifact_digest().as_slice()).bind(map.frame_binding_digest(world).as_slice()).execute(&h.pool).await?;
            super::super::item_look::seed_main_backpack(&h, &[(&core, 2)]).await?;
            super::super::activity_runtime::allocate_ready_instance(&h, 88).await?;
            if let Some(mode)=soulseals {
                super::super::soulseal_fights::exercise(&owner,&h,admitted,mode).await?;
                drop(owner);drop(authority);drop(seal);return h.cleanup().await;
            }
            let target =
                super::super::autowalk::object(map.base().digest(), tile(32375, 31157), 0)?;
            let intent = UseIntent {
                target: UseTarget::ItemByDefinition(reference(&core)?),
                use_with: None,
                use_with_map_item: Some(MapUseTarget::Object(target)),
                use_with_position: None,
            };
            let command = UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1,
                item_fence: Some(source),
            };
            let result = owner
                .dispatch_soulpit_items(deps.actor, command, &intent, None, None)
                .await
                .map_err(|_| "Soulpit USE failed")?
                .ok_or("Soulpit USE not routed")?;
            if capacity < 12 {
                assert_eq!(result.disposition, UseDisposition::Rejected);
                assert_eq!(entry_counts(&h).await?, (2, 0, 0, 0));
                assert_eq!(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session_at(admitted.game_session_id)
                        .await?
                        .0
                        .session_state(),
                    GameSessionState::Active
                );
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            assert_eq!(result.disposition, UseDisposition::Committed);
            assert_eq!(entry_counts(&h).await?, (1, 1, 1, 1));
            let switch = owner
                .activity_runtimes
                .pending_switch(admitted)
                .map_err(|_| "read scope barrier")?
                .ok_or("source scope barrier")?;
            let receipt =
                oteryn_protocol_oteryn::activity_scope::decode_activity_scope(&switch.payload)
                    .map_err(debug)?;
            assert!(matches!(
                receipt.scope,
                oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
            ));
            assert_eq!(
                receipt.source_session,
                admitted.game_session_id.as_bytes().as_slice()
            );
            assert_ne!(switch.destination.runtime_actor, Some(deps.actor));
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(deps.actor, admitted.game_session_id)
                    .is_err(),
                "shared entry must retire the source actor rather than duplicate it"
            );
            let instance = owner
                .activity_runtimes
                .get(
                    switch
                        .destination
                        .item_fence
                        .ok_or("destination fence")?
                        .runtime_scope,
                    1,
                )
                .ok_or("Instance runtime")?;
            let destination = owner.runtime_view(&instance);
            let position = destination
                .runtime
                .lock()
                .await
                .read_actor_position(switch.destination.runtime_actor.ok_or("Instance actor")?)
                .map_err(debug)?
                .position();
            assert_eq!(
                position,
                MovementLocalPosition {
                    x: 32373,
                    y: 31151,
                    floor: 8
                }
            );
            {
                let runtime = destination.runtime.lock().await;
                let states = destination.states.spell_states.lock().await;
                assert!(
                    states
                        .get(
                            &runtime,
                            switch.destination.runtime_actor.ok_or("Instance actor")?,
                            switch.destination.game_session_id
                        )
                        .is_some(),
                    "the shared owner must transfer the actual live player spell state"
                );
            }
            // Drive the durable server lifecycle after the real Item USE entry.
            // Physical wave orchestration is qualified independently by runtime tests.
            let fence = switch
                .destination
                .item_fence
                .ok_or("current Instance item fence")?;
            let initial = h
                .root
                .read_soulpit_encounter(
                    &authority,
                    &h.node,
                    receipt.handoff,
                    fence.runtime_scope,
                    fence.scope_ownership_generation,
                )
                .await
                .map_err(debug)?
                .ok_or("native entry encounter")?;
            if drive_native {
                owner
                    .activity_runtimes
                    .finish_switch(admitted, switch.destination)
                    .map_err(|_| "deliver entry barrier")?;
                let started = initial.encounter.snapshot().started_at_micros;
                let shared = crate::gameplay_transport::BossActivityHandoff;
                shared
                    .drive_soulpit(&owner, started + 4_000_000, false)
                    .await
                    .map_err(|_| "production Soulpit spawn drive")?;
                let stored = h
                    .root
                    .read_soulpit_encounter(
                        &authority,
                        &h.node,
                        receipt.handoff,
                        fence.runtime_scope,
                        fence.scope_ownership_generation,
                    )
                    .await
                    .map_err(debug)?
                    .ok_or("driven encounter")?;
                assert_eq!(
                    stored.encounter.snapshot().phase,
                    crate::domain::soulpit::EncounterPhase::Fighting
                );
                // Crystal soulpit.lua:31-84 makes seven random spawn attempts.
                // The carrier preserves failed attempts on occupied/duplicate cells;
                // only acknowledged identities become live monsters.
                let attempted:serde_json::Value=sqlx::query_scalar("SELECT intent->'action'->'Spawned'->'monster_ids' FROM game_soulpit_events WHERE handoff_id=encode($1,'hex')::uuid AND (intent->'action') ? 'Spawned' ORDER BY revision LIMIT 1")
                    .bind(receipt.handoff.as_slice()).fetch_one(&h.pool).await?;
                let attempted: Vec<Option<u64>> = serde_json::from_value(attempted)?;
                assert_eq!(
                    attempted.len(),
                    7,
                    "every source wave must record all seven spawn attempt slots"
                );
                let mut expected_ids: Vec<_> = attempted.into_iter().flatten().collect();
                expected_ids.sort_unstable();
                assert!(expected_ids.len() <= 7);
                assert_eq!(
                    stored.encounter.snapshot().monsters_alive,
                    expected_ids.len()
                );
                let actors = {
                    let mut runtime = destination.runtime.lock().await;
                    let kernel = runtime.borrow_soulpit_kernel().map_err(debug)?;
                    kernel
                        .carrier
                        .soulpit_actors(kernel.continuity, receipt.handoff)
                        .map_err(debug)?
                };
                let mut identities: Vec<_> = actors
                    .iter()
                    .map(|actor| (actor.tag.monster_id, actor.actor))
                    .collect();
                identities.sort_by_key(|(id, _)| *id);
                assert_eq!(
                    identities.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                    expected_ids,
                    "physical actors must exactly match successful durable spawn slots"
                );
                {
                    let states = destination.states.spell_states.lock().await;
                    for actor in &actors {
                        assert!(
                            states
                                .monster_melee
                                .creature_ai
                                .state(actor.actor)
                                .is_some(),
                            "spawned Soulpit creature needs its real native AI owner"
                        );
                    }
                }
                shared
                    .drive_soulpit(&owner, started + 4_000_000, false)
                    .await
                    .map_err(|_| "production Soulpit repeated deadline")?;
                let repeated = {
                    let mut runtime = destination.runtime.lock().await;
                    let kernel = runtime.borrow_soulpit_kernel().map_err(debug)?;
                    kernel
                        .carrier
                        .soulpit_actors(kernel.continuity, receipt.handoff)
                        .map_err(debug)?
                };
                let mut repeated: Vec<_> = repeated
                    .into_iter()
                    .map(|actor| (actor.tag.monster_id, actor.actor))
                    .collect();
                repeated.sort_by_key(|(id, _)| *id);
                assert_eq!(
                    repeated, identities,
                    "re-driving the same deadline must not duplicate a physical wave"
                );
                let source_map = owner.bundle_map().ok_or("Channel map")?;
                let instance_map = destination.bundle_map().ok_or("Instance map")?;
                assert!(
                    !std::ptr::eq(source_map, instance_map),
                    "Instance must own a private mutable map"
                );
                assert!(!std::ptr::eq(&source_map.overlay, &instance_map.overlay));
                assert!(!std::ptr::eq(&source_map.facts, &instance_map.facts));
                shared
                    .drive_soulpit(
                        &owner,
                        started + crate::domain::soulpit::TIME_LIMIT_MICROS,
                        false,
                    )
                    .await
                    .map_err(|_| "production Soulpit timeout drive")?;
                let back = owner
                    .activity_runtimes
                    .pending_switch(switch.destination)
                    .map_err(|_| "timeout return barrier")?
                    .ok_or("timeout Channel successor")?;
                assert_eq!(
                    back.destination
                        .item_fence
                        .ok_or("timeout return fence")?
                        .runtime_scope,
                    source.runtime_scope
                );
                let (masteries,rewards):(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_character_animus_masteries),(SELECT count(*) FROM game_soulpit_survival_rewards)").fetch_one(&h.pool).await?;
                assert_eq!((masteries, rewards), (0, 0));
                drop(destination);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let survivor = CurrentCharacterGameplayFence {
                character_id: fence.character_id,
                game_session_id: fence.game_session_id,
                connection_generation: fence.connection_generation,
                character_lease_generation: fence.character_lease_generation,
                runtime_scope: fence.runtime_scope,
                scope_ownership_generation: fence.scope_ownership_generation,
                expected_character_revision: CharacterRevision::new(revision.parse()?)
                    .map_err(debug)?,
            };
            use crate::durability::soulpit::{SoulpitAction, SoulpitStepRequest};
            let mut events = Vec::new();
            for wave in 1_u8..=4 {
                let start = initial.encounter.snapshot().started_at_micros
                    + u64::from(wave - 1) * 5_000_000;
                let monster_ids =
                    [1_u64, 2, 3, 4, 5, 6, 7].map(|slot| Some(u64::from(wave) * 100 + slot));
                events.push((SoulpitAction::Advance, start + 4_000_000));
                events.push((
                    SoulpitAction::Spawned { wave, monster_ids },
                    start + 4_000_000,
                ));
                for monster_id in monster_ids.into_iter().flatten() {
                    events.push((
                        SoulpitAction::Defeated { wave, monster_id },
                        start + 4_500_000,
                    ));
                }
                events.push((SoulpitAction::Advance, start + 5_000_000));
            }
            for (index, (action, now_micros)) in events.into_iter().enumerate() {
                h.root
                    .advance_soulpit(
                        &authority,
                        &h.node,
                        SoulpitStepRequest {
                            handoff_id: receipt.handoff,
                            occurrence: id(150 + u8::try_from(index)?),
                            runtime_scope: fence.runtime_scope,
                            scope_ownership_generation: fence.scope_ownership_generation,
                            expected_revision: u64::try_from(index)?,
                            now_micros,
                            action,
                            survivors: vec![survivor],
                        },
                    )
                    .await
                    .map_err(debug)?;
            }
            let body = destination
                .animus_mastery_page(
                    switch.destination,
                    oteryn_protocol_oteryn::animus_mastery::AnimusPage {
                        offset: 0,
                        limit: 50,
                    },
                )
                .await
                .ok_or("earned mastery native query")?;
            let masteries = oteryn_protocol_oteryn::animus_mastery::decode_page(&body)
                .ok_or("earned mastery decode")?;
            assert_eq!(masteries.total, 1);
            assert_eq!(masteries.keys, vec![initial.species]);
            owner
                .activity_runtimes
                .finish_switch(admitted, switch.destination)
                .map_err(|_| "deliver entry barrier")?;
            let returned = destination
                .return_due_activity(
                    switch.destination,
                    crate::durability::activity_handoff::ActivityReturnReason::SoulpitTerminal,
                )
                .await
                .map_err(|_| "native terminal return")?;
            let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(
                commit,
            ) = returned
            else {
                return Err(format!("native terminal return refused: {returned:?}").into());
            };
            destination
                .publish_activity_commit(&commit)
                .map_err(|_| "publish return barrier")?;
            let back = owner
                .activity_runtimes
                .pending_switch(switch.destination)
                .map_err(|_| "read return barrier")?
                .ok_or("return scope barrier")?;
            let returned_scope =
                oteryn_protocol_oteryn::activity_scope::decode_activity_scope(&back.payload)
                    .map_err(debug)?;
            assert!(matches!(
                returned_scope.scope,
                oteryn_protocol_oteryn::activity_scope::ActivityScope::Channel { .. }
            ));
            assert_eq!(
                back.destination
                    .item_fence
                    .ok_or("returned item fence")?
                    .runtime_scope,
                source.runtime_scope
            );
            assert_ne!(
                back.destination.game_session_id,
                switch.destination.game_session_id
            );
            let back_actor = back
                .destination
                .runtime_actor
                .ok_or("returned Channel actor")?;
            let back_position = owner
                .runtime
                .lock()
                .await
                .read_actor_position(back_actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                back_position,
                MovementLocalPosition {
                    x: 32373,
                    y: 31158,
                    floor: 8
                }
            );
            let body = owner
                .animus_mastery_page(
                    back.destination,
                    oteryn_protocol_oteryn::animus_mastery::AnimusPage {
                        offset: 0,
                        limit: 50,
                    },
                )
                .await
                .ok_or("returned native mastery query")?;
            assert_eq!(
                oteryn_protocol_oteryn::animus_mastery::decode_page(&body)
                    .ok_or("returned mastery decode")?,
                masteries
            );
            drop(destination);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

fn tile(x: u16, y: u16) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos { x, y, floor: -8 }
}

fn bundle(obelisk: u32) -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{self, Extent, Family, PaletteEntry, Sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        ("oteryn:item.tibia.i47367", obelisk),
        ("oteryn:item.tibia.i3031", 0),
    );
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    bundle.manifest.palette.truncate(2);
    bundle.manifest.palette[1] = PaletteEntry {
        key: "oteryn:item.tibia.i47367".into(),
        family: Family::Item,
        id: obelisk,
        terrain: None,
    };
    bundle.manifest.world = Extent {
        min_x: 32362,
        min_y: 31132,
        max_x: 32391,
        max_y: 31163,
        floors: vec![-8],
    };
    let mut sectors = std::collections::BTreeMap::<(u16, u16), Sector>::new();
    for y in 31132..=31162 {
        for x in 32362..=32390 {
            let key = (x / 32, y / 32);
            let sector = sectors.entry(key).or_insert_with(|| Sector {
                floor: -8,
                sx: key.0,
                sy: key.1,
                tiles: Vec::new(),
            });
            let mut items = vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            }];
            if (x, y) == (32375, 31157) {
                items.push(Item {
                    palette: 1,
                    depth: 0,
                    attrs: Attrs::default(),
                });
            }
            sector.tiles.push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
        }
    }
    let mut sectors: Vec<_> = sectors.into_values().collect();
    sectors.sort_by_key(|s| (s.floor, s.sy, s.sx));
    let bytes = writer::write(&bundle.manifest, &sectors, &bundle.spawns).map_err(debug)?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((bytes, pins))
}
