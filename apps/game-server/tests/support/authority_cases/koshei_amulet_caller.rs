//! KOSHEI: real framed USE on current Ground handles through the composed owner.
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, AdmittedSession, ComposedTestOwner, ConnectionEnd, ControllerBinding, FirstEntryOutcome,
    FreshAdmissionAuthority, IDLE_LIVENESS, SelectedCapabilities, SessionContinuity,
    serve_admitted,
};
use oteryn_gameplay_dispatch::gameplay_transport::{
    ComposedFreshAdmission, FreshEvidenceSource, SeamRooms,
};
use oteryn_gameplay_quests::quest::koshei_amulet as source;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, decode_use_intent_target,
    encode_item_use_intent,
};
use sqlx::Connection;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_world::map;
use oteryn_gameplay_world::map::boot::BundleWorld;

const CREATURE: &str = "oteryn:creature.koshei_the_deathless";
const QUEST: &str = "oteryn:quest.koshei_the_deathless_quest";
const MAIN_HASH: &str = "1a1c543dbe511c3a5654d1385956224b7323243ace53fa5eb142e676361fc420";
const FRAGMENT: &str =
    "oteryn:quest-progress/chosen-source/koshei_the_deathless_quest/s1/visited/koshei_amulet_part1";
const FRAGMENTS: &str = "oteryn:quest-progress/chosen-source/koshei_the_deathless_quest/s1";
const AMULET_PROGRESS: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet";
const AMULET_STARTED: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet_started_at";
const CORPSE: MovementLocalPosition = MovementLocalPosition {
    x: 33271,
    y: 32406,
    floor: 12,
};

#[test]
fn koshei_current_ground_handles_real_use_with_refusals_door_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = fixture::Harness::create_with_admission_seed(
                        admin.clone(),
                        "koshei_wire",
                        true,
                        false,
                    )
                    .await?;
                    let (login, url) = runtime_login(&h).await?;
                    let result = exercise(&h, &url, &login).await;
                    h.cleanup().await?;
                    let mut admin = sqlx::PgConnection::connect(&admin).await?;
                    sqlx::query(sqlx::AssertSqlSafe(format!("DROP ROLE {login}")))
                        .execute(&mut admin)
                        .await?;
                    result?;
                    eprintln!("KOSHEI_AUTHORITY_PG_EXECUTED");
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Koshei wire thread failed")?
        .map_err(Into::into)
}

async fn runtime_login(h: &fixture::Harness) -> TestResult<(String, String)> {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let login = format!("koshei_runtime_{suffix}");
    let password = format!("{login}_test");
    let options = h.pool.connect_options();
    let database = options.get_database().ok_or("test database name")?;
    for statement in [
        format!("CREATE ROLE {login} LOGIN PASSWORD '{password}' IN ROLE oteryn_game_runtime"),
        format!("GRANT CONNECT ON DATABASE {database} TO {login}"),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .execute(&h.pool)
            .await?;
    }
    let (_, address) = h
        .database
        .url
        .split_once('@')
        .ok_or("test database address")?;
    Ok((
        login.clone(),
        format!(
            "postgresql://{login}:{password}@{address}?options=-c%20role%3Doteryn_game_runtime"
        ),
    ))
}

