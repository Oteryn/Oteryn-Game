//! Quest lane qd, batch 2: door, body and hand-in routes over the shared world scenario.
//! Each case names its TibiaWiki source; the route is driven through NPC talk, the
//! placed quest door, a real step and USE on the production placement.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const PREFIX: &str = "qd-b2-";
const TROLL_SABOTAGE: &str = "qd-b2-troll-sabotage";
const WHITE_RAVEN: &str = "qd-b2-white-raven-monastery";
const RAVEN_TRACK: &str = "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery";
const RAVEN_START: Position = Position {
    x: 32172,
    y: 31936,
    floor: -7,
};
const RAVEN_DOOR: Position = Position {
    x: 32171,
    y: 31936,
    floor: -7,
};
const FUGIO_BODY: Position = Position {
    x: 32262,
    y: 31861,
    floor: -11,
};
const FUGIO_STAND: Position = Position {
    x: 32262,
    y: 31862,
    floor: -11,
};

#[test]
fn qd_b2_troll_sabotage_grarkharok_lady_name_and_elvenhair_rope() -> TestResult {
    run(TROLL_SABOTAGE)
}

#[test]
fn qd_b2_white_raven_monastery_warded_door_fugio_body_and_ankh() -> TestResult {
    run(WHITE_RAVEN)
}

fn run(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, start(name)?, false)
    })
}

pub(super) fn start(name: &str) -> TestResult<Position> {
    match name {
        TROLL_SABOTAGE | WHITE_RAVEN => Ok(RAVEN_START),
        _ => Err(format!("unknown qd-b2 world case {name}").into()),
    }
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let key = match name {
        // The NPC-only Troll Sabotage case reuses the White Raven scene.
        TROLL_SABOTAGE | WHITE_RAVEN => "oteryn:quest-callback/qd-white-raven-fugio-body",
        _ => return Err(format!("unknown qd-b2 world case {name}").into()),
    };
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|c| c.key == key)
        .ok_or_else(|| format!("qd-b2 world callback {key}").into())
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    match name {
        TROLL_SABOTAGE | WHITE_RAVEN => native_scene(
            native,
            vec![
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z07-x125-y124.b3"
                    )),
                    region: (7, 125, 124),
                    bounds: (32168, 31932, 32176, 31940),
                    start: (32172, 31936),
                    expected: vec![ExpectedTile {
                        position: (32171, 31936),
                        ids: vec![409, 1642],
                        counted_entry: None,
                    }],
                },
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z11-x126-y124.b3"
                    )),
                    region: (11, 126, 124),
                    bounds: (32259, 31859, 32266, 31865),
                    start: (32262, 31862),
                    expected: vec![ExpectedTile {
                        position: (32262, 31861),
                        ids: vec![410, 2573, 4240],
                        counted_entry: None,
                    }],
                },
            ],
            &[],
        ),
        _ => Err(format!("unknown qd-b2 world case {name}").into()),
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    match name {
        TROLL_SABOTAGE => troll_sabotage(owner, admitted, h, authority).await,
        WHITE_RAVEN => white_raven(owner, admitted, h, authority).await,
        _ => Err(format!("unknown qd-b2 world case {name}").into()),
    }
}

async fn count(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}

async fn completed(h: &Harness, quest: &str) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE quest_key=$1 AND completed_character_revision IS NOT NULL)")
        .bind(quest).fetch_one(&h.pool).await?)
}

async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
    plain: crate::gameplay_transport::capabilities::SelectedCapabilities,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("qd-b2 map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        1,
    )
    .ok_or("qd-b2 placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let command = admitted.continuity.next_command_id;
    // The bare use wire carries no appearance state, so it joins without the talk set.
    let talk = std::mem::replace(&mut admitted.continuity.selected_capabilities, plain);
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        encode_use_intent(&target).map_err(debug)?,
    )
    .await;
    admitted.continuity.selected_capabilities = talk;
    decode_use_result(&reply?).map_err(debug)
}

