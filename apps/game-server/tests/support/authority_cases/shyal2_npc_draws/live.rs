//! The Character fixture supplies placed actors to the production NPC dispatcher.
use super::{ORDER, PART, draw_program, fixture};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementFacing, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use fixture::{debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{ComposedFreshAdmission, FreshEvidenceSource};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::Mutex;

#[test]
fn shyal2_live_telas_dialogue_assigns_three_distinct_ingredients() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "shyal2_live_telas", &ORDER, 6, 1)
}

#[test]
fn shyal2_live_golem_dialogue_draws_each_request_from_the_full_pool() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "shyal2_live_golem", &[PART], 12, 13)
}

fn run(
    admin: String,
    label: &'static str,
    tracks: &'static [&'static str],
    pool: i64,
    requests: u64,
) -> fixture::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create_with_admission_seed(admin, label, true, false)
                        .await?;
                    let result = exercise(&h, tracks, pool, requests).await;
                    h.cleanup().await?;
                    result?;
                    eprintln!(
                        "POSTGRES_CASE_COMPLETED {label} duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "live NPC draw thread failed")?
        .map_err(Into::into)
}

async fn exercise(
    h: &fixture::Harness,
    tracks: &[&str],
    pool: i64,
    requests: u64,
) -> fixture::TestResult {
    use oteryn_gameplay_content::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use oteryn_gameplay_npc::npc_talk::NpcTalkBook;

    let program = draw_program(tracks, pool)?;
    super::super::engnpc2_live_transport::fresh(h).await?;
    let scope = fixture::fence(1)?.runtime_scope;
    let world = scope.world_id();
    let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let (controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world, None,
        );
    let generation = controller.active().ok_or("active generation")?;
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
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: program.npc.into(),
                    placements: vec![NpcPlacement {
                        x: u16::try_from(at.x)?,
                        y: u16::try_from(at.y + 1)?,
                        floor: -i8::try_from(at.floor)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find_map(|npc| {
            (runtime.npc_key(npc.actor) == Some(program.npc)).then_some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        })
        .ok_or("placed draw NPC")?;
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
            include_bytes!("../../spell-save-tls-fixture/ca.der").to_vec(),
        )],
        vec![tokio_rustls::rustls::pki_types::CertificateDer::from(
            include_bytes!("../../spell-save-tls-fixture/client.der").to_vec(),
        )],
        tokio_rustls::rustls::pki_types::PrivateKeyDer::try_from(
            include_bytes!("../../spell-save-tls-fixture/client-key.der").to_vec(),
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
        program.catalogue.with_fixture_npc_rules(program.source),
    ));
    let npcs = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = npcs.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model,
            &[program.npc],
        )));
    owner.states.chat.admit_as(
        session,
        "Ritual witness",
        Some(id(fixture::CHARACTER)),
        SemanticTimeMicros::from_micros(0),
    );
    owner.states.chat.set_npc_service(session);
    let mut fence = composed_test_owner::load_quests(&owner, session).await?;
    let greeting = talk(&owner, actor, npc, fence, 8500, "hi").await;
    assert_eq!(greeting.disposition, NpcIntentDisposition::Ok);
    let mut observed = BTreeSet::new();
    for request in 0..requests {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("NPC dispatch clock fixture")?;
        let outcome = talk(&owner, actor, npc, fence, 8501 + request, program.keyword).await;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{request}");
        assert!(!outcome.unknown);
        let next = composed_test_owner::load_quests(&owner, session).await?;
        assert_eq!(
            next.expected_character_revision.get(),
            fence.expected_character_revision.get() + 1
        );
        fence = next;
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        let assigned = tracks
            .iter()
            .map(|track| {
                state
                    .tracks()
                    .get(*track)
                    .copied()
                    .ok_or("NPC draw missing")
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(assigned.len(), tracks.len());
        assert!(assigned.iter().all(|index| (1..=pool).contains(index)));
        observed.extend(assigned);
    }
    if requests > 1 {
        assert!(observed.len() < usize::try_from(requests)?);
    } else {
        let repeated = talk(&owner, actor, npc, fence, 8600, program.keyword).await;
        assert!(!repeated.unknown);
        let read = composed_test_owner::load_quests(&owner, session).await?;
        assert_eq!(
            read.expected_character_revision,
            fence.expected_character_revision
        );
    }
    Ok(())
}

async fn talk(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    npc: EntityRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    command_id: u64,
    text: &str,
) -> oteryn_gameplay_npc::npc_command::NpcTalkOutcome {
    owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: fence.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await
}
