//! World robbery projection, native lethal receipts, corpse custody and committed save.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::QualifiedNativeEntryRoom;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::world_event::bank_robbery::{self, City, Projection};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    NodeId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use crate::gameplay_transport::{ComposedFreshAdmission, activity_runtime::SharedOwner};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::carrier::RaidSpawnOutcome;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_world::bank_robbery as projection;

use bank_robbery::BAG;

#[path = "bankrob/returns.rs"]
mod returns;

#[test]
fn bankrob_city_spawn_and_top_damage_loot() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        for city in bank_robbery::cities()? {
            let started = std::time::Instant::now();
            let h =
                Harness::create_with_admission_seed(admin.clone(), "bankrob_city", false, false)
                    .await?;
            eprintln!("bankrob-marker: bankrob_city_spawn_and_top_damage_loot db ready");
            let result = scenario(&h, city, false, None).await;
            h.cleanup().await?;
            eprintln!(
                "bankrob_city_spawn_and_top_damage_loot city={} elapsed={:?}",
                city.key,
                started.elapsed()
            );
            result?;
        }
        Ok(())
    })
}

#[test]
fn bankrob_server_save_expires_bags_and_reopens() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async |admin| {
        use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "bankrob_save", false, false).await?;
        eprintln!("bankrob-marker: bankrob_server_save_expires_bags_and_reopens db ready");
        let schedule = Schedule::parse(r#"{"events":[{"name":"double","startdate":"01/01/2030","enddate":"01/02/2030","ingame":{"lootrate":200}}]}"#).map_err(debug)?;
        let _clock = test_clock::set(schedule, 1_893_456_000);
        let result = scenario(&h, &bank_robbery::cities()?[0], true, None).await;
        h.cleanup().await?;
        eprintln!(
            "bankrob_server_save_expires_bags_and_reopens elapsed={:?}",
            started.elapsed()
        );
        result
    })
}

