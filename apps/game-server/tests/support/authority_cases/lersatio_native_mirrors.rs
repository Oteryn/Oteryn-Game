//! Native Lersatio mirrors through the composed production USE caller and PostgreSQL.
use super::engnpc2_paint_transport::world as native_world;
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementLocalPosition, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_world::map::{
    boot::{BundleCollisionIndexExt, BundleMap},
    overlay::TilePos,
    view::MapFacts,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;
const COUNTER: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter";
const MIRROR: TilePos = TilePos {
    x: 32965,
    y: 31457,
    floor: -2,
};

#[test]
fn lersatio_native_mirrors_real_use_counter_census_cohort_and_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(128 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create_with_admission_seed(admin, "lersatio_native_mirrors", true, false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("PG_EXECUTED lersatio_native_mirrors_real_use_counter_census_cohort_and_reset duration={:?}", started.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(debug)
    })?.join().map_err(|_| "Lersatio PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    super::engnpc2_live_transport::fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(session)
            .await?;
    let start = (32966, 31457);
    let scene = native_world::native_scene(
        native,
        vec![
            native_world::NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../content/world/placements/region-z02-x128-y122.b3"
                )),
                region: (2, 128, 122),
                bounds: (32963, 31456, 32969, 31460),
                start,
                expected: vec![],
            },
            native_world::NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../content/world/placements/region-z01-x128-y122.b3"
                )),
                region: (1, 128, 122),
                bounds: (32966, 31459, 32969, 31460),
                start: (32967, 31459),
                expected: vec![],
            },
        ],
        &[],
        &[8667, 8721, 8325, 8326],
        world,
        channel,
    )?;
    let cells = scene
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = pin
        .qualified_room()
        .bundle_view(
            cells,
            scene.frame_binding_digest(world),
            scene.map_revision_digest(),
            oteryn_gameplay_content::content::NativeEntryStart {
                x: i32::from(start.0),
                y: i32::from(start.1),
                floor: 2,
            },
        )
        .ok_or("qualified native seal scene")?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let start = room.entry_start();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        24,
        channel_pin,
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(&id(41)).map_err(debug)?,
                current.current_character_lease().generation(),
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let at = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (start.x, start.y, 2));
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    for key in ["oteryn:creature.lersatio", "oteryn:creature.vampire"] {
        runtime.companion_policy(key).map_err(debug)?;
    }
    runtime.use_classic_creature_deaths().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let content =
        oteryn_gameplay_world::world_runtime::ReferenceContentGeneration::from_content(room.door())
            .map_err(debug)?;
    let fence = oteryn_gameplay_world::world_runtime::ScopeContentGenerationFence::for_test(
        scope,
        fixture::fence(1)?.scope_ownership_generation,
        content,
    );
    let door = Mutex::new(
        oteryn_gameplay_world::world_runtime::bind_native_entry_door(
            room.door(),
            &fence,
            scope,
            fixture::fence(1)?.scope_ownership_generation,
        )
        .map_err(debug)?,
    );
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(),
        ("127.0.0.1".into(), 1),
        "source.test".into(),
        "source.test".into(),
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/ca.der").to_vec(),
        )],
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../spell-save-tls-fixture/client.der").to_vec(),
        )],
        tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(
            include_bytes!("../spell-save-tls-fixture/client-key.der").to_vec(),
        )?,
    )
    .map_err(debug)?;
    let evidence = FreshEvidenceSource::new(descriptor);
    let spells = oteryn_gameplay_spell::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        oteryn_gameplay_social::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(debug)?;
    let charms =
        oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue::embedded()
            .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner: ComposedFreshAdmission<'_, '_, '_> = ComposedTestOwner {
        root: &h.root,
        character: &authority,
        holder: &h.node,
        evidence: &evidence,
        world_id: world,
        channel_id: channel,
        required_capabilities: composed_test_owner::empty_capabilities(world, channel)?,
        runtime: &runtime,
        room: &room,
        door: &door,
        spells: &spells,
        achievements: &achievements,
        imported_charms: &charms,
    }
    .build();
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let oteryn_gameplay_content::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(
        index,
    ) = room.movement_cells().index()
    else {
        return Err("native bundle index".into());
    };
    let map = index.map();
    check_data()?;
    let lease_generation = current.current_character_lease().generation();
    {
        let runtime = runtime.lock().await;
        initialize(&owner, &runtime, actor, session)?;
    }
    let (target, key) = object(map, MIRROR, 8667)?;
    let begun = now(&owner);
    assert_eq!(
        use_mirror(&mut owner, actor, session, 1, target).await?,
        UseDisposition::Committed
    );
    let published_by = now(&owner);
    assert_eq!(counter(h).await?, 1);
    let (broken, _) = object(map, MIRROR, 8721)?;
    assert_ne!(
        use_mirror(&mut owner, actor, session, 2, broken).await?,
        UseDisposition::Committed
    );
    assert_eq!(counter(h).await?, 1);
    object(
        map,
        TilePos {
            x: 32967,
            y: 31457,
            floor: -2,
        },
        8326,
    )?;
    let due = map.facts.next_movement_decay_time().ok_or("mirror timer")?;
    assert!((begun + 32_000_000..=published_by + 30_000_000).contains(&due));
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    advance_to(&mut owner, due)?;
    assert_eq!(map.facts.expire_movement_items(now(&owner)), 1);
    assert_eq!(map.facts.object_revision(key), 2);
    object(map, MIRROR, 8667)?;
    let destination = MovementLocalPosition {
        x: 32967,
        y: 31459,
        floor: 1,
    };
    let (monster, departed, departed_before, present) = {
        let mut runtime = runtime.lock().await;
        let source = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let monster = runtime
            .admit_source_pinned_lab_creature(source, "oteryn:creature.vampire", 475)
            .map_err(debug)?;
        let departed_session = GameSessionId::decode(&id(99)).map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(departed_session)
            .map_err(debug)?;
        let departed = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(departed, source)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                departed,
                departed_session,
                CharacterLease::new(
                    CharacterId::decode(&id(119)).map_err(debug)?,
                    lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let expected = runtime.read_actor_position(departed).map_err(debug)?;
        let closed = oteryn_gameplay_world::map::floor::BundleDoors(&map.facts);
        let proof = oteryn_gameplay_world::map::floor::prepare_quest_encounter_ejection(
            &runtime,
            &owner.movement_cells,
            departed,
            departed_session,
            expected,
            destination,
            &closed,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        let departed_before = runtime.read_actor_position(departed).map_err(debug)?;
        let mut present = vec![actor];
        for seed in 100..118 {
            let player_session = GameSessionId::decode(&id(seed)).map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(player_session)
                .map_err(debug)?;
            let player = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(player, source)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    player,
                    player_session,
                    CharacterLease::new(
                        CharacterId::decode(&id(seed + 20)).map_err(debug)?,
                        lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            initialize(&owner, &runtime, player, player_session)?;
            present.push(player);
        }
        (monster, departed, departed_before, present)
    };
    let monster_before = runtime
        .lock()
        .await
        .read_actor_position(monster)
        .map_err(debug)?;
    let (restored, _) = object(map, MIRROR, 8667)?;
    assert_eq!(
        use_mirror(&mut owner, actor, session, 3, restored.clone()).await?,
        UseDisposition::Committed
    );
    assert_eq!(counter(h).await?, 0);
    // The wire owns its cached acknowledgement; the caller must never duplicate its cohort.
    use_mirror(&mut owner, actor, session, 3, restored).await?;
    assert_eq!(counter(h).await?, 0);
    {
        let runtime = runtime.lock().await;
        for player in present {
            assert_eq!(
                runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position(),
                destination
            );
        }
        assert_eq!(
            runtime.read_actor_position(departed).map_err(debug)?,
            departed_before,
            "a player who left for the destination is not moved again"
        );
        assert_eq!(
            runtime.read_actor_position(monster).map_err(debug)?,
            monster_before,
            "source monsters are excluded"
        );
        let census = runtime.positioned_melee_census().map_err(debug)?;
        for (identity, expected) in [
            ("oteryn:creature.lersatio", 1),
            ("oteryn:creature.vampire", 2),
        ] {
            assert_eq!(
                census
                    .iter()
                    .filter(|(who, at, session)| session.is_none()
                        && at.position().floor == 1
                        && runtime.matches_live_creature_identity(*who, identity.as_bytes()))
                    .count(),
                expected
            );
        }
    }
    let current_revision = map.facts.object_revision(key);
    let stale_due = map.facts.next_movement_decay_time().ok_or("second reset")?;
    let broken = map
        .facts
        .quest_callback_replacement(key, 8721)
        .ok_or("broken mirror")?;
    assert!(
        map.facts
            .apply_quest_callback(key, current_revision, broken, &[], &[], false, None)
    );
    advance_to(&mut owner, stale_due)?;
    map.facts.expire_movement_items(now(&owner));
    assert_eq!(map.facts.object_revision(key), current_revision + 1);
    object(map, MIRROR, 8721)?;
    Ok(())
}

fn now(owner: &ComposedFreshAdmission<'_, '_, '_>) -> u64 {
    u64::try_from(owner.clock_origin.elapsed().as_micros()).unwrap_or(u64::MAX)
}
fn advance_to(owner: &mut ComposedFreshAdmission<'_, '_, '_>, due: u64) -> TestResult {
    let remaining = due.checked_sub(now(owner)).ok_or("timer already overdue")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_micros(remaining))
        .ok_or("clock")?;
    Ok(())
}
fn initialize(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
) -> TestResult {
    // The canonical unpromoted level-one maxima; these players do not cast or take damage.
    let facts = oteryn_gameplay_spell::spell::cast::CharacterCastFacts {
        vocation: oteryn_gameplay_spell::spell::Vocation::None,
        level: 1,
        magic_level: 0,
        max_health: 150,
        max_mana: 0,
        max_soul: 100,
    };
    owner
        .states
        .spell_states
        .try_lock()
        .map_err(debug)?
        .initialize(
            runtime,
            actor,
            session,
            facts,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now(owner)),
        )
        .ok_or("player state")?;
    Ok(())
}
async fn use_mirror(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    command: u64,
    target: WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let decoded =
        oteryn_protocol_oteryn::world_object::decode_use_intent_target(&[4, 6, 11, 17], &payload)
            .map_err(debug)?;
    let oteryn_protocol_oteryn::world_object::UseTarget::WorldObject(target) = decoded.target
    else {
        return Err("world-object USE".into());
    };
    let fence = composed_test_owner::load_quests(owner, session).await?;
    Ok(owner
        .use_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            target,
        )
        .await
        .disposition)
}
async fn counter(h: &fixture::Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(COUNTER).fetch_one(&h.pool).await?)
}
fn object(map: &BundleMap, at: TilePos, appearance: u16) -> TestResult<(WorldObjectTarget, u64)> {
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("native tile")?;
    let mut found = Vec::new();
    for (ordinal, (id, _)) in base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
    {
        let ordinal = u8::try_from(ordinal)?;
        let key =
            oteryn_gameplay_world::map::view::placement_key(at, ordinal).ok_or("placement")?;
        if !map.facts.base_hidden(key)
            && map
                .facts
                .base_entry(at, ordinal, *id)
                .is_some_and(|facts| facts.appearance_id == appearance)
        {
            found.push(key);
        }
    }
    if found.len() != 1 {
        return Err(format!(
            "native object appearance={appearance} at={at:?} candidates={found:?}"
        )
        .into());
    }
    let key = found[0];
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok((
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        },
        key,
    ))
}
fn check_data() -> TestResult {
    use oteryn_gameplay_quests::quest::{source_world_objects, world_callbacks};
    let callbacks = world_callbacks::load().map_err(debug)?;
    let mirrors: Vec<_> = callbacks
        .into_iter()
        .filter(|c| {
            c.key
                .starts_with("oteryn:quest-callback/blood-brothers-lersatio-mirror-")
        })
        .collect();
    assert_eq!(mirrors.len(), 6);
    for callback in &mirrors {
        assert!(callback.has_valid_effects());
        assert!(source_world_objects::source_is_ready(callback, 8667));
        assert!(!source_world_objects::source_is_ready(callback, 8721));
        assert_eq!(callback.effects.reset_after_ms, Some(30_000));
    }
    let second = mirrors
        .iter()
        .find(|c| c.effects.encounter.is_some())
        .ok_or("second phase")?;
    let mut invalid = second.clone();
    invalid.effects.item_cycle = Some(serde_json::from_value(
        serde_json::json!({"position":{"x":32965,"y":31457,"floor":-2},"remove":8667,"create":8721}),
    )?);
    assert!(
        !invalid.has_valid_effects(),
        "a third shared operation remains invalid"
    );
    let mut timer_only = second.clone();
    timer_only.effects.scene = None;
    assert!(timer_only.has_valid_effects());
    assert!(!source_world_objects::source_is_ready(&timer_only, 8721));
    timer_only
        .effects
        .encounter
        .as_mut()
        .ok_or("encounter")?
        .spectators = None;
    assert!(
        !timer_only.has_valid_effects(),
        "empty fixed staging remains invalid"
    );
    Ok(())
}