async fn exercise(h: &fixture::Harness, runtime_url: &str, runtime_login: &str) -> TestResult {
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
    let (scene, start) = shipped_encounter(native, world, channel)?;
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
                x: start.x,
                y: start.y,
                floor: start.floor,
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
        12,
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
    assert_eq!((at.x, at.y, at.floor), (start.x, start.y, start.floor));
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
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
    let mut runtime_connection = sqlx::PgConnection::connect(runtime_url).await?;
    let (login, role): (String, String) = sqlx::query_as("SELECT session_user, current_user")
        .fetch_one(&mut runtime_connection)
        .await?;
    assert_eq!(login, runtime_login);
    assert_eq!(role, "oteryn_game_runtime");
    // The original route's lock requires UPDATE on Ground even for an empty result.
    let original_lock = sqlx::query(
        "SELECT spatial_position FROM game_item_ground_locations WHERE false FOR SHARE",
    )
    .execute(&mut runtime_connection)
    .await;
    assert!(matches!(
        original_lock,
        Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("42501")
    ));
    runtime_connection.close().await?;
    let runtime_root = crate::durability::DurabilityRoot::connect_test_runtime(runtime_url)?;
    assert!(runtime_root.maintain_ready_once().await?);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = runtime_root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner: ComposedFreshAdmission<'_, '_, '_> = ComposedTestOwner {
        root: &runtime_root,
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
    owner.rooms = SeamRooms::new(None, Some(&room));
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i7532").await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($3,'hex')::uuid)").bind(id(93).as_slice()).bind(world.as_bytes().as_slice()).bind(id(103).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(91).as_slice()).bind(id(93).as_slice()).bind(world.as_bytes().as_slice()).bind(id(113).as_slice()).execute(&mut *tx).await?;
    let amulet_started = restore_predecessor_save(&mut tx).await?;
    tx.commit().await?;
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    let index = native.item_index().ok_or("index")?;
    let reference = |id| {
        index
            .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
            .ok_or("definition")
    };
    let (amulet, gold, corpse, other) = (
        reference(7532)?,
        reference(3031)?,
        reference(7535)?,
        reference(7537)?,
    );
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: world,
        channel_id: channel,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                .map_err(debug)?,
            account_id: id(40),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            4,
            6,
            oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_V1,
            11,
            15,
            18,
        ],
    )
    .ok_or("capabilities")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Item(std::num::NonZeroU64::MIN)),
    })
    .map_err(debug)?;
    let reward_door = oteryn_world_bundle::bundle::placement_key(-12, 33269, 32446, 1)
        .ok_or("Koshei reward door source placement")?;
    assert_eq!(
        scene
            .facts()
            .doors()
            .toggle(reward_door, 0, false, 40, false),
        Some(map::door::DoorUse::Locked)
    );
    let no_target = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: None,
    })
    .map_err(debug)?;
    refuse(&mut owner, &mut admitted, h, &no_target, amulet, None).await?;
    let spawned = {
        let mut runtime = runtime.lock().await;
        let sources = scene
            .activation_spawns(|key| {
                let policy = runtime.companion_policy(key).ok()?;
                Some((
                    u64::try_from(policy.maximum_health).ok()?,
                    policy.blocked_by_nearby_players,
                ))
            })
            .ok_or("shipped spawn policies")?;
        let [(points, _)] = sources.as_slice() else {
            return Err("unique Koshei spawn source".into());
        };
        let [(position, delay, key, _, _)] = points.as_slice() else {
            return Err("unique Koshei spawn point".into());
        };
        assert_eq!(
            (*position, *delay, key.as_str()),
            ((CORPSE.x, CORPSE.y, CORPSE.floor), 15_000, CREATURE)
        );
        runtime
            .begin_activation_spawn_sources(sources)
            .map_err(debug)?;
        let progress = runtime.realize_activation_spawn_window(0).map_err(debug)?;
        assert_eq!((progress.realized_total, progress.remaining), (1, 0));
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, pos, session)| {
                (session.is_none() && pos.position() == CORPSE).then_some(actor)
            })
            .ok_or("live Koshei at the donor cell")?
    };
    composed_test_owner::defeat_spawned_creature(&owner, &admitted, spawned, 900, CREATURE).await?;
    let (body, materialized, retirement): (sqlx::types::Uuid, i64, Option<i64>) = sqlx::query_as(
        "SELECT item_instance_id,materialized_at,game_corpse_retirement_deadline(item_instance_id) FROM game_item_mint_receipts WHERE loot_purpose_key='CORPSE_MATERIALIZATION' AND loot_table_production_key='oteryn:item.tibia.i7535'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(retirement, None, "Crystal's last stage has no timer");
    let stages: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT duration_millis,target_production_key,expires_at_unix_ms-$2 FROM game_spell_field_temporal_schedules WHERE item_instance_id=$1 ORDER BY source_stage",
    ).bind(body).bind(materialized).fetch_all(&h.pool).await?;
    assert_eq!(
        stages,
        [
            (10_000, "oteryn:item.tibia.i7537".into(), 10_000),
            (10_000, "oteryn:item.tibia.i7538".into(), 20_000),
            (300_000, "oteryn:item.tibia.i7539".into(), 320_000),
            (300_000, "oteryn:item.tibia.i7540".into(), 620_000),
        ]
    );
    refuse(&mut owner, &mut admitted, h, &payload, gold, Some(corpse)).await?;
    let until_first_decay: i64 = sqlx::query_scalar(
        "SELECT greatest(0, $1::bigint - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint)",
    )
    .bind(materialized + 10_050)
    .fetch_one(&h.pool)
    .await?;
    tokio::time::sleep(std::time::Duration::from_millis(
        until_first_decay.try_into()?,
    ))
    .await;
    runtime_root.maintain_ready_once().await?;
    composed_test_owner::drain_source_item_deadlines(&owner).await;
    let aged: String = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=$1",
    )
    .bind(body)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(aged, "oteryn:item.tibia.i7537");
    refuse(&mut owner, &mut admitted, h, &payload, amulet, Some(other)).await?;

    // Drive the ordinary delay, warning and admission windows with no nearby viewers.
    let respawned = {
        let mut runtime = runtime.lock().await;
        let mut reports = Vec::new();
        for now in [60_000_000, 90_000_000, 120_000_000] {
            reports.push(
                runtime
                    .drive_spawn_respawns(now, |_, _| false)
                    .map_err(debug)?,
            );
        }
        runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find_map(|(actor, pos, session)| {
                (session.is_none() && pos.position() == CORPSE && actor != spawned).then_some(actor)
            })
            .ok_or_else(|| {
                format!("Koshei did not respawn from the shipped encounter: {reports:?}")
            })?
    };
    owner.clock_origin -= std::time::Duration::from_secs(120);
    composed_test_owner::defeat_spawned_creature(&owner, &admitted, respawned, 901, CREATURE)
        .await?;
    let before = ground_snapshot(h).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let (next, result, actual, feedback) = wire(
        &owner,
        admitted,
        command,
        &payload,
        Some(amulet),
        Some(corpse),
        true,
    )
    .await?;
    admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(feedback, source::callback()?.success_message);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    let copy = runtime_root
        .read_character_quest_state(&authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().get(source::TRACK), Some(&1));
    assert!(copy.meets_door_storage(41351));
    assert_eq!(
        scene
            .facts()
            .doors()
            .toggle(reward_door, 0, false, 40, copy.meets_door_storage(41351)),
        Some(map::door::DoorUse::Toggled {
            revision: 1,
            open: true
        })
    );
    assert_eq!(copy.tracks().get(FRAGMENT), Some(&1));
    assert_eq!(copy.tracks().get(FRAGMENTS), Some(&1));
    assert_eq!(copy.tracks().get(AMULET_PROGRESS), Some(&2));
    assert_eq!(copy.tracks().get(AMULET_STARTED), Some(&amulet_started));
    let (_, replay, _, _) = wire(&owner, admitted, command, &actual, None, None, false).await?;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (1, 0, 1, 1));
    let after = ground_snapshot(h).await?;
    assert_eq!(before, after);
    assert_eq!(
        scene
            .facts()
            .movement_effects_since(0)
            .iter()
            .filter(|(_, at, effect)| i32::from(at.x) == CORPSE.x
                && i32::from(at.y) == CORPSE.y
                && -i16::from(at.floor) == CORPSE.floor
                && *effect == 14)
            .count(),
        1
    );
    Ok(())
}