async fn scenario(
    h: &Harness,
    city: &'static City,
    save: bool,
    hand_in: Option<returns::Case>,
) -> TestResult {
    let mut deps = Dependencies::new()?;
    let scope = RuntimeScopeRefV1::channel(deps.admitted.world_id, deps.admitted.channel_id);
    let world = scope.world_id();
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let reset_source = (save || hand_in == Some(returns::Case::Second)).then_some(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )
        .as_slice(),
    );
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_source(
            world,
            "content-1",
            reset_source,
        )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let mut origin = at(city.cell);
    origin.x -= 2;
    let scene = scene(scope, city, origin, hand_in, native)?;
    let cells = scene
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = super::rust_lottery_fixtures::bundle_action_room(
        activated.qualified_room(),
        &scene,
        world,
        &cells,
    )?;
    let digest = active.identity().server_artifact_digest();
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        digest,
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (origin.x, origin.y, origin.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        64,
        pin.clone(),
    )
    .map_err(debug)?;
    runtime.use_classic_creature_deaths().map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, origin)
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    deps.combat_room = Some(room);
    deps.runtime = tokio::sync::Mutex::new(runtime);

    // The reset uses the real activated native entry content (the combat view
    // above supplies only the geometry at the city cells for this fixture).
    let reset_room = activated.qualified_room();
    if save {
        assert!(reset_room.movement_cells().house_tiles().is_known_empty());
    }
    record_epoch(h, scope, reset_room).await?;
    use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session_request};
    let platform_source = if hand_in == Some(returns::Case::Synchronized) {
        Some(RecoverySource::start(h).await?)
    } else {
        None
    };
    let fresh = if platform_source.is_some() {
        prepare_owned_session_request(h).await?
    } else {
        prepare_current_session(h).await?
    };
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let sites = projection::loaded(&cells)?;
    assert!(
        h.root
            .poll_bank_robbery(&authority, &h.node, scope, generation, 0, sites)
            .await?
            .is_none()
    );
    let session = deps.admitted.game_session_id;
    let bank = if hand_in.is_some() {
        Some(returns::Bank::new(scope, pin, session, origin)?)
    } else {
        None
    };
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "Character item fence")?
        .ok_or("current Character fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                deps.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            digest,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let starter = crate::durability::starter_grant::StarterTemplate {
        definition: definition("oteryn:item.tibia.i2854"),
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native);
    h.root
        .grant_starter_items(&authority, &h.node, &fence.item_fence(), &weights, &starter)
        .await
        .map_err(debug)?;
    // Exercise loot-only settlement. The second live actor supplies the final
    // hit, but this fixture does not create a second durable progression root.
    deps.initialize(&owner).await;

    if let Some(bank) = &bank {
        owner.runtime = SharedOwner::Borrowed(&bank.runtime);
        bank.install(&owner, session)?;
        if hand_in == Some(returns::Case::Announcements) {
            bank.before_robbery(&owner, fence.item_fence()).await?;
        }
        owner.runtime = SharedOwner::Borrowed(&deps.runtime);
    }
    sqlx::query(
        "UPDATE game_bank_robbery_occurrences SET boot_ms=boot_ms-3600000 WHERE reset_epoch=0",
    )
    .execute(&h.pool)
    .await?;
    let selected = bank_robbery::with_test_roll(
        city.key.as_str(),
        h.root
            .poll_bank_robbery(&authority, &h.node, scope, generation, 0, sites),
    )
    .await?
    .ok_or("city occurrence")?;
    assert_eq!(selected.city.key, city.key);
    let target = {
        let mut runtime = deps.runtime.lock().await;
        projection::project(&mut runtime, &cells, selected).map_err(debug)?;
        projection::project(&mut runtime, &cells, selected).map_err(debug)?;
        assert_cohort(&runtime, selected)?
    };

    kill_with_top_damage(&owner, deps.actor, session, target, origin).await?;
    let mints: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(b.item_instance_id) FROM game_bank_robbery_bags b
         JOIN game_bank_robbery_occurrences o USING (world_id,reset_epoch)
         JOIN game_item_instances i USING (item_instance_id)
         JOIN game_item_corpse_container_entries e USING (item_instance_id)
         JOIN game_item_mint_receipts r ON r.item_instance_id=e.parent_item_instance_id
         WHERE o.firing_id=encode($1,'hex')::uuid AND o.state=2
           AND i.definition_production_key=$2 AND i.quantity=1
           AND r.corpse_top_damage_character_id=encode($3,'hex')::uuid ORDER BY b.ordinal LIMIT 3",
    )
    .bind(selected.firing_id.as_slice())
    .bind(BAG)
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_all(&h.pool)
    .await?;
    let double = save || hand_in.is_some_and(returns::Case::double_loot);
    assert_eq!(mints.len(), if double { 2 } else { 1 });
    if double {
        assert_ne!(mints[0], mints[1]);
    }
    if save || hand_in.is_some_and(|case| case != returns::Case::Announcements) {
        // A real pickup needs the holder's durable progression level to derive
        // carried capacity. Bind it after the loot-only fight has settled.
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());
        let item = mints[0].clone().try_into().map_err(|_| "bag UUID")?;
        take_bag(
            h,
            &authority,
            fence.item_fence(),
            CommandRef::new(session, CommandId::new(4).map_err(debug)?),
            native,
            &starter.definition,
            item,
        )
        .await
        .map_err(|error| format!("pick up the robbery bag: {error}"))?;
        if hand_in.is_some_and(returns::Case::double_loot) {
            take_bag(
                h,
                &authority,
                fence.item_fence(),
                CommandRef::new(session, CommandId::new(5).map_err(debug)?),
                native,
                &starter.definition,
                mints[1].as_slice().try_into().map_err(|_| "bag UUID")?,
            )
            .await?;
        }
        if save {
            save_world(h, &authority, scope, reset_room, selected, item)
                .await
                .map_err(|error| format!("commit the robbery World save: {error}"))?;
        }
    }
    if let (Some(case), Some(bank)) = (hand_in, bank.as_ref()) {
        owner.runtime = SharedOwner::Borrowed(&bank.runtime);
        let first_bag = mints[0].as_slice().try_into().map_err(|_| "bag UUID")?;
        let second_bag = mints
            .get(1)
            .map(|id| id.as_slice().try_into())
            .transpose()
            .map_err(|_| "bag UUID")?;
        bank.exercise(
            case,
            &mut owner,
            fence.item_fence(),
            h,
            first_bag,
            second_bag,
        )
        .await?;
        if case == returns::Case::Second {
            save_world(
                h,
                &authority,
                scope,
                reset_room,
                selected,
                second_bag.ok_or("second bag")?,
            )
            .await?;
        }
    }
    Ok(())
}

fn definition(key: &str) -> crate::durability::item_mint::TypedDefinitionRef {
    crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn at([x, y, z]: [u16; 3]) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(x),
        y: i32::from(y),
        floor: z as i16,
    }
}