async fn provision(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: &str,
    id: u64,
    position: Position,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position,
            item: item.into(),
            count: 1,
        },
        id,
        true,
    )
    .await
}

/// TibiaWiki The_White_Raven_Monastery_Quest: Dalbrect's brooch earns the passage and
/// Costello's trust, Costello opens the warded catacomb doors, brother Fugio's body in
/// the Ghostlands holds a backpack with his diary, and Costello trades the diary for a
/// blessed Ankh, which ends the quest.
async fn white_raven(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    const BROOCH: &str = "oteryn:item.tibia.i3205";
    const BACKPACK: &str = "oteryn:item.tibia.i2854";
    const DIARY: &str = "oteryn:item.tibia.i3212";
    const ANKH: &str = "oteryn:item.tibia.i3214";
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load_for(
        owner.quest_catalogue.as_ref().ok_or("catalogue")?,
    )
    .map_err(debug)?;
    let find = |key: &str| {
        callbacks
            .iter()
            .find(|c| c.key == key)
            .ok_or_else(|| format!("White Raven callback {key}"))
    };
    let brooch = find("oteryn:quest-reward/l2-white-raven-brooch")?;
    let catacombs = find("oteryn:quest-reward/l2-white-raven-catacombs")?;
    let diary = find("oteryn:quest-reward/l2-white-raven-diary")?;
    let [dalbrect, costello] = super::classic_quests::install_positions(
        owner,
        admitted,
        [brooch.npc.as_str(), catacombs.npc.as_str()],
        [
            Position {
                x: 32173,
                y: 31935,
                floor: -7,
            },
            Position {
                x: 32173,
                y: 31937,
                floor: -7,
            },
        ],
    )
    .await?;
    let plain = admitted.continuity.selected_capabilities;
    select_talk_capabilities(owner, admitted, &[3, 6])?;
    let talk = super::qengine_rewards::wire_reward;
    let track = |name: &str| format!("{RAVEN_TRACK}/{name}");
    // The warded door stays shut before Costello's permission.
    assert_ne!(
        use_at(owner, admitted, RAVEN_DOOR, plain).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        talk(owner, admitted, costello, catacombs, false).await?.0,
        NpcIntentDisposition::Rejected,
        "Costello trusts only Dalbrect's friends"
    );
    provision(owner, admitted, BROOCH, 54001, RAVEN_START).await?;
    assert_eq!(
        talk(owner, admitted, dalbrect, brooch, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(count(h, BROOCH).await?, 0);
    assert_eq!(
        talk(owner, admitted, costello, catacombs, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, &track("door")).await?,
        1
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    // The placed warded door (action 10137) now opens and the player walks through.
    assert_eq!(
        use_at(owner, admitted, RAVEN_DOOR, plain).await?,
        UseDisposition::Committed
    );
    let command = admitted.continuity.next_command_id;
    super::questpc8_authored_programmes::step(owner, admitted, command, StepDirection::West)
        .await?;
    // Fixture setup between disconnected crops: the catacombs lead down to the Ghostlands.
    super::l6_native_signs::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        FUGIO_STAND,
    )
    .await?;
    assert_eq!(count(h, BACKPACK).await?, 0);
    assert_eq!(
        use_at(owner, admitted, FUGIO_BODY, plain).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        count(h, BACKPACK).await?,
        1,
        "Fugio's body holds a backpack"
    );
    assert_eq!(
        count(h, DIARY).await?,
        1,
        "the backpack holds Fugio's diary"
    );
    assert_ne!(
        use_at(owner, admitted, FUGIO_BODY, plain).await?,
        UseDisposition::Committed,
        "once per character"
    );
    assert_eq!(count(h, DIARY).await?, 1);
    super::l6_native_signs::relocate(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        admitted.game_session_id,
        RAVEN_START,
    )
    .await?;
    assert_eq!(count(h, ANKH).await?, 0);
    assert_eq!(
        talk(owner, admitted, costello, diary, false).await?.0,
        NpcIntentDisposition::Ok
    );
    assert_eq!(count(h, DIARY).await?, 0);
    assert_eq!(count(h, ANKH).await?, 1, "Costello gives the blessed Ankh");
    assert_eq!(
        super::classic_quests::progress(owner, admitted, authority, &track("diary")).await?,
        2
    );
    assert!(completed(h, "oteryn:quest.the_white_raven_monastery_quest").await?);
    Ok(())
}

/// TibiaWiki Troll_Sabotage_Quest: Jerom's mission, Chief Grarkharok's threat, troll lady
/// and name exchange for Jerom's Family Necklace, and the Elvenhair Rope from Jerom.
async fn troll_sabotage(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline";
    const NECKLACE: &str = "oteryn:item.tibia.i7754";
    const ROPE: &str = "oteryn:item.tibia.i646";
    let [jerom, chief] = super::classic_quests::install_positions(
        owner,
        admitted,
        ["oteryn:npc.jerom", "oteryn:npc.chief_grarkharok"],
        [
            Position {
                x: 32173,
                y: 31935,
                floor: -7,
            },
            Position {
                x: 32173,
                y: 31937,
                floor: -7,
            },
        ],
    )
    .await?;
    let name = owner
        .states
        .chat
        .speaker_name(admitted.game_session_id)
        .ok_or("speaker name")?;
    let mut command = admitted.continuity.next_command_id + 1000;
    let progress = super::classic_quests::progress;
    // Grarkharok ignores a threat before Jerom's mission.
    for text in ["hi", "kill", "lady", "yes", name.as_str(), "bye"] {
        say(owner, admitted, &mut command, chief, text).await?;
    }
    assert_eq!(count(h, NECKLACE).await?, 0);
    for text in ["hi", "mission", "yes", "bye"] {
        say(owner, admitted, &mut command, jerom, text).await?;
    }
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 1);
    // A name that is not the speaker's own is not an answer.
    for text in ["hi", "kill", "lady", "yes", "Nobody Here"] {
        say(owner, admitted, &mut command, chief, text).await?;
    }
    assert_eq!(count(h, NECKLACE).await?, 0);
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 1);
    say(owner, admitted, &mut command, chief, "kill").await?;
    let lady = say(owner, admitted, &mut command, chief, "lady").await?;
    assert!(
        lady.iter()
            .any(|line| line.contains("troll lady for Grarkharok")),
        "{lady:?}"
    );
    say(owner, admitted, &mut command, chief, "yes").await?;
    let named = say(owner, admitted, &mut command, chief, &name).await?;
    assert!(
        named
            .iter()
            .any(|line| line.starts_with(&format!("{name}?!?"))),
        "{named:?}"
    );
    assert_eq!(
        count(h, NECKLACE).await?,
        1,
        "Grarkharok hands over the necklace"
    );
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 2);
    say(owner, admitted, &mut command, chief, "bye").await?;
    say(owner, admitted, &mut command, jerom, "hi").await?;
    let thanks = say(owner, admitted, &mut command, jerom, "mission").await?;
    assert!(
        thanks.iter().any(|line| line.contains("strange rope")),
        "{thanks:?}"
    );
    assert_eq!(count(h, NECKLACE).await?, 0);
    assert_eq!(count(h, ROPE).await?, 1, "Jerom gives the Elvenhair Rope");
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 3);
    assert!(completed(h, "oteryn:quest.troll_sabotage_quest").await?);
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: &mut u64,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    *command += 1;
    super::classic_quests::talk(owner, admitted, npc, *command, text).await
}

/// Selects NPC talk capabilities the way the generic reward flow does.
pub(super) fn select_talk_capabilities(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    wanted: &[u32],
) -> TestResult {
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend(wanted);
    selected.sort_unstable();
    selected.dedup();
    use crate::gameplay_transport::connection::FreshAdmissionAuthority as _;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    Ok(())
}
