//! Sinatuki's shipped dialogue and quest hand-ins through framed NPC commands.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MessageType, MovementFacing,
    MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements, WorldId,
    decode_wire_envelope,
};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, loader::load_embedded_quest_state};
use oteryn_protocol_oteryn::npc_service::{
    CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, DELTA_TYPE_NPC_CONVERSATION_V1,
    NpcIntentDisposition, NpcTalkIntent, STATE_DOMAIN_NPC_CONVERSATION, decode_npc_conversation,
    decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES, EntityRef,
};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

const NPC: &str = "oteryn:npc.sinatuki";
const QUEST: &str = "oteryn:quest.sinatuki_quest";
const STAGE: &str = "oteryn:quest-progress/qc/sinatuki/fish";
const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const SHARD: &str = "oteryn:item.tibia.i7290";
const WRONG: &str = "oteryn:item.tibia.i3578";
const HAND_IN: &str =
    "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>";
const REWARD: &str = "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you and gives you five glimmering crystals>";
const GIGGLE: &str = "Kisavuta! <giggles>";
const TRACKS: [(&str, i64); 2] = [
    (
        "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
        3,
    ),
    (
        "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
        5,
    ),
];

#[derive(Clone, Copy)]
enum Case {
    Trout,
    PerchResume,
    EmbeddedKeyword,
    KeywordPrefix,
    MissingDonorPrerequisite,
}

#[test]
fn sinatuki_exchange_postgres_dispatch() -> TestResult {
    run_case("sinatuki_exchange", Case::Trout)
}

#[test]
fn sinatuki_exchange_postgres_resume() -> TestResult {
    run_case("sinatuki_resume", Case::PerchResume)
}

#[test]
fn sinatuki_exchange_rejects_an_embedded_occurrence() -> TestResult {
    run_case("sinatuki_embedded_occurrence", Case::EmbeddedKeyword)
}

#[test]
fn sinatuki_exchange_accepts_a_keyword_prefix() -> TestResult {
    run_case("sinatuki_keyword_prefix", Case::KeywordPrefix)
}

#[test]
fn sinatuki_exchange_missing_donor_prerequisite_is_refused_partial() -> TestResult {
    run_case("sinatuki_donor_partial", Case::MissingDonorPrerequisite)
}

fn run_case(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || scenario(name, case))?;
    eprintln!("POSTGRES_CASE_COMPLETED {name} {:?}", started.elapsed());
    Ok(())
}