fn scene(
    scope: RuntimeScopeRefV1,
    city: &City,
    origin: MovementLocalPosition,
    hand_in: Option<returns::Case>,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    // Trade checks occupied endpoints through the qualified spell tile reader.
    // Use the native grass identity so both movement and sight can read the ground.
    manifest.palette[0].key = "oteryn:terrain.tibia.i4526".into();
    let board = if hand_in == Some(returns::Case::Announcements) {
        let key = returns::BOARD;
        let facts =
            crate::map::boot::native_item_definition(Some(native), key).ok_or("Board Item")?;
        let palette = u32::try_from(manifest.palette.len())?;
        manifest.palette.push(bundle::PaletteEntry {
            key: key.into(),
            family: bundle::Family::Item,
            id: facts.reference.get() - 1,
            terrain: None,
        });
        Some(palette)
    } else {
        None
    };
    let floor = i8::try_from(-origin.floor)?;
    let x = u16::try_from(origin.x)?;
    let y = u16::try_from(origin.y)?;
    manifest.world = bundle::Extent {
        min_x: x - 1,
        max_x: city.cell[0] + 3,
        min_y: y - 2,
        max_y: y + 3,
        floors: vec![floor],
    };
    let mut by_sector = std::collections::BTreeMap::<_, Vec<_>>::new();
    for ty in manifest.world.min_y..manifest.world.max_y {
        for tx in manifest.world.min_x..manifest.world.max_x {
            by_sector
                .entry((ty / 32, tx / 32))
                .or_default()
                .push(sector::Tile {
                    x: tx,
                    y: ty,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: std::iter::once(0)
                        .chain(board.filter(|_| tx == x && ty == y + 1))
                        .map(|palette| sector::Item {
                            palette,
                            depth: 0,
                            attrs: Default::default(),
                        })
                        .collect(),
                });
        }
    }
    let sectors: Vec<_> = by_sector
        .into_iter()
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let map_revision = crate::map::overlay::map_revision(&crate::map::load(&bytes, &pins)?);
    Ok(crate::map::boot::check(
        bytes,
        crate::map::boot::BootPins {
            map_revision,
            bundle: pins,
            start: crate::map::overlay::TilePos { x, y, floor },
        },
    )?
    .boot(
        scope.world_id(),
        scope.channel_id().ok_or("channel")?,
        |key| {
            crate::map::boot::tests::items(key)
                .or_else(|| crate::map::boot::native_item_definition(Some(native), key))
        },
    )?)
}

fn assert_cohort(runtime: &ChannelRuntimeV1, selected: Projection) -> TestResult<ExactActorRef> {
    let city = selected.city;
    let specs = std::iter::once((city.robber.as_str(), city.cell)).chain(
        city.escort.iter().flat_map(|escort| {
            escort
                .cells
                .iter()
                .copied()
                .map(|cell| (escort.creature.as_str(), cell))
        }),
    );
    let bodies = runtime.raid_creatures(selected.firing_id).map_err(debug)?;
    assert_eq!(bodies.len(), if city.escort.is_some() { 5 } else { 1 });
    let mut robber = None;
    for (status, (key, cell)) in bodies.into_iter().zip(specs) {
        let RaidSpawnOutcome::Placed(actor) = status.outcome else {
            return Err("unplaced robber cohort".into());
        };
        assert!(status.alive);
        let body = runtime.companion_snapshot(actor).map_err(debug)?;
        assert_eq!(body.position, at(cell));
        assert_eq!(body.state.policy.definition_key, key);
        if status.boss {
            robber = Some(actor);
        }
    }
    robber.ok_or_else(|| "robber missing from projected cohort".into())
}

async fn kill_with_top_damage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    winner: ExactActorRef,
    session: GameSessionId,
    target: ExactActorRef,
    origin: MovementLocalPosition,
) -> TestResult {
    use crate::foundation::{CharacterId, CharacterLease, OwnerDamageCommand};
    use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
    assert!(owner.refresh_source_party_presence(winner, session).await);
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native generation")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = owner.runtime.lock().await;
        let final_session = GameSessionId::decode(&id(87)).map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(final_session)
            .map_err(debug)?;
        let final_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                final_actor,
                MovementLocalPosition {
                    y: origin.y + 1,
                    ..origin
                },
            )
            .map_err(debug)?;
        let final_character = CharacterId::decode(&id(88)).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                final_actor,
                final_session,
                CharacterLease::new(final_character, 1).map_err(debug)?,
            )
            .map_err(debug)?;
        let body = runtime.companion_snapshot(target).map_err(debug)?;
        let key = &body.state.policy.definition_key;
        for (actor, who, damage) in [
            (winner, session, body.health - 1),
            (final_actor, final_session, 1),
        ] {
            runtime
                .borrow_exact_actor_commit_at(now)
                .commit_damage_for_bound_attacker(
                    target,
                    actor,
                    CommandRef::new(who, CommandId::new(2).map_err(debug)?),
                    0,
                    OwnerDamageCommand {
                        target: key.as_bytes(),
                        occurrence: who.as_bytes(),
                        binding: b"bank-robbery-kill-test",
                        damage,
                    },
                )
                .map_err(debug)?;
        }
        record_creature_kill(
            &mut runtime,
            KillSink {
                queue: &queue,
                rewards: native.reward_table(),
                activity: &[],
                prey: None,
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: final_character,
                lease_generation: 1,
                session: final_session,
                actor: final_actor,
            },
            now,
            Some(observed),
        );
    }
    for _ in 0..8 {
        if owner.drain_kill_rewards(session).await == DrainOutcome::Drained {
            return Ok(());
        }
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Err("native Bank Robbery loot did not drain".into())
}

