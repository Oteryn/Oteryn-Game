//! Level gates and level-scaled rewards through the real NPC dialogue owner.
use crate::bestiary_postgres_harness as fixture;
use fixture::{debug, id};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId, NodeId};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_gameplay_quests::quest::reward_callbacks;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;
use tokio::sync::Mutex;
type TestResult<T = ()> = fixture::TestResult<T>;

#[test]
fn reward_callback_maximum_level_refuses_above_bound() -> TestResult { run(false, 25) }
#[test]
fn reward_callback_maximum_level_pays_at_or_below_bound() -> TestResult {
    run(false, 24)?;
    run(false, 20)
}
#[test]
fn reward_callback_level_scaled_experience_pays_factor_times_level() -> TestResult {
    run(true, 20)?;
    run(true, 40)
}

fn run(spirit: bool, level: i64) -> TestResult {
    let Some(admin) = fixture::configured_admin() else { return Ok(()); };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(debug)?.block_on(async {
            let h = fixture::Harness::create_with_admission_seed(admin, "reward_levels", true, false).await?;
            let result = exercise(&h, spirit, level).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup?;
            eprintln!("PG_EXECUTED reward_levels spirit={spirit} level={level}");
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "reward level PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness, spirit: bool, level: i64) -> TestResult {
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
    let start = pin.qualified_room().entry_start();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
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
    let runtime = Mutex::new(runtime);
    let room = oteryn_gameplay_content::content::qualify_native_entry_room(world).map_err(debug)?;
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


    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let key = if spirit { "oteryn:quest-reward/l6-spirit-confession" } else { "oteryn:quest-reward/m01c-pilgrimage-s8" };
    let callback = reward_callbacks::load_for(catalogue)?.into_iter().find(|c| c.key == key).ok_or("production callback")?;
    let terms = serde_json::to_value(&callback)?;
    if spirit { assert_eq!(terms["experience_per_level"], 50); }
    else { assert_eq!(terms["maximum_level"], 24); }
    // Establish only this terminal dialogue's prerequisites; production callback,
    // catalogue, fresh session, premium and all durable writers remain real.
    super::engnpc2_live_transport::seed_tool(h, native, "oteryn:item.tibia.i11544").await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$1").bind(level).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_instances SET quantity=3 WHERE definition_production_key='oteryn:item.tibia.i11544'").execute(&mut *tx).await?;
    for s in &callback.storage {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,value,quest_key,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=excluded.value")
            .bind(id(41).as_slice()).bind(&s.track).bind(s.minimum).bind(&callback.quest).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let fence = composed_test_owner::load_quests(&owner, session).await?;
    let npcs = oteryn_gameplay_content::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")),
        std::ffi::OsStr::new("world"),
        oteryn_gameplay_content::content::npc_catalogue_preproduction_limits(),
    ).map_err(debug)?;
    let model = npcs.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(Arc::new(oteryn_gameplay_npc::npc_talk::NpcTalkBook::from_model_with_native_quests(&model, &[&callback.npc])));
    owner.states.chat.admit(session, "Reward witness", oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1));
    owner.states.chat.set_npc_service(session);
    let npc = {
        let mut r = runtime.lock().await;
        r.place_npcs(&crate::foundation::NpcPlacementTable { npcs: vec![crate::foundation::NpcPlacements {
            key: callback.npc.clone(), placements: vec![crate::foundation::NpcPlacement {
                x: u16::try_from(at.x + 1)?, y: u16::try_from(at.y)?, floor: -i8::try_from(at.floor)?, direction: crate::foundation::MovementFacing::South,
            }],
        }] }, |_| true, true).map_err(debug)?;
        let n = r.visible_entities().npcs.into_iter().next().ok_or("placed NPC")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef { identity: n.actor.placement_identity(), generation: n.generation }
    };
    let say = |command_id, text: String| owner.npc_talk(actor, UseCommand { game_session_id: session, command_id, item_fence: Some(fence.item_fence()) }, NpcTalkIntent { npc_actor: npc, text });
    let hello = say(17000, "hi".into()).await;
    assert_eq!(hello.disposition, NpcIntentDisposition::Ok);
    let outcome = say(17001, callback.keywords[0].clone()).await;
    let gold: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1").fetch_one(&h.pool).await?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1").bind(key).fetch_one(&h.pool).await?;
    if !spirit && level > 24 {
        assert_eq!(outcome.disposition, NpcIntentDisposition::Rejected);
        assert_eq!(gold, 0);
        assert_eq!(receipts, 0);
        assert_eq!(outcome.conversation.ok_or("refusal conversation")?.lines, callback.failure_reply);
    } else {
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        assert_eq!(receipts, 1);
        if spirit {
            let amount: i64 = sqlx::query_scalar("SELECT coalesce((SELECT sum(amount) FROM game_character_quest_xp_obligations),(SELECT sum(experience_awarded) FROM game_character_xp_receipts),0)::bigint").fetch_one(&h.pool).await?;
            assert_eq!(amount, 50 * level);
        } else { assert_eq!(gold, 20); }
    }
    Ok(())
}
