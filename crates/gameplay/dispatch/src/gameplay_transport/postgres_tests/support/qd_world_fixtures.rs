use oteryn_gameplay_world::map::view::MapFacts;
use super::fixture::{debug, Harness};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::durability::character_authority::ReconciledCharacterAuthority;

use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};

use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

use oteryn_protocol_oteryn::world_object::*;

use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const PREFIX: &str = "qd-world-";

pub(super) const AHMET: &str = "qd-world-steal-from-thieves";
pub(super) const OASIS_BOX: Position = Position { x: 33131, y: 32661, floor: -7 };
const OASIS_APPROACH: Position = Position { x: 33130, y: 32660, floor: -7 };

pub(super) const SAMS: &str = "qd-world-sams-old-backpack";

const SAMS_NPC: &str = "oteryn:quest-progress/quest/u7_5/sams_old_backpack/sams_old_backpack_npc";

const SAMS_DOOR: &str = "oteryn:quest-progress/quest/u7_5/sams_old_backpack/sams_old_backpack_door";

const SAMS_CHEST: Position = Position {
    x: 32455,
    y: 31968,
    floor: -14,
};

const SAMS_GATE: Position = Position {
    x: 32455,
    y: 31967,
    floor: -14,
};

pub(super) fn start(name: &str) -> TestResult<Position> {
    match name {
        AHMET => Ok(OASIS_APPROACH),
        SAMS => Ok(Position {
            x: 32455,
            y: 31966,
            floor: -14,
        }),
        _ => Err(format!("unknown qd world case {name}").into()),
    }
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let key = match name {
        AHMET => "oteryn:quest-callback/qd-steal-from-thieves-oasis-box",
        SAMS => "oteryn:quest-callback/qd-sams-old-backpack-chest",
        _ => return Err(format!("unknown qd world case {name}").into()),
    };
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|c| c.key == key)
        .ok_or_else(|| format!("qd world callback {key}").into())
}

pub(super) fn world(
    name: &str,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{native_scene, ExpectedTile, NativeCrop};
    match name {
        AHMET => native_scene(
            native,
            vec![NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z07-x129-y127.b3"
                )),
                region: (7, 129, 127),
                bounds: (33128, 32659, 33134, 32665),
                start: (OASIS_APPROACH.x, OASIS_APPROACH.y),
                expected: vec![ExpectedTile {
                    position: (33131, 32661),
                    ids: vec![4515, 2473],
                    counted_entry: None,
                }],
            }],
            &[],
        ),
        SAMS => native_scene(
            native,
            vec![NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z14-x126-y124.b3"
                )),
                region: (14, 126, 124),
                bounds: (32452, 31963, 32458, 31969),
                start: (32455, 31966),
                expected: vec![
                    ExpectedTile {
                        position: (32455, 31967),
                        ids: vec![355, 1644],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (32455, 31968),
                        ids: vec![355, 2472],
                        counted_entry: None,
                    },
                ],
            }],
            &[],
        ),
        _ => Err(format!("unknown qd world case {name}").into()),
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
        AHMET => super::qd_steal_from_thieves_fixtures::exercise(owner, admitted, h, authority).await,
        SAMS => sams_old_backpack(owner, admitted, h, authority).await,
        _ => Err(format!("unknown qd world case {name}").into()),
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

pub(super) async fn use_at(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
    plain: crate::gameplay_transport::capabilities::SelectedCapabilities,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("qd map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        1,
    )
    .ok_or("qd placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let command = admitted.continuity.next_command_id;
    // The bare use wire carries no appearance state, so it joins without the talk set.
    let talk = std::mem::replace(&mut admitted.continuity.selected_capabilities, plain);
    let reply = super::quest_world_callbacks_fixtures::wire(
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

/// TibiaWiki Sam's_Old_Backpack_Quest: Sam takes the backpack, Kroox grants the mine
/// door after "Sam sent me", and the chest behind the sealed door holds a Dwarven Armor.
async fn sams_old_backpack(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    const ARMOR: &str = "oteryn:item.tibia.i3397";
    super::qengine_outfit_stories_fixtures::earn_level(owner, admitted, authority, 35).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load_for(
        owner.quest_catalogue.as_ref().ok_or("catalogue")?,
    )
    .map_err(debug)?;
    let sam = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/sam-backpack")
        .ok_or("sam")?;
    let kroox = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qd-sams-old-backpack-kroox")
        .ok_or("kroox")?;
    let [sam_npc, kroox_npc] = super::classic_quests_fixtures::install_positions(
        owner,
        admitted,
        [sam.npc.as_str(), kroox.npc.as_str()],
        [
            Position {
                x: 32454,
                y: 31965,
                floor: -14,
            },
            Position {
                x: 32456,
                y: 31965,
                floor: -14,
            },
        ],
    )
    .await?;
    let plain = admitted.continuity.selected_capabilities;
    select_talk_capabilities(owner, admitted, &[3, 6])?;
    let talk = super::qengine_rewards_fixtures::wire_conversation;
    // Neither the sealed door nor the chest answer before Kroox's permission.
    assert_ne!(
        use_at(owner, admitted, SAMS_GATE, plain).await?,
        UseDisposition::Committed
    );
    let (result, _, _) = talk(
        owner,
        admitted,
        kroox_npc,
        kroox,
        false,
        vec!["hi", "sam sent me"],
    )
    .await?;
    assert_eq!(
        result,
        NpcIntentDisposition::Rejected,
        "Kroox waits for Sam's letter"
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 32455,
                y: 31966,
                floor: -14,
            },
            item: "oteryn:item.tibia.i3244".into(),
            count: 1,
        },
        53000,
        true,
    )
    .await?;
    let (result, _, _) = talk(
        owner,
        admitted,
        sam_npc,
        sam,
        false,
        vec!["hi", "backpack", "yes", "yes"],
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, SAMS_NPC).await?,
        1
    );
    let (result, _, _) = talk(
        owner,
        admitted,
        kroox_npc,
        kroox,
        false,
        vec!["hi", "sam send me"],
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, SAMS_NPC).await?,
        2
    );
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, SAMS_DOOR).await?,
        1
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    // The placed quest door (action 40603) now opens, and the player walks into the alcove.
    assert_eq!(
        use_at(owner, admitted, SAMS_GATE, plain).await?,
        UseDisposition::Committed
    );
    let command = admitted.continuity.next_command_id;
    super::questpc8_authored_programmes_fixtures::step(owner, admitted, command, StepDirection::South)
        .await?;
    assert_eq!(count(h, ARMOR).await?, 0);
    assert_eq!(
        use_at(owner, admitted, SAMS_CHEST, plain).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        count(h, ARMOR).await?,
        1,
        "the chest holds one Dwarven Armor"
    );
    assert_eq!(
        super::classic_quests_fixtures::progress(owner, admitted, authority, SAMS_NPC).await?,
        3
    );
    assert!(completed(h, "oteryn:quest.sam_s_old_backpack_quest").await?);
    assert_ne!(
        use_at(owner, admitted, SAMS_CHEST, plain).await?,
        UseDisposition::Committed,
        "once per character"
    );
    assert_eq!(count(h, ARMOR).await?, 1);
    Ok(())
}

/// Selects NPC talk capabilities the way the generic reward flow does (3, 6 and, for a
/// character with an appearance state, appearance 25).
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
