//! Selected ordered contacts at original native NPC centers; no SQL progress seeds.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::content::{
    NpcDataCatalogue, native_gameplay::NativeGameplayState, npc_catalogue_preproduction_limits,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{
    MessageType, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
    decode_wire_envelope,
};
use crate::gameplay_transport::{
    connection::{ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted},
    npc_talk::NpcTalkBook,
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;
const STORY: &str = "oteryn:quest-progress/l6/paradox/story";
const ACHIEVEMENT: &str = "oteryn:achievement/mathemagician";
#[derive(Clone, Copy)]
struct Contact {
    key: &'static str,
    at: Position,
    ground: u32,
    approach_ground: u32,
}
const CONTACTS: &[Contact] = &[
    Contact {
        key: "oteryn:npc.oldrak",
        at: Position {
            x: 32817,
            y: 32261,
            floor: -6,
        },
        ground: 409,
        approach_ground: 410,
    },
    Contact {
        key: "oteryn:npc.zoltan",
        at: Position {
            x: 33266,
            y: 31847,
            floor: -4,
        },
        ground: 409,
        approach_ground: 409,
    },
    Contact {
        key: "oteryn:npc.padreia",
        at: Position {
            x: 32302,
            y: 31814,
            floor: -7,
        },
        ground: 454,
        approach_ground: 454,
    },
    Contact {
        key: "oteryn:npc.lubo",
        at: Position {
            x: 32489,
            y: 32119,
            floor: -7,
        },
        ground: 408,
        approach_ground: 408,
    },
    Contact {
        key: "oteryn:npc.a_prisoner",
        at: Position {
            x: 32394,
            y: 32138,
            floor: -13,
        },
        ground: 408,
        approach_ground: 408,
    },
];
fn approach(c: Contact) -> Position {
    Position {
        x: c.at.x - 1,
        ..c.at
    }
}
pub(super) fn start() -> Position {
    approach(CONTACTS[0])
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::qengine_rewards::scenario(name)
}
#[test]
fn l6_paradox_story_native_ordered() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-ordered")
}
#[test]
fn l6_paradox_story_native_out_of_order() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-out-of-order")
}
#[test]
fn l6_paradox_story_native_lesson_guard_and_replay() -> TestResult {
    run("qengine-rewards-questpc4-paradox-story-lesson-guard")
}
pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    super::l6_world_pickups::native_scene(native, crops()?, &[])
}
pub(super) fn crops() -> TestResult<Vec<super::l6_world_pickups::NativeCrop>> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop};
    let bytes: [&'static [u8]; 5] = [
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z06-x128-y126.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z04-x129-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x126-y124.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z07-x126-y125.b3"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z13-x126-y125.b3"
        )),
    ];
    let mut crops = Vec::new();
    let first = start();
    for (i, c) in CONTACTS.iter().copied().enumerate() {
        for (p, id) in [(approach(c), c.approach_ground), (c.at, c.ground)] {
            crops.push(NativeCrop {
                bytes: bytes[i],
                region: (u8::try_from(-p.floor)?, p.x / 256, p.y / 256),
                bounds: (p.x, p.y, p.x + 1, p.y + 1),
                start: (first.x, first.y),
                expected: vec![ExpectedTile {
                    position: (p.x, p.y),
                    ids: vec![id],
                    counted_entry: None,
                }],
            });
        }
    }
    Ok(crops)
}
async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Vec<EntityRef>> {
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let allowed: std::collections::BTreeSet<_> =
        model.npcs().keys().map(|r| r.key.clone()).collect();
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Paradox pilgrim",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    runtime
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut npcs: Vec<_> = CONTACTS
        .iter()
        .map(|c| NpcPlacements {
            key: c.key.into(),
            placements: vec![NpcPlacement {
                x: c.at.x,
                y: c.at.y,
                floor: c.at.floor,
                direction: MovementFacing::South,
            }],
        })
        .collect();
    npcs.sort_by(|a, b| a.key.cmp(&b.key));
    runtime
        .place_npcs(
            &NpcPlacementTable { npcs },
            |key| allowed.contains(key),
            true,
        )
        .map_err(debug)?;
    CONTACTS
        .iter()
        .map(|c| {
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|n| {
                    (runtime.npc_key(n.actor) == Some(c.key)).then_some(EntityRef {
                        identity: n.actor.placement_identity(),
                        generation: n.generation,
                    })
                })
                .ok_or_else(|| "native source NPC not admitted".into())
        })
        .collect()
}
struct Reply {
    id: u64,
    payload: Vec<u8>,
    body: Vec<u8>,
    disposition: NpcIntentDisposition,
}
async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
) -> TestResult<Vec<Reply>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("talk clock")?;
    let first = admitted.continuity.next_command_id;
    let commands = words
        .iter()
        .enumerate()
        .map(|(i, text)| {
            Ok((
                first + u64::try_from(i)?,
                encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: (*text).into(),
                })
                .map_err(debug)?,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let frames = commands
        .iter()
        .map(|(id, payload)| {
            oteryn_protocol_oteryn::encode_client_command(
                admitted.continuity.connection_generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: *id,
                    command_type: 7,
                    payload,
                },
            )
            .map_err(debug)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let generation = admitted.continuity.connection_generation;
    let labels = words.iter().map(|w| (*w).to_owned()).collect::<Vec<_>>();
    let reader = tokio::spawn(async move {
        let result: TestResult<Vec<Reply>> = async {
            let mut replies = Vec::new();
            let mut window = tokio::time::Instant::now();
            for (index, frame) in frames.iter().enumerate() {
                // NPC0-RL-04: use the same four-message window as wire_reward.
                if index != 0 && index % 4 == 0 {
                    tokio::time::sleep_until(window + std::time::Duration::from_secs(1)).await;
                    window = tokio::time::Instant::now();
                }
                client
                    .write_all(&u32::try_from(frame.len())?.to_be_bytes())
                    .await?;
                client.write_all(frame).await?;
                loop {
                    let mut prefix = [0; 4];
                    client.read_exact(&mut prefix).await?;
                    let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
                    client.read_exact(&mut frame).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == MessageType::LivenessProbe {
                        super::qengine_rewards::acknowledge_probe(
                            &mut client,
                            generation,
                            envelope.payload(),
                        )
                        .await?;
                    }
                    if envelope.message_type() == MessageType::CommandResult {
                        let command =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                                .map_err(debug)?;
                        let expected = &commands[replies.len()];
                        assert_eq!(command.command_id, expected.0);
                        eprintln!(
                            "PARADOX_STORY_REPLY word={:?} result={:?}",
                            labels[replies.len()],
                            decode_npc_intent_result(command.payload).map_err(debug)?
                        );
                        replies.push(Reply {
                            id: command.command_id,
                            payload: expected.1.clone(),
                            body: command.payload.to_vec(),
                            disposition: decode_npc_intent_result(command.payload)
                                .map_err(debug)?,
                        });
                        break;
                    }
                }
                if (index + 1) % 4 == 0 {
                    window = tokio::time::Instant::now();
                }
            }
            client.shutdown().await?;
            let mut drain = Vec::new();
            client.read_to_end(&mut drain).await?;
            Ok(replies)
        }
        .await;
        result.map_err(|e| e.to_string())
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let replies = reader.await?.map_err(std::io::Error::other)?;
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    } else {
        return Err("NPC conversation did not retain admitted continuity".into());
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(replies)
}
async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    index: usize,
) -> TestResult {
    super::l6_native_signs::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        approach(CONTACTS[index]),
    )
    .await
}
async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1)",
    )
    .bind(STORY)
    .fetch_one(&h.pool)
    .await?)
}
async fn snapshot(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('tracks',(SELECT jsonb_agg(to_jsonb(t) ORDER BY track_key) FROM game_character_quest_tracks t),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY committed_character_revision) FROM game_character_quest_receipts r),'achievements',(SELECT jsonb_agg(to_jsonb(a) ORDER BY achievement_key) FROM game_account_achievements a),'grants',(SELECT jsonb_agg(to_jsonb(g) ORDER BY source_event_id) FROM game_account_achievement_grant_requests g))").fetch_one(&h.pool).await?)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_paradox_story version={version}");
    let npcs = install(owner, admitted).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    super::l6_paradox::raise_level(owner, admitted, authority, h, 30).await?;
    let initial = snapshot(h).await?;
    if name.ends_with("out-of-order") {
        relocate(owner, admitted, 1).await?;
        conversation(owner, admitted, npcs[1], &["hi", "crunor's caress"]).await?;
        assert_eq!(
            snapshot(h).await?,
            initial,
            "later NPC cannot begin the protected story"
        );
    }
    relocate(owner, admitted, 0).await?;
    let replies = conversation(owner, admitted, npcs[0], &["hi", "hugo"]).await?;
    assert!(
        replies
            .iter()
            .all(|r| r.disposition == NpcIntentDisposition::Ok)
    );
    assert_eq!(progress(h).await?, 1);
    if name.ends_with("lesson-guard") {
        let before = snapshot(h).await?;
        relocate(owner, admitted, 4).await?;
        conversation(
            owner,
            admitted,
            npcs[4],
            &["hi", "math", "yes", "green", "yes"],
        )
        .await?;
        assert_eq!(
            snapshot(h).await?,
            before,
            "the selected lesson cannot skip the ordered contacts"
        );
    }
    for (index, word, expected) in [
        (1, "crunor's caress", 2),
        (2, "footnote", 3),
        (3, "stable", 4),
    ] {
        relocate(owner, admitted, index).await?;
        let replies = conversation(owner, admitted, npcs[index], &["hi", word]).await?;
        assert!(
            replies
                .iter()
                .all(|r| r.disposition == NpcIntentDisposition::Ok),
            "selected contact word {word}"
        );
        assert_eq!(
            progress(h).await?,
            expected,
            "only the exact ordered contact advances the story"
        );
    }
    let before = snapshot(h).await?;
    relocate(owner, admitted, 1).await?;
    conversation(owner, admitted, npcs[1], &["hi", "crunor's caress"]).await?;
    assert_eq!(
        snapshot(h).await?,
        before,
        "revisiting an earlier contact cannot reset progress or mint another receipt"
    );
    relocate(owner, admitted, 4).await?;
    conversation(owner, admitted, npcs[4], &["hi", "yes"]).await?;
    assert_eq!(
        snapshot(h).await?,
        before,
        "yes without the source lesson topic grants nothing"
    );
    let lesson = conversation(
        owner,
        admitted,
        npcs[4],
        &["hi", "math", "yes", "green", "yes"],
    )
    .await?;
    eprintln!("PARADOX_STORY_AFTER_LESSON progress={}", progress(h).await?);
    assert!(
        lesson
            .iter()
            .all(|r| r.disposition == NpcIntentDisposition::Ok),
        "lesson dispositions {:?}",
        lesson
            .iter()
            .map(|r| (r.id, r.disposition))
            .collect::<Vec<_>>()
    );
    assert_eq!(progress(h).await?, 5);
    let achievements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?;
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        (achievements, grants),
        (1, 1),
        "lesson and account achievement commit exactly once"
    );
    let committed = snapshot(h).await?;
    let last = lesson.last().ok_or("lesson reply")?;
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, last.id, 7, last.payload.clone())
            .await?,
        last.body
    );
    assert_eq!(
        snapshot(h).await?,
        committed,
        "original command replay repeats no story or achievement write"
    );
    conversation(
        owner,
        admitted,
        npcs[4],
        &["hi", "math", "yes", "green", "yes"],
    )
    .await?;
    assert_eq!(
        snapshot(h).await?,
        committed,
        "a fresh repeated lesson cannot grant another achievement source"
    );
    Ok(())
}