fn scenario(name: &str, case: Case) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        owner.quest_catalogue = Some(Arc::new(
            load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let tracks = if matches!(case, Case::MissingDonorPrerequisite) {
            &TRACKS[..1]
        } else {
            &TRACKS
        };
        seed(
            &h,
            owner.quest_catalogue.as_deref().ok_or("quest catalogue")?,
            tracks,
        )
        .await?;
        deps.initialize(&owner).await;
        let placement = shipped_placement()?;
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: i32::from(placement.x) - 1,
                        y: i32::from(placement.y),
                        floor: -i16::from(placement.floor),
                    },
                )
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![placement],
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
                .ok_or("Sinatuki placement")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        open_session(&owner, &mut admitted).await?;

        talk(
            &mut owner,
            &mut admitted,
            npc,
            "chuqua",
            Some(&["Chuqua jamjam!! Tiyopa Sinatuki?"]),
        )
        .await?;
        let keyword_case: Option<(&str, Option<&[&str]>)> = match case {
            Case::EmbeddedKeyword => Some(("nupi anupi", Some(&[]))),
            Case::KeywordPrefix => Some(("nupix", Some(&[REWARD]))),
            Case::MissingDonorPrerequisite => Some(("nupi", None)),
            Case::Trout | Case::PerchResume => None,
        };
        if let Some((text, reply)) = keyword_case {
            add_item(&h, 201, TROUT, 100, 1).await?;
            talk(&mut owner, &mut admitted, npc, text, reply).await?;
            let traded = matches!(case, Case::KeywordPrefix);
            assert_inventory(&h, if traded { [0, 0, 5] } else { [100, 0, 0] }).await?;
            assert_eq!(
                quest_state(&h).await?,
                if traded {
                    (Some(2), true, 1)
                } else {
                    (None, false, 0)
                }
            );
            drop(seal);
            return h.cleanup().await;
        }
        let resume = matches!(case, Case::PerchResume);
        let fish = if resume { PERCH } else { TROUT };

        if !resume {
            add_item(&h, 201, WRONG, 100, 1).await?;
            talk(&mut owner, &mut admitted, npc, "nupi", Some(&[GIGGLE])).await?;
            assert_eq!(carried(&h, WRONG).await?, 100, "wrong item is kept");
            assert_inventory(&h, [0, 0, 0]).await?;
            assert_eq!(quest_state(&h).await?, (None, false, 0));
        }
        add_item(&h, 202, fish, 98, 2).await?;
        talk(&mut owner, &mut admitted, npc, "nupi", Some(&[GIGGLE])).await?;
        assert_eq!(carried(&h, fish).await?, 98, "short hand-in is kept");
        assert_eq!(quest_state(&h).await?, (None, false, 0));

        add_item(&h, 203, fish, if resume { 2 } else { 1 }, 3).await?;
        talk(
            &mut owner,
            &mut admitted,
            npc,
            if resume { "nupi please" } else { "nupi" },
            Some(if resume {
                &[GIGGLE, REWARD]
            } else {
                &[HAND_IN]
            }),
        )
        .await?;
        assert_inventory(&h, [0, 0, 5]).await?;
        assert_eq!(quest_state(&h).await?, (Some(2), true, 1));

        if resume {
            talk(
                &mut owner,
                &mut admitted,
                npc,
                "bye",
                Some(&["Good bye, Fixture Hero."]),
            )
            .await?;
            // Re-admit the cold QuestState while retaining the session's wire command history.
            assert!(
                owner
                    .states
                    .quest_sessions
                    .lock()
                    .map_err(debug)?
                    .remove(&admitted.game_session_id)
                    .is_some()
            );
            owner.admit_quest_session(&admitted).await;
        }

        let reward: &[&str] = if resume { &[GIGGLE, REWARD] } else { &[REWARD] };
        add_item(&h, 204, fish, 100, 4).await?;
        talk(&mut owner, &mut admitted, npc, "nupi", Some(reward)).await?;
        assert_eq!(carried(&h, fish).await?, 0);
        assert_eq!(carried(&h, SHARD).await?, 10);
        add_item(&h, 205, fish, 100, 5).await?;
        talk(&mut owner, &mut admitted, npc, "nupi", Some(reward)).await?;
        assert_eq!(carried(&h, fish).await?, 0);
        assert_eq!(carried(&h, SHARD).await?, 15);
        assert_eq!(quest_state(&h).await?, (Some(2), true, 1));

        drop(seal);
        h.cleanup().await
    })
}

fn shipped_placement() -> TestResult<NpcPlacement> {
    let entry = super::npc_placement_fixtures::published(NPC)?.ok_or("shipped Sinatuki")?;
    let cell = &entry["cell"];
    assert_eq!(
        (
            cell["x"].as_u64(),
            cell["y"].as_u64(),
            cell["floor"].as_u64()
        ),
        (Some(32425), Some(31085), Some(7))
    );
    assert_eq!(entry["direction"], "north");
    Ok(NpcPlacement {
        x: u16::try_from(cell["x"].as_u64().ok_or("placement x")?)?,
        y: u16::try_from(cell["y"].as_u64().ok_or("placement y")?)?,
        floor: -i8::try_from(cell["floor"].as_u64().ok_or("placement floor")?)?,
        direction: MovementFacing::North,
    })
}