/// Restore the pinned mainland save with contiguous receipts for its prior actions.
async fn restore_predecessor_save(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> TestResult<i64> {
    let started_at: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM statement_timestamp()))::bigint - 86401",
    )
    .fetch_one(&mut **tx)
    .await?;
    let history = [
        (
            "step",
            "oteryn:quest-callback/m01b/koshei-the-deathless/s1/koshei_amulet_part1",
            started_at - 1,
            [(FRAGMENT, 0, 1), (FRAGMENTS, 0, 1)],
        ),
        (
            "command",
            "oteryn:quest-reward/qnpcfx-koshei-start",
            started_at,
            [(AMULET_PROGRESS, 0, 1), (AMULET_STARTED, 0, started_at)],
        ),
        (
            "command",
            "oteryn:quest-reward/qnpcfx-koshei-collect",
            started_at + 86400,
            [
                (AMULET_PROGRESS, 1, 2),
                (AMULET_STARTED, started_at, started_at),
            ],
        ),
    ];
    for (original, (cause, transition, at, effects)) in (1_i64..).zip(history) {
        let committed = original + 1;
        let tracks = effects.map(|(track, _, _)| track);
        let before = effects.map(|(_, before, _)| before);
        let after = effects.map(|(_, _, after)| after);
        sqlx::query(
            "INSERT INTO game_character_quest_receipts(\
               character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,\
               request_binding,pinned_content_revision,definition_hash,completes,\
               track_keys,values_before,values_after,original_character_revision,\
               committed_character_revision,level_before,level_after,experience_before,\
               experience_after,profile_revision,ruleset_revision,content_revision,\
               simulation_revision,evidence_revision,declaration_revision,policy_revision,\
               reward_revision,committed_at) \
             SELECT character_id,$2,encode($3,'hex')::uuid,$4,$5,$6,\
               sha256(uuid_send(character_id)||convert_to($5,'UTF8')),'content-1',decode($7,'hex'),false,\
               $8,$9,$10,$4,$11,level,level,total_experience,total_experience,\
               profile_revision,ruleset_revision,content_revision,simulation_revision,\
               evidence_revision,declaration_revision,policy_revision,reward_revision,$12 \
             FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice()).bind(cause).bind(id(51).as_slice()).bind(original)
        .bind(transition).bind(QUEST).bind(MAIN_HASH).bind(tracks.as_slice())
        .bind(before.as_slice()).bind(after.as_slice()).bind(committed).bind(at * 1000)
        .execute(&mut **tx).await?;
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,quest_key,track_key,value,committed_character_revision) \
             SELECT encode($1,'hex')::uuid,$2,track,value,$5 FROM unnest($3::text[],$4::bigint[]) AS change(track,value) \
             ON CONFLICT(character_id,track_key) DO UPDATE \
             SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
        )
        .bind(id(41).as_slice()).bind(QUEST).bind(tracks.as_slice()).bind(after.as_slice())
        .bind(committed).execute(&mut **tx).await?;
    }
    let prior_save = sqlx::query(
        "WITH prior_save AS (\
           UPDATE game_character_roots SET character_revision=4 \
           WHERE character_id=encode($1,'hex')::uuid AND character_revision=1 \
           RETURNING character_id,character_revision\
         ) UPDATE game_character_progression_state p \
           SET character_revision=s.character_revision FROM prior_save s \
           WHERE p.character_id=s.character_id AND p.character_revision=1",
    )
    .bind(id(41).as_slice())
    .execute(&mut **tx)
    .await?;
    assert_eq!(prior_save.rows_affected(), 1);
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) \
         SELECT character_id,$2,'content-1',decode($3,'hex'),2,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice()).bind(QUEST).bind(MAIN_HASH).execute(&mut **tx).await?;
    Ok(started_at)
}

