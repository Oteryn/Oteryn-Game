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
    Contact {
        key: "oteryn:npc.riddler",
        at: Position {
            x: 32479,
            y: 31901,
            floor: -2,
        },
        ground: 409,
        approach_ground: 409,
    },
];

fn approach(c: Contact) -> Position {
    Position {
        x: if c.key.ends_with(".riddler") { c.at.x + 2 } else { c.at.x - 1 },
        ..c.at
    }
}

fn contacts(riddler: bool) -> &'static [Contact] {
    if riddler { CONTACTS } else { &CONTACTS[..5] }
}

pub(super) fn start() -> Position {
    approach(CONTACTS[0])
}

pub(super) fn world(native: &NativeGameplayState, riddler: bool) -> TestResult<crate::map::boot::BundleWorld> {
    super::l6_world_pickups_fixtures::native_scene(native, crops(riddler)?, &[])
}

pub(super) fn crops(riddler: bool) -> TestResult<Vec<super::l6_world_pickups_fixtures::NativeCrop>> {
    use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop};
    let bytes: [&'static [u8]; 6] = [
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
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z02-x126-y124.b3"
        )),
    ];
    let mut crops = Vec::new();
    let first = start();
    for (i, c) in contacts(riddler).iter().copied().enumerate() {
        for (p, id) in [(approach(c), c.approach_ground), (c.at, c.ground)] {
            crops.push(NativeCrop {
                bytes: bytes[i],
                region: (u8::try_from(-p.floor)?, p.x / 256, p.y / 256),
                bounds: (p.x, p.y, p.x + 1, p.y + 1),
                start: (first.x, first.y),
                expected: vec![ExpectedTile {
                    position: (p.x, p.y),
                    ids: if c.key.ends_with(".riddler") && p == c.at {
                        vec![id, 2576]
                    } else {
                        vec![id]
                    },
                    counted_entry: None,
                }],
            });
        }
    }
    if !riddler {
        return Ok(crops);
    }
    crops.push(NativeCrop {
        bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z01-x126-y124.b3"
        )),
        region: (1, 126, 124),
        bounds: (32478, 31905, 32479, 31906),
        start: (first.x, first.y),
        expected: vec![ExpectedTile {
            position: (32478, 31905),
            ids: vec![409],
            counted_entry: None,
        }],
    });
    Ok(crops)
}

async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    production: bool,
    contacts: &[Contact],
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
    let native_npcs: Vec<_> = if production {
        vec!["oteryn:npc.riddler"]
    } else {
        contacts.iter().map(|contact| contact.key).collect()
    };
    owner.states.chat.install_npc_talk(Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &native_npcs),
    ));
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
    let mut npcs: Vec<_> = contacts
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
    let table = if production {
        super::paradox_tower_journey_fixtures::placements()?
    } else {
        NpcPlacementTable { npcs }
    };
    runtime
        .place_npcs(
            &table,
            |key| allowed.contains(key),
            true,
        )
        .map_err(debug)?;
    contacts
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

pub(super) struct Reply {
    id: u64,
    payload: Vec<u8>,
    body: Vec<u8>,
    pub(super) disposition: NpcIntentDisposition,
}

pub(super) async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
) -> TestResult<Vec<Reply>> {
    conversation_with_checkpoint(owner, admitted, npc, words, async |_| Ok(())).await
}

pub(super) async fn conversation_with_checkpoint(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
    mut checkpoint: impl AsyncFnMut(usize) -> TestResult,
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
    let reader = async move {
        async {
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
                        super::qengine_rewards_fixtures::acknowledge_probe(
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
                checkpoint(index + 1).await?;
            }
            client.shutdown().await?;
            let mut drain = Vec::new();
            client.read_to_end(&mut drain).await?;
            Ok(replies)
        }
        .await
    };
    let active = *admitted;
    let admission = &*owner;
    let server = async move { serve_admitted(&mut server, active, admission, IDLE_LIVENESS).await };
    let (end, replies): (_, TestResult<Vec<Reply>>) = tokio::join!(server, reader);
    let replies = replies?;
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
    super::l6_native_signs_fixtures::relocate(
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
    let journey = name == super::paradox_tower_journey_fixtures::NAME;
    let riddler = name.ends_with("-riddler");
    let npcs = install(owner, admitted, journey, contacts(riddler)).await?;
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
    super::l6_paradox_fixtures::raise_level(owner, admitted, authority, h, 30).await?;
    let carried = if journey {
        Some(super::paradox_tower_journey_fixtures::carried(owner, admitted, h).await?)
    } else {
        None
    };
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
    if let Some(before) = &carried {
        assert_eq!(
            super::paradox_tower_journey_fixtures::carried(owner, admitted, h).await?,
            *before
        );
    }
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
        if let Some(before) = &carried {
            assert_eq!(
                super::paradox_tower_journey_fixtures::carried(owner, admitted, h).await?,
                *before
            );
        }
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
    if let Some(before) = &carried {
        assert_eq!(
            super::paradox_tower_journey_fixtures::carried(owner, admitted, h).await?,
            *before
        );
    }
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
        super::quest_world_callbacks_fixtures::wire(owner, admitted, last.id, 7, last.payload.clone())
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
    if riddler {
        relocate(owner, admitted, 5).await?;
        let replies = conversation(
            owner, admitted, npcs[5],
            &["hi", "test", "yes", "goshnar", "demonbunny", "tha'kull", "yes",
              "breath", "silence", "old", "yes", "green", "none", "1"],
        ).await?;
        assert!(replies.iter().all(|r| r.disposition == NpcIntentDisposition::Ok));
        assert_eq!(
            owner.runtime.lock().await
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?.position(),
            super::quest_world_callbacks_fixtures::local(Position { x: 32478, y: 31905, floor: -1 }),
        );
        let values: Vec<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE track_key IN ($1,$2) ORDER BY track_key",
        )
        .bind("oteryn:quest-progress/quest/u7_24/the_paradox_tower/favorite_colour")
        .bind("oteryn:quest-progress/quest/u7_24/the_paradox_tower/mathemagics")
        .fetch_all(&h.pool).await?;
        assert_eq!(values, [2, 2]);
    }
    Ok(())
}