async fn open_session(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
    let session = admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            session,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.item_fence = Some(fence.item_fence());
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_NPC_SERVICE_V1, CAPABILITY_WORLD_SPATIAL_ENTITIES],
        )
        .filter(|selected| selected.contains(CAPABILITY_NPC_SERVICE_V1))
        .ok_or("NPC capabilities")?;
    owner.refresh_quest_session(session, true).await;
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
    expected_reply: Option<&[&str]>,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let first = admitted.continuity.next_command_id;
    let last = first.checked_add(1).ok_or("command ID")?;
    // A transport close ends NPC focus. Greet and hand in on the same connection.
    for (command_id, message) in [(first, "hi"), (last, text)] {
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: message.into(),
        })
        .map_err(debug)?;
        let frame = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &frame).await?;
    }
    let serve = async {
        let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let receive = async {
        let mut results = Vec::new();
        let mut lines = Vec::new();
        loop {
            let frame = match read_frame(&mut client).await {
                Ok(frame) => frame,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            };
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::StateDelta => {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == STATE_DOMAIN_NPC_CONVERSATION
                        && delta.delta_type == DELTA_TYPE_NPC_CONVERSATION_V1
                    {
                        lines.extend(decode_npc_conversation(delta.payload).map_err(debug)?.lines);
                    }
                }
                MessageType::CommandResult => {
                    let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        .map_err(debug)?;
                    let disposition = if command.command_id == last && expected_reply.is_none() {
                        NpcIntentDisposition::Rejected
                    } else {
                        NpcIntentDisposition::Ok
                    };
                    assert_eq!(
                        decode_npc_intent_result(command.payload).map_err(debug)?,
                        disposition,
                        "{text}: command {}",
                        command.command_id,
                    );
                    results.push(command.command_id);
                    if command.command_id == last {
                        client.shutdown().await?;
                    }
                }
                _ => {}
            }
        }
        assert_eq!(results, [first, last], "{text}");
        let Some((greeting, reply)) = lines.split_first() else {
            return Err("missing Sinatuki greeting".into());
        };
        assert_eq!(greeting, "Greetings, Fixture Hero.");
        assert_eq!(reply, expected_reply.unwrap_or_default(), "{text}");
        Ok(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serve, receive),
    )
    .await?;
    let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
        return Err(format!("{end:?}").into());
    };
    *admitted = next;
    result
}

async fn seed(h: &Harness, catalogue: &QuestStateCatalogue, tracks: &[(&str, i64)]) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    for &(track, value) in tracks {
        let quest = &catalogue.track(track).ok_or("prerequisite track")?.quest;
        let hash = catalogue
            .definition_hash(quest)
            .ok_or("prerequisite definition")?;
        sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'content-1',$3,2,2) ON CONFLICT(character_id,quest_key) DO NOTHING")
            .bind(id(41).as_slice()).bind(quest).bind(hash.as_slice()).execute(&mut *fixture).await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)")
            .bind(id(41).as_slice()).bind(track).bind(quest).bind(value).execute(&mut *fixture).await?;
    }
    mint_fixture(&mut fixture, 200, "oteryn:item.tibia.i2854", 1).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice())
        .execute(&mut *fixture).await?;
    fixture.commit().await?;
    Ok(())
}

async fn add_item(h: &Harness, seed: u8, item: &str, count: u32, ordinal: i64) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    seed_entry(&mut fixture, seed, item, count, ordinal).await?;
    fixture.commit().await?;
    Ok(())
}

async fn quest_state(h: &Harness) -> TestResult<(Option<i64>, bool, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
         EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$2 AND completed_character_revision IS NOT NULL),
         (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/jamjam')",
    ).bind(STAGE).bind(QUEST).fetch_one(&h.pool).await?)
}

async fn carried(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i
         JOIN game_item_container_entries e USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2",
    ).bind(id(41).as_slice()).bind(item).fetch_one(&h.pool).await?)
}

async fn mint_fixture(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
) -> TestResult {
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(seed, item, "definition-r1", count),
    ))
    .execute(&mut **fixture)
    .await?;
    Ok(())
}

async fn seed_entry(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
    ordinal: i64,
) -> TestResult {
    mint_fixture(fixture, seed, item, count).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice())
        .bind(ordinal).bind(id(246).as_slice()).execute(&mut **fixture).await?;
    Ok(())
}
async fn assert_inventory(h: &Harness, expected: [i64; 3]) -> TestResult {
    for (item, quantity) in [TROUT, PERCH, SHARD].into_iter().zip(expected) {
        assert_eq!(carried(h, item).await?, quantity, "{item}");
    }
    Ok(())
}