async fn record_epoch(
    h: &Harness,
    scope: RuntimeScopeRefV1,
    room: &QualifiedNativeEntryRoom,
) -> TestResult {
    let world = scope.world_id();
    let channel = scope.channel_id().ok_or("channel")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: server,
                    client_artifact_digest: client,
                    frame_binding_digest: frame,
                }
            )
            .await?
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_bytes().as_slice()).bind(server.as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str())
        .bind(client.as_slice()).bind(frame.as_slice()).fetch_one(&h.pool).await?);
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "current-session custody bindings are explicit"
)]
async fn take_bag(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    command: CommandRef,
    native: &crate::content::native_gameplay::NativeGameplayState,
    backpack: &crate::durability::item_mint::TypedDefinitionRef,
    item: [u8; 16],
) -> TestResult {
    use crate::durability::item_transfer::{
        ItemTransferDestination, ItemTransferOutcome, ItemTransferRequest,
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let weights = NativeItemWeights(native);
    let request = ItemTransferRequest {
        weight_generation_digest: native.source_digest(),
        command,
        source_item_instance_id: item,
        destination: ItemTransferDestination::MainBackpack,
        item: generation_item_facts(native, &definition(BAG)).map_err(debug)?,
        backpack: Some(generation_item_facts(native, backpack).map_err(debug)?),
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    };
    let permit =
        crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(fence.runtime_scope)
            .await
            .map_err(debug)?;
    let mut candidate = h
        .root
        .freeze_item_transfer(authority, &h.node, fence, request, &weights)
        .await
        .map_err(debug)?;
    assert!(matches!(
        h.root
            .commit_item_transfer(&permit, authority, &h.node, fence, &mut candidate, &weights)
            .await
            .map_err(debug)?,
        ItemTransferOutcome::Committed(_)
    ));
    Ok(())
}

async fn save_world(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    scope: RuntimeScopeRefV1,
    room: &QualifiedNativeEntryRoom,
    selected: Projection,
    offline_bag: [u8; 16],
) -> TestResult {
    use super::planned_world_reset_fixtures::reset_fixture;
    let world = scope.world_id();
    let channel = scope.channel_id().ok_or("channel")?;
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    // Model a fully drained controller. The reset still requires the native
    // fresh assignment, retirement and epoch-finalization transactions.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3")
        .execute(&h.pool)
        .await?;
    let carried: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(offline_bag.as_slice()).fetch_one(&h.pool).await?;
    assert!(carried);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)"
        )
        .bind(world.as_bytes().as_slice())
        .bind(server.as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str())
        .bind(client.as_slice())
        .bind(frame.as_slice())
        .fetch_one(&h.pool)
        .await?
    );
    let reset = h
        .root
        .retiring_world_reset(world)
        .await?
        .ok_or("retiring save")?;
    reset_fixture::replace(h, world, channel, &h.node).await?;
    assert_eq!(
        h.root
            .finish_world_reset(authority, &h.node, &reset, room, room)
            .await
            .map_err(debug)?,
        Some(1)
    );
    let state: (i16, i64) = sqlx::query_as(
        "SELECT o.state, count(*) FILTER (WHERE i.lifecycle=2 AND i.quantity=0)
         FROM game_bank_robbery_occurrences o JOIN game_bank_robbery_bags b USING(world_id,reset_epoch)
         JOIN game_item_instances i USING(item_instance_id)
         WHERE o.firing_id=encode($1,'hex')::uuid GROUP BY o.state",
    ).bind(selected.firing_id.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (4, 2));
    let held_after_save: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_container_entries
         WHERE item_instance_id=encode($1,'hex')::uuid)",
    )
    .bind(offline_bag.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(!held_after_save);
    reset_fixture::publish_ready(h, 2).await?;
    let generation = ScopeOwnershipGeneration::new(2).map_err(debug)?;
    let sites = [true; 4];
    assert!(
        bank_robbery::with_test_roll(
            "carlin",
            h.root
                .poll_bank_robbery(authority, &h.node, scope, generation, 0, sites)
        )
        .await?
        .is_none()
    );
    assert!(
        bank_robbery::with_test_roll(
            "carlin",
            h.root
                .poll_bank_robbery(authority, &h.node, scope, generation, 1, sites)
        )
        .await?
        .is_none()
    );
    let epochs: Vec<(String, i16)> = sqlx::query_as(
        "SELECT reset_epoch::text, state FROM game_bank_robbery_occurrences WHERE world_id=encode($1,'hex')::uuid ORDER BY reset_epoch LIMIT 3"
    ).bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(epochs, [("0".into(), 4), ("1".into(), 0)]);
    Ok(())
}
