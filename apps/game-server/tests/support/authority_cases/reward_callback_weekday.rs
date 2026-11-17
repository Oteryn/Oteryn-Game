//! An authored weekday prerequisite through the production NPC talk dispatcher.
use super::engnpc2_live_transport::{fresh, seed_tool};
use crate::bestiary_postgres_harness as fixture;
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CharacterId, CharacterLease, GameSessionId,
    MovementFacing, NodeId, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use fixture::{TestResult, debug, id};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::{
    self, ComposedTestOwner, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_dispatch::gameplay_transport::{
    ComposedFreshAdmission, FreshEvidenceSource, NpcTalkBook,
};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
use tokio::sync::Mutex;

const NPC: &str = "oteryn:npc.paulie";
const QUEST: &str = "oteryn:quest.weekday_reward_fixture";
const TRACK: &str = "oteryn:quest-progress/weekday-reward-fixture/stage";
const CALLBACK: &str = "oteryn:quest-reward/weekday-reward-fixture";

#[test]
fn reward_callback_weekday_refuses_then_grants_through_npc_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h = fixture::Harness::create_with_admission_seed(
                        admin,
                        "reward_callback_weekday",
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "weekday reward PG thread failed")?
        .map_err(Into::into)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let callback = serde_json::json!({
        "key": CALLBACK, "quest": QUEST, "npc": NPC,
        "keywords": ["weekday reward"], "topic": 0, "next_topic": 0,
        "requires_weekdays": [1, 3],
        "storage": [{"track": TRACK, "minimum": 0, "maximum": 0, "set": 1}],
        "take": [], "give": {"item": "oteryn:item.tibia.i3035", "count": 1, "contents": []},
        "outfit": null, "achievement": null,
        "reply": ["Here is your reward."], "sources": ["NPC reward weekday regression case"]
    });
    let recipe_sha256 = {
        use sha2::{Digest, Sha256};
        Sha256::digest(serde_json::to_vec(&callback)?)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let bindings: Bindings = serde_json::from_value(serde_json::json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST, "recipe_sha256": recipe_sha256,
            "tracks": [{"key": TRACK, "initial": 0, "minimum": 0, "maximum": 1}],
            "stages": [{
                "key": "claim", "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": callback}
            }],
            "sources": ["NPC reward weekday regression case"]
        }]
    }))?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
    bindings.attach(&base).map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> TestResult {
    fresh(h).await?;
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
    seed_tool(h, native, "oteryn:item.tibia.i17673").await?;
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
                    key: NPC.into(),
                    placements: vec![NpcPlacement {
                        floor: -i8::try_from(at.floor)?,
                        x: u16::try_from(at.x + 1)?,
                        y: u16::try_from(at.y)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let placed = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .next()
        .ok_or("NPC")?;
    let npc = EntityRef {
        identity: placed.actor.placement_identity(),
        generation: placed.generation,
    };
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
    owner.quest_catalogue = Some(Arc::new(catalogue()?));
    composed_test_owner::load_quests(&owner, session).await?;
    let npc_model = oteryn_gameplay_content::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content")),
        std::ffi::OsStr::new("world"),
        oteryn_gameplay_content::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?
    .service_model_for_project()
    .map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &npc_model,
            &[NPC],
        )));
    owner.states.chat.admit(
        session,
        "Weekday witness",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    );
    owner.states.chat.set_npc_service(session);

    // Monday 05:59 CEST is still Sunday on the server-save calendar.
    let before_save: i64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM TIMESTAMPTZ '2026-07-06 05:59 Europe/Berlin')::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    h.root.set_test_quest_clock(before_save).map_err(debug)?;
    let greeting = talk(&mut owner, actor, session, npc, 1, "hi").await?;
    assert_eq!(greeting.0, NpcIntentDisposition::Ok);
    let before = snapshot(h).await?;
    assert_eq!(before, (0, 0, 0));
    let refused = talk(&mut owner, actor, session, npc, 2, "weekday reward").await?;
    assert_eq!(refused.0, NpcIntentDisposition::Rejected);
    assert!(!refused.1);
    assert!(!refused.2);
    assert_eq!(snapshot(h).await?, before);

    h.root.advance_test_quest_clock(60).map_err(debug)?;
    let granted = talk(&mut owner, actor, session, npc, 3, "weekday reward").await?;
    assert_eq!(granted.0, NpcIntentDisposition::Ok);
    assert!(!granted.1);
    assert!(granted.2);
    assert_eq!(snapshot(h).await?, (before.0 + 1, 1, before.2 + 1));
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: GameSessionId,
    npc: EntityRef,
    command_id: u64,
    text: &str,
) -> TestResult<(NpcIntentDisposition, bool, bool)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC conversation clock")?;
    let fence = composed_test_owner::load_quests(owner, session).await?;
    let outcome = owner
        .npc_talk(
            actor,
            UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await;
    Ok((
        outcome.disposition,
        outcome.unknown,
        outcome.inventory_changed,
    ))
}

async fn snapshot(h: &fixture::Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT
            (SELECT COALESCE(sum(i.quantity),0)::bigint
               FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
               JOIN game_item_instances i ON i.item_instance_id=c.item
              WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3035'),
            COALESCE((SELECT value FROM game_character_quest_tracks
                       WHERE character_id=encode($1,'hex')::uuid AND track_key=$3), 0),
            (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$4)",
    )
    .bind(id(fixture::CHARACTER).as_slice())
    .bind(id(fixture::WORLD).as_slice())
    .bind(TRACK)
    .bind(CALLBACK)
    .fetch_one(&h.pool)
    .await?)
}
