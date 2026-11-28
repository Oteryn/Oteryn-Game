//! The selected Farmine settlement, with the source objects and native quest owners.
use super::fixture::{Harness, debug, id};
use super::m39_bone_journey_fixtures::carried;
use super::quest_world_callbacks_fixtures::{local, wire};
use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::content::native_gameplay::NativeGameplayState;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const NAME: &str = "authored-new-frontier";
pub(super) const BAIT_RECOVERY: &str = "authored-new-frontier-bait-recovery";
pub(super) const POLISHER_RETRY: &str = "authored-new-frontier-polisher-retry";
const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
const BOSS: &str = "oteryn:creature.shard_of_corruption";
const SCOUT: Position = pos(33082, 31532, 7);
const TREES: [Position; 3] = [
    pos(32515, 31927, 7),
    pos(32474, 31947, 7),
    pos(32458, 31997, 7),
];
const CONTACTS: [(&str, Position, Position); 8] = [
    ("ongulf", pos(33024, 31539, 12), pos(33024, 31538, 12)),
    ("melfar", pos(32522, 31966, 7), pos(32521, 31966, 7)),
    ("lazaran", pos(32992, 31471, 3), pos(32992, 31472, 3)),
    ("humgolf", pos(32598, 31880, 9), pos(32598, 31881, 9)),
    ("wyrdin", pos(33266, 31835, 6), pos(33266, 31836, 6)),
    ("angus", pos(32670, 32730, 7), pos(32670, 32729, 7)),
    ("telas", pos(33276, 31800, 6), pos(33276, 31802, 6)),
    ("king_tibianus", pos(32311, 32172, 6), pos(32311, 32173, 6)),
];
const CITY: Position = pos(33061, 31527, 12);
const ZAO: Position = pos(32993, 31547, 4);
const MINE_PLATFORM: Position = pos(33055, 31527, 12);
const MINE_EXIT: Position = pos(33065, 31489, 15);
const BOSS_CELL: Position = pos(33072, 31434, 15);
const POTIONS: Position = pos(33057, 31029, 7);

const fn pos(x: u16, y: u16, z: i8) -> Position {
    Position { x, y, floor: -z }
}

async fn track(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(
        super::m39_bone_journey_fixtures::stage(h, &format!("{TRACK}{suffix}"))
            .await?
            .unwrap_or(0),
    )
}

async fn state(h: &Harness, phase: i64, stage: (&str, i64), bait: i64) -> TestResult {
    assert_eq!(track(h, "questline").await?, phase);
    assert_eq!(track(h, stage.0).await?, stage.1, "{}", stage.0);
    assert_eq!(carried(h, 9843).await?, bait);
    assert_eq!(
        track(h, "mission08").await?,
        0,
        "later source trial is distinct"
    );
    Ok(())
}

async fn travel_to(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    p: Position,
) -> TestResult {
    // Move between remote source crops using a fixture transport without any Quest effects.
    owner.refresh_quest_session(a.game_session_id, false).await;
    let map = owner.bundle_map().ok_or("native world map")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = a.runtime_actor.ok_or("player")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        a.game_session_id,
        before,
        local(p),
        false,
        &doors,
    )
    .map_err(|error| format!("travel to {p:?}: {error:?}"))?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    p: Position,
) -> TestResult {
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("player")?)
            .map_err(debug)?
            .position(),
        local(p)
    );
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
    expected: NpcIntentDisposition,
) -> TestResult<u64> {
    owner.refresh_quest_session(a.game_session_id, false).await;
    let commands = words
        .iter()
        .enumerate()
        .map(|(i, text)| Ok((a.continuity.next_command_id + u64::try_from(i)?, *text)))
        .collect::<TestResult<Vec<_>>>()?;
    let last = commands.last().ok_or("conversation")?.0;
    if expected == NpcIntentDisposition::Ok {
        super::grizzly_npc_fixtures::wire::talk(owner, a, npc, &commands).await?;
    } else {
        super::grizzly_npc_fixtures::wire::talk_refused(owner, a, npc, &commands).await?;
    }
    Ok(last)
}

