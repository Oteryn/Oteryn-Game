//! Real command-7 Riddler finale after the five ordered source contacts.
//! Relocation between these isolated native scenes is explicit fixture setup.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacements};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const NPC: &str = "oteryn:npc.riddler";
const PREFIX: &str = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/";

const SCENARIO: &str = "qengine-rewards-questpc4-paradox-story-riddler-wire";

pub(super) fn selected(name: &str) -> bool {
    name == SCENARIO
}

#[test]
fn l6_paradox_story_native_riddler_wire_finale() -> TestResult {
    super::l6_paradox_story::run(SCENARIO)
}

pub(super) fn placement() -> NpcPlacements {
    NpcPlacements {
        key: NPC.into(),
        placements: vec![NpcPlacement {
            x: 32479,
            y: 31901,
            floor: -2,
            direction: MovementFacing::South,
        }],
    }
}

pub(super) fn crops() -> TestResult<Vec<super::l6_world_pickups::NativeCrop>> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop};
    let first = super::l6_paradox_story::start();
    Ok(vec![
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z02-x126-y124.b3"
            )),
            region: (2, 126, 124),
            bounds: (32478, 31901, 32480, 31902),
            start: (first.x, first.y),
            expected: [32478, 32479]
                .into_iter()
                .map(|x| ExpectedTile {
                    position: (x, 31901),
                    ids: vec![409, 2576],
                    counted_entry: None,
                })
                .collect(),
        },
        NativeCrop {
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
        },
    ])
}

async fn tracks(h: &Harness) -> TestResult<std::collections::BTreeMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key,value FROM game_character_quest_tracks ORDER BY track_key LIMIT 32",
    )
    .fetch_all(&h.pool)
    .await?;
    Ok(rows.into_iter().collect())
}

async fn value_snapshot(h: &Harness) -> TestResult<serde_json::Value> {
    // This isolated harness owns one character/account and only fixture items.
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'bank',(SELECT jsonb_agg(to_jsonb(b) ORDER BY to_jsonb(b)::text) FROM game_account_bank_balances b),'xp',(SELECT jsonb_agg(to_jsonb(p)-'character_revision' ORDER BY character_id) FROM game_character_progression_state p),'achievements',(SELECT jsonb_agg(to_jsonb(a) ORDER BY achievement_key) FROM game_account_achievements a),'grants',(SELECT jsonb_agg(to_jsonb(g) ORDER BY source_event_id) FROM game_account_achievement_grant_requests g))").fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let before_tracks = tracks(h).await?;
    assert_eq!(
        before_tracks.get(&format!("{PREFIX}the_feared_hugo")),
        Some(&4)
    );
    assert_eq!(before_tracks.get(&format!("{PREFIX}mathemagics")), Some(&1));
    let before_value = value_snapshot(h).await?;
    let teleports: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?;
    let npc = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find_map(|n| {
                (runtime.npc_key(n.actor) == Some(NPC)).then_some(EntityRef {
                    identity: n.actor.placement_identity(),
                    generation: n.generation,
                })
            })
            .ok_or("native Riddler not admitted")?
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    // This is scene setup, not a continuous tower walk.
    super::l6_native_signs::relocate(
        owner,
        actor,
        admitted.game_session_id,
        Position {
            x: 32478,
            y: 31901,
            floor: -2,
        },
    )
    .await?;
    // A new connection snapshot clears focus. Keep the greeting and all answers
    // on one transport, inspecting the topic before sending the next answer.
    let steps = [
        ("hi", None),
        ("test", Some(1)),
        ("yes", Some(2)),
        ("goshnar", Some(3)),
        ("demonbunny", Some(4)),
        ("tha'kull", Some(6)),
        ("yes", Some(7)),
        ("breath", Some(8)),
        ("silence", Some(9)),
        ("old", Some(10)),
        ("yes", Some(11)),
        ("green", Some(12)),
        ("none", Some(13)),
        ("1", None),
    ];
    let words = steps.map(|(word, _)| word);
    let states = owner.states.clone();
    let session = admitted.game_session_id;
    let replies = super::l6_paradox_story::conversation_observed(
        owner,
        admitted,
        npc,
        &words,
        move |index, disposition| {
            let (word, topic) = steps[index];
            assert_eq!(disposition, NpcIntentDisposition::Ok, "Riddler {word}");
            if let Some(topic) = topic {
                assert_eq!(
                    states.npc_conversations.catalogue_topic_for_test(session),
                    Some(topic),
                    "Riddler topic after {word}"
                );
            }
        },
    )
    .await?;
    assert_eq!(replies.len(), steps.len());
    let placed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        placed.position(),
        super::quest_world_callbacks::local(Position {
            x: 32478,
            y: 31905,
            floor: -1
        })
    );
    assert!(!owner.states.newhaven_travel.held(admitted.game_session_id));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_pending_arrivals")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_quest_npc_teleport_receipts")
            .fetch_one(&h.pool)
            .await?,
        teleports + 1
    );
    let mut expected_tracks = before_tracks;
    expected_tracks.insert(format!("{PREFIX}favorite_colour"), 2);
    expected_tracks.insert(format!("{PREFIX}mathemagics"), 2);
    assert_eq!(tracks(h).await?, expected_tracks);
    assert_eq!(
        value_snapshot(h).await?,
        before_value,
        "Riddler grants no extra items, bank value, XP or achievement"
    );
    eprintln!("PARADOX_RIDDLER_WIRE_COMPLETED");
    Ok(())
}