async fn snapshot(h: &fixture::Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i7532' AND lifecycle=1), (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2), (SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1)").bind(source::TRACK).bind(source::KEY).fetch_one(&h.pool).await?)
}

fn shipped_encounter(
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<(BundleWorld, MovementLocalPosition)> {
    use oteryn_world_bundle::bundle::{BuildClass, Identity};
    use oteryn_world_bundle_compiler::{compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut project = source::project_regions(&root, &[(129, 126), (130, 126)])?;
    project
        .families
        .spawns
        .retain(|spawn| spawn.key == "oteryn:spawn.x33271_y32406_z12");
    let (registry, palette) = source::registry(&root, &project)?;
    let identity = Identity {
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "content-1".into(),
        ..Identity::default()
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: identity.clone(),
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    assert_eq!(compiled.spawns.points, 1);
    let pins = map::BundlePins {
        digest: compiled.digest,
        project_format_version: identity.project_format_version,
        world_schema_version: identity.world_schema_version,
        content_revision: identity.content_revision,
        production: false,
    };
    let at = |x, y| map::overlay::TilePos { x, y, floor: -12 };
    let start = at(33271, 32405);
    let scene = map::boot::boot(
        &compiled.bytes,
        &map::boot::BootPins {
            map_revision: format!(
                "sha256:{}",
                compiled
                    .digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            bundle: pins,
            start,
        },
        world,
        channel,
        |key| map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)?;
    assert!(scene.enterable(start));
    assert!(scene.enterable(at(33271, 32406)));
    Ok((
        scene,
        MovementLocalPosition {
            x: i32::from(start.x),
            y: i32::from(start.y),
            floor: 12,
        },
    ))
}

#[test]
fn koshei_catacomb_source_route_connects_surface_to_encounter() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let world = fixture::fence(1).map_err(|error| error.to_string())?.runtime_scope.world_id();
            let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(|error| format!("{error:?}"))?;
            let (controller, _) = oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
            let native = controller.active().and_then(|active| active.native_gameplay()).ok_or("native gameplay")?;
            let (scene, _) = shipped_encounter(native, world, channel).map_err(|error| error.to_string())?;
            assert_catacomb_route(&scene).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Koshei map thread failed")?
        .map_err(Into::into)
}

fn assert_catacomb_route(scene: &BundleWorld) -> TestResult {
    use map::door::DoorUse;
    use map::facts::{QuestCallbackProjection, QuestTimedPassage};
    use map::floor::{Ground, palette_mask, resolve_landing};
    use map::view::MapFacts;
    use oteryn_gameplay_world::movement::CardinalStep;
    use oteryn_world_bundle::bundle::{Family, placement_key};
    use std::collections::{BTreeSet, VecDeque};

    let gates = scene.facts().doors();
    let ordinary = placement_key(-8, 33276, 32451, 1).ok_or("catacomb door")?;
    let expertise = placement_key(-12, 33271, 32418, 1).ok_or("Koshei level gate")?;
    assert_eq!(
        gates.toggle(expertise, 0, false, 39, false),
        Some(DoorUse::Locked)
    );
    for key in [ordinary, expertise] {
        assert_eq!(
            gates.toggle(key, 0, false, 40, false),
            Some(DoorUse::Toggled {
                revision: 1,
                open: true
            })
        );
    }

    let coffin = map::overlay::TilePos {
        x: 33273,
        y: 32458,
        floor: -8,
    };
    let coffin_key =
        placement_key(coffin.floor, coffin.x, coffin.y, 1).ok_or("coffin placement")?;
    let facts = scene.facts();
    let base_item = |at: map::overlay::TilePos, ordinal: u8| {
        let tile = scene.base().tile(at.x, at.y, at.floor)?;
        let (&id, _) = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))?;
        Some((id, facts.base_entry(at, ordinal, id)?))
    };
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let sundial = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-callback/koshei-sundial")
        .ok_or("registered sundial")?;
    for callback in callbacks.iter().filter(|callback| {
        callback
            .key
            .starts_with("oteryn:quest-callback/koshei-statue-")
            || callback.key == "oteryn:quest-callback/koshei-sundial"
    }) {
        let binding = callback
            .selector
            .native
            .as_ref()
            .ok_or("catacomb selector")?;
        let at = map::overlay::TilePos {
            x: binding.position.x,
            y: binding.position.y,
            floor: binding.position.floor,
        };
        let key = placement_key(at.floor, at.x, at.y, binding.ordinal).ok_or("catacomb object")?;
        let (id, current) = base_item(at, binding.ordinal).ok_or("catacomb base object")?;
        assert!(facts.quest_callback_native_object(key, id, binding.appearance));
        if let Some([_, appearance]) = callback.lever {
            let required = sundial
                .world_requirements
                .iter()
                .find(|requirement| requirement.position == binding.position)
                .ok_or("sundial statue requirement")?;
            if current.appearance_id != required.appearance {
                let mut turned = current;
                turned.appearance_id = appearance;
                assert!(facts.apply_quest_callback(key, 0, turned, &[], &[], false, None));
            }
            assert_eq!(
                base_item(at, binding.ordinal).map(|(_, entry)| entry.appearance_id),
                Some(required.appearance)
            );
        } else {
            assert!(!scene.enterable(coffin));
            let (_, closed) = base_item(coffin, 1).ok_or("closed coffin")?;
            let opened = facts
                .quest_callback_replacement(coffin_key, 167)
                .ok_or("open coffin Item")?;
            assert_eq!((closed.appearance_id, opened.appearance_id), (162, 167));
            let due = u64::from(
                callback
                    .effects
                    .passages_reset_after_ms
                    .ok_or("coffin timer")?,
            ) * 1_000;
            let projection = QuestCallbackProjection::TimedPassages {
                due,
                entries: vec![QuestTimedPassage {
                    key: coffin_key,
                    expected_revision: 0,
                    before: closed,
                    after: Some(opened),
                    passable: true,
                }],
                source_reset: None,
            };
            assert!(facts.apply_quest_callback(
                key,
                0,
                current,
                &[],
                &[],
                false,
                Some(&projection)
            ));
            assert_eq!(facts.expire_movement_items(due - 1), 0);
        }
    }
    assert!(scene.enterable(coffin));
    let mut floors = scene.floors().clone();
    let mask = palette_mask(Family::Item, "oteryn:item.tibia.i167");
    assert_ne!(mask, 0);
    floors.add(coffin, mask);
    let ground = |(x, y, z)| {
        let (Ok(x), Ok(y), Ok(z)) = (u16::try_from(x), u16::try_from(y), i8::try_from(z)) else {
            return Ground::Absent;
        };
        let at = map::overlay::TilePos { x, y, floor: -z };
        if scene.base().tile(at.x, at.y, at.floor).is_none() {
            Ground::Absent
        } else if scene.enterable(at) {
            Ground::Walkable
        } else {
            Ground::Blocked
        }
    };
    let surface = (33281, 32444, 7);
    let boss = (CORPSE.x, CORPSE.y, i32::from(CORPSE.floor));
    let mut visited = BTreeSet::from([surface]);
    let mut pending = VecDeque::from([surface]);
    while let Some(cell) = pending.pop_front() {
        if cell == boss {
            break;
        }
        for direction in [
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::South,
            CardinalStep::West,
        ] {
            if let Ok(next) = resolve_landing(&floors, ground, |_| true, cell, direction)
                && visited.insert(next)
            {
                pending.push_back(next);
            }
        }
    }
    assert!(
        visited.contains(&boss),
        "the opened catacomb route must reach Koshei from the mainland"
    );
    assert_eq!(facts.expire_movement_items(120_000_000), 1);
    assert!(!scene.enterable(coffin));
    for key in [ordinary, expertise] {
        assert_eq!(
            gates.toggle(key, 1, false, 40, false),
            Some(DoorUse::Toggled {
                revision: 2,
                open: false
            })
        );
    }
    Ok(())
}

async fn ground_snapshot(h: &fixture::Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT coalesce(jsonb_agg(jsonb_build_array(to_jsonb(i),to_jsonb(g)) ORDER BY i.item_instance_id),'[]') FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key IN ('oteryn:item.tibia.i7535','oteryn:item.tibia.i7537')").fetch_one(&h.pool).await?)
}

async fn refuse(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &fixture::Harness,
    payload: &[u8],
    tool: std::num::NonZeroU32,
    target: Option<std::num::NonZeroU32>,
) -> TestResult {
    let before = ground_snapshot(h).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (next, result, _, _) = wire(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        payload,
        Some(tool),
        target,
        false,
    )
    .await?;
    *admitted = next;
    assert_ne!(result, UseDisposition::Committed);
    assert_eq!(snapshot(h).await?, (0, 1, 0, 1));
    assert_eq!(ground_snapshot(h).await?, before);
    Ok(())
}

async fn wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command: u64,
    payload: &[u8],
    source: Option<std::num::NonZeroU32>,
    target: Option<std::num::NonZeroU32>,
    feedback_expected: bool,
) -> TestResult<(AdmittedSession, UseDisposition, Vec<u8>, Option<String>)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let client_side = async move {
        let (mut source_handle, mut target_handle) = (None, None);
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&bytes)?;
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
                        source_handle = inventory
                            .entries
                            .iter()
                            .find(|entry| Some(entry.item_definition_ref) == source)
                            .map(|entry| entry.handle)
                            .or(source_handle);
                    }
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        target_handle =
                            map.tiles
                                .iter()
                                .flat_map(|tile| &tile.items)
                                .find_map(|item| match (item.definition, item.origin) {
                                    (
                                        MapDefinition::Item(definition),
                                        MapOrigin::Handle(handle),
                                    ) if Some(definition) == target => Some(handle),
                                    _ => None,
                                })
                                .or(target_handle);
                    }
                }
            }
            if envelope.message_type() == crate::foundation::MessageType::SnapshotCommit {
                break;
            }
        }
        let actual = if source.is_some() || target.is_some() {
            let mut intent = decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                payload,
            )
            .map_err(debug)?;
            if source.is_some() {
                intent.target = UseTarget::Item(source_handle.ok_or("current source handle")?);
            }
            if target.is_some() {
                intent.use_with_map_item = Some(MapUseTarget::Item(
                    target_handle.ok_or("current Ground handle")?,
                ));
            }
            encode_item_use_intent(&intent).map_err(debug)?
        } else {
            payload.to_vec()
        };
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: command,
                command_type: 2,
                payload: &actual,
            },
        )?;
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        let mut result = None;
        let mut feedback = None;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&bytes)?;
            if envelope.message_type() == crate::foundation::MessageType::CommandResult {
                let envelope_result =
                    oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                if envelope_result.command_id == command {
                    result = Some(
                        oteryn_protocol_oteryn::world_object::decode_use_result(
                            envelope_result.payload,
                        )
                        .map_err(debug)?,
                    );
                }
            }
            if feedback_expected
                && envelope.message_type() == crate::foundation::MessageType::StateDelta
            {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id == oteryn_protocol_oteryn::chat::STATE_DOMAIN_CHAT
                    && delta.delta_type == 1
                    && let oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } =
                        oteryn_protocol_oteryn::chat::decode_chat_line(delta.payload)
                            .map_err(debug)?
                {
                    feedback = Some(text);
                }
            }
            if let Some(result) = result
                && (!feedback_expected || feedback.is_some() || result != UseDisposition::Committed)
            {
                client.shutdown().await?;
                return Ok::<_, Box<dyn std::error::Error>>((result, actual, feedback));
            }
        }
    };
    let server_side = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exchange = async {
        let (result, end) = tokio::join!(client_side, server_side);
        let (result, actual, feedback) = result?;
        let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
            return Err(format!("unexpected server end {end:?}").into());
        };
        Ok::<_, Box<dyn std::error::Error>>((next, result, actual, feedback))
    };
    tokio::time::timeout(std::time::Duration::from_secs(30),async {tokio::select! {result=exchange=>result,result=maintain_ready(owner)=>{result?;Err("maintenance stopped".into())}}}).await?
}

async fn read_frame(stream: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
    let length = usize::try_from(stream.read_u32().await?)?;
    if length > 1 << 20 {
        return Err("frame bound".into());
    }
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn maintain_ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    loop {
        if owner.root.has_ready_demand() {
            owner.root.maintain_ready_once().await.map_err(debug)?;
        }
        tokio::task::yield_now().await;
    }
}