fn tiles(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    p: Position,
    ordinal: u8,
    expected: UseDisposition,
) -> TestResult {
    owner.refresh_quest_session(a.game_session_id, false).await;
    let target = object_target(owner, tiles(p), ordinal)?;
    assert_eq!(
        use_target(owner, a, a.continuity.next_command_id, &target).await?,
        expected,
        "{p:?}"
    );
    Ok(())
}

async fn bait_tree(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    p: Position,
    expected: UseDisposition,
) -> TestResult {
    travel_to(owner, a, Position { y: p.y + 1, ..p }).await?;
    let source = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i9843", "definition-r1"))
        .ok_or("native bait Item")?;
    let target = object_target(owner, tiles(p), 1)?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
    })
    .map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let (resumed, outcome, _) = super::rust_lottery_fixtures::exchange(
        owner,
        *a,
        a.continuity.next_command_id,
        &payload,
        Some(source),
        None,
    )
    .await?;
    *a = resumed;
    assert_eq!(outcome, expected);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use NpcIntentDisposition::{Ok as Spoken, Rejected};
    use UseDisposition::{Committed, Rejected as UseRejected};
    owner.admit_premium(id(40), a.game_session_id);
    assert!(
        owner
            .current_quest_fence(a.game_session_id)
            .await
            .map_err(debug)?
            .is_some()
    );
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 13, 15, 16, 17, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Frontier capabilities")?;

    let names = CONTACTS.map(|(name, _, _)| format!("oteryn:npc.{name}"));
    let [ongulf, melfar, lazaran, humgolf, wyrdin, angus, telas, king] =
        super::classic_quests_fixtures::install_positions(
            owner,
            a,
            names.each_ref().map(String::as_str),
            CONTACTS.map(|(_, place, _)| place),
        )
        .await?;
    state(h, 0, ("mission01", 0), 0).await?;
    let xp = experience(h).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    say(
        owner,
        a,
        ongulf,
        &["hi", "project", "short", "mission"],
        Spoken,
    )
    .await?;
    state(h, 1, ("mission01", 1), 0).await?;
    travel_to(
        owner,
        a,
        Position {
            x: SCOUT.x - 1,
            ..SCOUT
        },
    )
    .await?;
    step(owner, a, a.continuity.next_command_id, StepDirection::East).await?;
    state(h, 2, ("mission01", 2), 0).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 3, ("mission01", 3), 0).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 4, ("mission02_1", 1), 0).await?;

    travel_to(owner, a, CONTACTS[1].2).await?;
    say(owner, a, melfar, &["hi", "mission", "yes"], Spoken).await?;
    state(h, 5, ("mission02_1", 2), 1).await?;
    if name == BAIT_RECOVERY {
        bait_tree(owner, a, TREES[2], Committed).await?;
        state(h, 5, ("mission02/beaver3", 1), 0).await?;
        let fence = owner
            .current_quest_fence(a.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("bait purchase fence")?;
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: CONTACTS[1].2,
                item: "oteryn:item.tibia.i3035".into(),
                count: 1,
            },
            90_010,
            true,
        )
        .await?;
        travel_to(owner, a, CONTACTS[1].2).await?;
        say(owner, a, melfar, &["hi", "flask", "yes"], Spoken).await?;
        state(h, 5, ("mission02_1", 2), 1).await?;
        assert_eq!(carried(h, 3035).await?, 0);
        for tree in [TREES[0], TREES[1]] {
            bait_tree(owner, a, tree, Committed).await?;
        }
        travel_to(owner, a, CONTACTS[1].2).await?;
        say(owner, a, melfar, &["hi", "mission"], Spoken).await?;
        return state(h, 6, ("mission02_1", 3), 1).await;
    }
    for (n, p) in TREES.into_iter().enumerate() {
        bait_tree(owner, a, p, Committed).await?;
        state(
            h,
            5,
            (&format!("mission02/beaver{}", n + 1), 1),
            i64::from(n < 2),
        )
        .await?;
        if n == 0 {
            bait_tree(owner, a, p, UseRejected).await?;
            state(h, 5, ("mission02/beaver2", 0), 1).await?;
        }
    }
    travel_to(owner, a, CONTACTS[1].2).await?;
    say(owner, a, melfar, &["hi", "mission"], Spoken).await?;
    state(h, 6, ("mission02_1", 3), 0).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 7, ("mission02_1", 4), 0).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 8, ("mission03", 1), 0).await?;

    travel_to(owner, a, CONTACTS[2].2).await?;
    say(owner, a, lazaran, &["hi", "mission", "peace"], Spoken).await?;
    state(h, 9, ("mission03", 2), 0).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 10, ("mission03", 3), 0).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 11, ("mission04", 1), 0).await?;
    travel_to(owner, a, MINE_PLATFORM).await?;
    use_object(owner, a, pos(33056, 31527, 12), 2, Committed).await?;
    at(owner, a, MINE_EXIT).await?;
    use_object(owner, a, pos(33064, 31489, 15), 2, Committed).await?;
    at(owner, a, MINE_EXIT).await?;
    use_object(owner, a, pos(33064, 31489, 15), 2, Committed).await?;
    at(owner, a, MINE_PLATFORM).await?;
    state(h, 11, ("mine_leader", 0), 0).await?;
    use_object(owner, a, pos(33056, 31527, 12), 2, Committed).await?;
    at(owner, a, MINE_PLATFORM).await?;
    use_object(owner, a, pos(33056, 31527, 12), 2, Committed).await?;
    at(owner, a, MINE_EXIT).await?;
    travel_to(
        owner,
        a,
        Position {
            x: BOSS_CELL.x - 1,
            ..BOSS_CELL
        },
    )
    .await?;
    defeat_mine_leader(owner, a, h).await?;
    state(h, 12, ("mine_leader", 1), 0).await?;
    travel_to(owner, a, MINE_EXIT).await?;
    use_object(owner, a, pos(33064, 31489, 15), 2, Committed).await?;
    at(owner, a, MINE_EXIT).await?;
    use_object(owner, a, pos(33064, 31489, 15), 2, Committed).await?;
    at(owner, a, MINE_PLATFORM).await?;
    state(h, 12, ("mine_leader", 1), 0).await?;

    travel_to(owner, a, CONTACTS[0].2).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 13, ("mission04", 2), 0).await?;
    say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 14, ("mission05_1", 1), 0).await?;
    say(owner, a, ongulf, &["hi", "mission"], Rejected).await?;
    state(h, 14, ("mission05_1", 1), 0).await?;
    for (npc, (name, _, approach)) in [humgolf, wyrdin, angus, telas]
        .into_iter()
        .zip(CONTACTS.into_iter().skip(3))
    {
        travel_to(owner, a, approach).await?;
        say(owner, a, npc, &["hi", "farmine", "reason"], Spoken).await?;
        state(h, 14, (&format!("mission05/{name}"), 3), 0).await?;
    }
    let king_approach = CONTACTS[7].2;
    travel_to(owner, a, king_approach).await?;
    assert_eq!(carried(h, 10009).await?, 0);
    if name == POLISHER_RETRY {
        say(owner, a, king, &["Hail King!", "farmine", "plea"], Spoken).await?;
        state(h, 14, ("mission05/king_tibianus", 2), 0).await?;
        say(owner, a, king, &["Hail King!", "farmine", "yes"], Spoken).await?;
        state(h, 14, ("mission05/king_tibianus", 2), 0).await?;
    }
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("crown polisher item fence")?;
    for ordinal in [90_011, 90_012] {
        super::quest_world_callbacks_fixtures::input_item(
            owner,
            fence.item_fence(),
            &oteryn_gameplay_quests::quest::world_callbacks::Offering {
                position: king_approach,
                item: "oteryn:item.tibia.i10009".into(),
                count: 1,
            },
            ordinal,
            true,
        )
        .await?;
    }
    assert_eq!(carried(h, 10009).await?, 2);
    if name == POLISHER_RETRY {
        say(owner, a, king, &["Hail King!", "farmine", "yes"], Spoken).await?;
        state(h, 14, ("mission05/king_tibianus_prepaid", 1), 0).await?;
        assert_eq!(carried(h, 10009).await?, 1);
        say(owner, a, king, &["Hail King!", "farmine", "plea"], Spoken).await?;
        assert_eq!(carried(h, 10009).await?, 1);
    } else {
        say(owner, a, king, &["Hail King!", "farmine", "reason"], Spoken).await?;
        assert_eq!(carried(h, 10009).await?, 2);
    }
    state(h, 14, ("mission05/king_tibianus", 3), 0).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    travel_to(owner, a, CITY).await?;
    use_object(owner, a, pos(33062, 31527, 12), 2, UseRejected).await?;
    at(owner, a, CITY).await?;
    travel_to(owner, a, CONTACTS[0].2).await?;
    let before_reward = experience(h).await?;
    assert!(before_reward >= xp);
    let terminal = say(owner, a, ongulf, &["hi", "mission"], Spoken).await?;
    state(h, 15, ("mission05_1", 2), 0).await?;
    assert_eq!(experience(h).await?, before_reward + 8000);
    assert_eq!(terminal_receipts(h).await?, (1, 1));

    // Every wire exchange closes its transport. The reconnect replays the exact command.
    let payload = encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: ongulf,
        text: "mission".into(),
    })
    .map_err(debug)?;
    let replay = wire(owner, a, terminal, 7, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay).map_err(debug)?,
        Spoken
    );
    say(owner, a, ongulf, &["hi", "mission"], Rejected).await?;
    assert_eq!(experience(h).await?, before_reward + 8000);
    assert_eq!(terminal_receipts(h).await?, (1, 1));
    state(h, 15, ("mission05_1", 2), 0).await?;

    travel_to(owner, a, CITY).await?;
    use_object(owner, a, pos(33062, 31527, 12), 2, Committed).await?;
    at(owner, a, ZAO).await?;
    use_object(owner, a, pos(32994, 31547, 4), 2, Committed).await?;
    at(owner, a, CITY).await?;

    travel_to(
        owner,
        a,
        Position {
            y: POTIONS.y - 1,
            ..POTIONS
        },
    )
    .await?;
    for item in [7439, 7440, 7443] {
        assert_eq!(carried(h, item).await?, 0);
    }
    use_object(owner, a, POTIONS, 1, Committed).await?;
    for item in [7439, 7440, 7443] {
        assert_eq!(carried(h, item).await?, 1);
    }
    use_object(owner, a, POTIONS, 1, UseDisposition::NothingToUse).await?;
    move_potion(owner, a, h).await?;
    for item in [7439, 7440, 7443] {
        assert_eq!(carried(h, item).await?, 1);
    }
    state(h, 15, ("mission05_1", 2), 0).await?;
    assert_log(owner, a).await?;
    Ok(())
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn terminal_receipts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes),         (SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key='oteryn:outfit.warmaster/quest-grant/0')")
        .bind(QUEST).fetch_one(&h.pool).await?)
}

async fn assert_log(owner: &ComposedFreshAdmission<'_, '_, '_>, a: &AdmittedSession) -> TestResult {
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    use oteryn_protocol_oteryn::{MessageType, decode_state_delta, decode_wire_envelope};
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let log = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&catalogue)?;
    let quest_index = u32::try_from(
        log.content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == QUEST)
            .ok_or("Frontier journal entry")?,
    )?;
    let payload = encode_quest_log_query(&QuestLogQuery::List).map_err(debug)?;
    let command = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: a.continuity.next_command_id,
            command_type: 22,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(command.len())?.to_be_bytes().to_vec();
    input.extend(command);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(owner, *a, &input).await?;
    let mut output = output.as_slice();
    let mut complete = false;
    while !output.is_empty() {
        let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut output).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == 16 {
                complete |= matches!(decode_quest_log(delta.payload).map_err(debug)?.view,
                    Some(QuestLogView::List(entries)) if entries.iter().any(|entry|
                        entry.quest_index == quest_index && entry.completed));
            }
        }
    }
    assert!(
        complete,
        "the wire journal must list the completed settlement"
    );
    Ok(())
}

pub(super) fn start() -> Position {
    CONTACTS[0].2
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{
        ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
    macro_rules! crop {
        ($file:literal, $region:expr, $bounds:expr, $start:expr, $expected:expr) => {
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/",
                    $file
                )),
                region: $region,
                bounds: $bounds,
                start: $start,
                expected: $expected,
            }
        };
    }
    let object = |x, y, ids| ExpectedTile {
        position: (x, y),
        ids,
        counted_entry: None,
    };
    // Half-open source rectangles keep only the encounter objects and walkable approaches.
    let crops = vec![
        crop!(
            "region-z12-x129-y123.b3",
            (12, 129, 123),
            (33024, 31538, 33025, 31540),
            (33024, 31538),
            vec![]
        ),
        crop!(
            "region-z12-x129-y123.b3",
            (12, 129, 123),
            (33055, 31527, 33057, 31528),
            (33055, 31527),
            vec![object(33056, 31527, vec![429, 2317, 2772])]
        ),
        crop!(
            "region-z12-x129-y123.b3",
            (12, 129, 123),
            (33061, 31527, 33063, 31528),
            (33061, 31527),
            vec![object(33062, 31527, vec![429, 2317, 2772])]
        ),
        crop!(
            "region-z04-x128-y123.b3",
            (4, 128, 123),
            (32993, 31547, 32995, 31548),
            (32993, 31547),
            vec![object(32994, 31547, vec![429, 2317, 2772])]
        ),
        crop!(
            "region-z07-x129-y123.b3",
            (7, 129, 123),
            (33081, 31532, 33083, 31533),
            (33081, 31532),
            vec![object(33082, 31532, vec![353, 1953])]
        ),
        crop!(
            "region-z07-x127-y124.b3",
            (7, 127, 124),
            (32521, 31966, 32523, 31967),
            (32521, 31966),
            vec![]
        ),
        crop!(
            "region-z07-x127-y124.b3",
            (7, 127, 124),
            (32515, 31927, 32516, 31929),
            (32515, 31928),
            vec![object(32515, 31927, vec![4515, 3625])]
        ),
        crop!(
            "region-z07-x126-y124.b3",
            (7, 126, 124),
            (32474, 31947, 32475, 31949),
            (32474, 31948),
            vec![object(32474, 31947, vec![4517, 3625])]
        ),
        crop!(
            "region-z07-x126-y124.b3",
            (7, 126, 124),
            (32458, 31997, 32459, 31999),
            (32458, 31998),
            vec![object(32458, 31997, vec![4517, 3625])]
        ),
        crop!(
            "region-z03-x128-y122.b3",
            (3, 128, 122),
            (32992, 31471, 32993, 31473),
            (32992, 31472),
            vec![]
        ),
        crop!(
            "region-z15-x129-y123.b3",
            (15, 129, 123),
            (33064, 31489, 33066, 31490),
            (33065, 31489),
            vec![]
        ),
        crop!(
            "region-z15-x129-y122.b3",
            (15, 129, 122),
            (33071, 31434, 33073, 31436),
            (33071, 31434),
            vec![]
        ),
        crop!(
            "region-z09-x127-y124.b3",
            (9, 127, 124),
            (32598, 31880, 32599, 31882),
            (32598, 31881),
            vec![]
        ),
        crop!(
            "region-z06-x129-y124.b3",
            (6, 129, 124),
            (33266, 31835, 33267, 31837),
            (33266, 31836),
            vec![]
        ),
        crop!(
            "region-z07-x127-y127.b3",
            (7, 127, 127),
            (32670, 32729, 32671, 32731),
            (32670, 32729),
            vec![]
        ),
        crop!(
            "region-z06-x129-y124.b3",
            (6, 129, 124),
            (33276, 31800, 33277, 31803),
            (33276, 31801),
            vec![]
        ),
        crop!(
            "region-z06-x126-y125.b3",
            (6, 126, 125),
            (32311, 32172, 32312, 32174),
            (32311, 32173),
            vec![]
        ),
        crop!(
            "region-z07-x129-y121.b3",
            (7, 129, 121),
            (33057, 31028, 33058, 31030),
            (33057, 31028),
            vec![object(33057, 31029, vec![8309, 2472])]
        ),
    ];
    native_scene_with_appearances(native, crops, &[], &[2772, 2773])
}

async fn defeat_mine_leader(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::attack::{
        COMMAND_TYPE_ATTACK_TARGET_INTENT, COMMAND_TYPE_FIGHT_MODES_INTENT, FightMode, FightModes,
        encode_attack_target_intent, encode_fight_modes_intent,
    };
    let player = a.runtime_actor.ok_or("player")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("creature policies")?;
    let scheduled = farmine_world_spawn()?;
    let spawn_key = scheduled.key.clone();
    {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .configure_boss_spawns(vec![scheduled])
            .map_err(debug)?;
    }
    crate::durability::DurabilityRoot::with_background_holder(
        owner.boss_spawn_pass(&mut std::collections::BTreeMap::new()),
    )
    .await
    .map_err(debug)?;
    // The donor spawn's 90-second deadline uses PostgreSQL wall time. Advance
    // only the fixture's initial DUE clock and let the normal owner claim it.
    let due = sqlx::query(
        "UPDATE game_boss_spawn_clocks SET due_at = 0
         WHERE world_id = encode($1, 'hex')::uuid AND channel_id = encode($2, 'hex')::uuid
            AND spawn_key = $3 AND cycle = 0 AND state = 1 AND due_at IS NOT NULL",
    )
    .bind(owner.world_id.as_bytes().as_slice())
    .bind(owner.channel_id.as_bytes().as_slice())
    .bind(spawn_key)
    .execute(&h.pool)
    .await?;
    assert_eq!(
        due.rows_affected(),
        1,
        "the captured mine leader is scheduled"
    );
    crate::durability::DurabilityRoot::with_background_holder(
        owner.boss_spawn_pass(&mut std::collections::BTreeMap::new()),
    )
    .await
    .map_err(debug)?;
    let (actor, target) = {
        let mut runtime = owner.runtime.lock().await;
        let actor = runtime
            .boss_spawns()
            .first()
            .and_then(|spawn| spawn.actor)
            .ok_or("scheduled mine leader was not realized")?;
        // The test starts at the lethal swing. Attacking, capturing the principal
        // and settling both combat and the quest receipt use their native owners.
        runtime.set_test_creature_health(actor, 1).map_err(debug)?;
        let visible = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|entry| entry.actor == actor)
            .ok_or("mine leader actor")?;
        (
            actor,
            EntityRef {
                identity: actor.placement_identity(),
                generation: visible.generation,
            },
        )
    };
    let command = super::annihilator_combat::command;
    command(
        owner,
        a,
        COMMAND_TYPE_FIGHT_MODES_INTENT,
        encode_fight_modes_intent(&FightModes {
            fight_mode: FightMode::Offensive,
            ..FightModes::DEFAULT
        }),
    )
    .await?;
    command(
        owner,
        a,
        COMMAND_TYPE_ATTACK_TARGET_INTENT,
        encode_attack_target_intent(Some(&target)).map_err(debug)?,
    )
    .await?;
    assert!(
        owner
            .refresh_source_party_presence(player, a.game_session_id)
            .await
    );
    for _ in 0..64 {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        // The production swing rechecks its equipment snapshot against this tick.
        owner.refresh_attack_weapon(player, a.game_session_id).await;
        owner.drain_auto_attacks().await;
        if owner
            .runtime
            .lock()
            .await
            .live_creature_health(actor)
            .is_none()
        {
            return super::qengine_kill_stories_fixtures::drain_deaths(owner, a, &[]).await;
        }
    }
    Err("no lethal attack against Shard of Corruption".into())
}

async fn move_potion(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
    use crate::gameplay_transport::item_view::ItemKey;
    let (potion, bag): (Vec<u8>, Vec<u8>) = sqlx::query_as(
        "SELECT uuid_send(child.item_instance_id), uuid_send(parent.item_instance_id) \
         FROM game_item_nested_entries e JOIN game_item_instances child USING(item_instance_id) \
         JOIN game_item_instances parent ON parent.item_instance_id=e.parent_item_instance_id \
         WHERE child.definition_production_key='oteryn:item.tibia.i7439' \
           AND parent.definition_production_key='oteryn:item.tibia.i2854' AND child.lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?;
    let children = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
        .bind(&bag).fetch_one(&h.pool)
    };
    assert_eq!(
        children().await?,
        3,
        "all three potions started in the claimed bag"
    );
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("MOVE fence")?;
    assert_eq!(
        owner
            .move_loop_item(
                a.runtime_actor.ok_or("player")?,
                UseCommand {
                    game_session_id: a.game_session_id,
                    command_id: 90_001,
                    item_fence: Some(fence.item_fence())
                },
                LoopMove {
                    item: ItemKey::Instance(potion.try_into().map_err(|_| "potion UUID")?),
                    count: None,
                    target: LoopTarget::MainBackpack,
                    slot: None
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    assert_eq!(children().await?, 2);
    Ok(())
}

pub(super) fn farmine_world_spawn() -> TestResult<crate::foundation::BossSpawn> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let read = |p: &str| -> TestResult<serde_json::Value> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(p))?)?)
    };
    let placements = read("content/world/npc-placements/index.json")?;
    let mut contacts = std::collections::BTreeSet::new();
    for shard in placements["shards"]
        .as_array()
        .ok_or("NPC placement shards")?
    {
        let doc = read(shard.as_str().ok_or("NPC placement shard")?)?;
        for row in doc["records"].as_array().ok_or("NPC placement records")? {
            let declaration = &row["declaration"];
            for (name, p, _) in CONTACTS {
                if declaration["npc"]
                    .as_str()
                    .and_then(|npc| npc.strip_prefix("oteryn:npc."))
                    == Some(name)
                    && declaration["cell"] == serde_json::json!({"x":p.x,"y":p.y,"floor":-p.floor})
                {
                    contacts.insert(name);
                }
            }
        }
    }
    assert_eq!(
        contacts.len(),
        CONTACTS.len(),
        "all eight production NPC cells"
    );
    let project = oteryn_world_bundle_compiler::source::project_regions(&root, &[(129, 122)])?;
    let (registry, palette) = oteryn_world_bundle_compiler::source::registry(&root, &project)?;
    let (table, _) = oteryn_world_bundle_compiler::compile::realize_spawns(
        &oteryn_world_bundle_compiler::compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: Default::default(),
            world: oteryn_world_bundle_compiler::source::world_extent(&root)?,
            build_class: oteryn_world_bundle::bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    let index = table
        .creatures
        .iter()
        .position(|creature| creature.key == BOSS)
        .ok_or("production mine-leader spawn")?;
    let index = u32::try_from(index)?;
    let points: Vec<_> = table
        .sources
        .iter()
        .flat_map(|source| {
            source
                .points
                .iter()
                .enumerate()
                .map(move |(ordinal, point)| (source, ordinal, point))
        })
        .filter(|(_, _, point)| point.creature == index)
        .collect();
    assert_eq!(points.len(), 1);
    let (source, ordinal, point) = points[0];
    assert_eq!(
        (point.x, point.y, source.floor),
        (BOSS_CELL.x, BOSS_CELL.y, BOSS_CELL.floor)
    );
    Ok(crate::foundation::BossSpawn {
        key: format!("{}:{ordinal}", source.key),
        creature: BOSS.into(),
        position: local(BOSS_CELL),
        policy: point
            .boss_policy
            .ok_or("scheduled mine leader boss policy")?,
        actor: None,
        cycle: 0,
        pending_claim: false,
    })
}
