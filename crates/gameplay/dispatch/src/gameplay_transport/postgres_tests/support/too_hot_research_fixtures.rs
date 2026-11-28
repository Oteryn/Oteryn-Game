use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, WorldObjectTarget};

use super::classic_quests_fixtures::{install_positions, progress, talk, travel};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::map::{overlay::TilePos, view::placement_key};

pub(super) const NAME: &str = "too-hot/research-coffin";
const STAGE: &str = "oteryn:quest-progress/too-hot/stage";
const RECRUITMENT: &str = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/quest_line_complete";
const BRODRAEM: Position = cell(32612, 31869);
const GNOMYSTERY: Position = cell(32608, 31878);
pub(super) const START: Position = cell(32608, 31879);
const COFFIN_STAND: Position = cell(32612, 31868);

const fn cell(x: u16, y: u16) -> Position {
    Position { x, y, floor: -9 }
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    // The compiler drops unresolved decorative overlays and keeps their native base objects.
    // The assertions below drive every selected object through the resulting placement stack.
    let start = TilePos {
        x: START.x,
        y: START.y,
        floor: START.floor,
    };
    super::source_world_crop::compiled_region(start, start, start, native)
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    callback: &Callback,
    revision: u64,
) -> TestResult<UseDisposition> {
    let native = callback.selector.native.as_ref().ok_or("native target")?;
    let at = native.position;
    let map = owner.bundle_map().ok_or("crypt map")?;
    let key = placement_key(
        TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        native.ordinal,
    )
    .ok_or("target placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: revision,
    })
    .map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let reply = Box::pin(super::quest_world_callbacks_fixtures::wire(
        owner, admitted, command, 2, payload,
    ))
    .await?;
    world_object::decode_use_result(&reply).map_err(debug)
}

async fn carried(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1",
    ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    coffin: &Callback,
) -> TestResult {
    let [gnome, keeper] = install_positions(
        owner,
        admitted,
        ["oteryn:npc.gnomystery", "oteryn:npc.brodraem_orcbeard"],
        [GNOMYSTERY, BRODRAEM],
    )
    .await?;
    talk(owner, admitted, gnome, 501, "hi").await?;
    talk(owner, admitted, gnome, 502, "favour").await?;
    let _ = super::qengine_rewards_fixtures::talk(owner, admitted, gnome, 503, "yes").await?;
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 0);
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(RECRUITMENT.into(), 2)],
        0xa7,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    for (id, text) in [(504, "talk"), (505, "favour"), (506, "yes")] {
        talk(owner, admitted, gnome, id, text).await?;
    }
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 1);

    travel(owner, admitted, COFFIN_STAND).await?;
    assert_eq!(
        use_object(owner, admitted, coffin, 0).await?,
        UseDisposition::Rejected
    );
    for (id, text) in [
        (508, "hi"),
        (509, "business"),
        (510, "incidents"),
        (511, "yes"),
    ] {
        talk(owner, admitted, keeper, id, text).await?;
    }
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 2);
    let _ = super::qengine_rewards_fixtures::talk(owner, admitted, keeper, 512, "report").await?;
    assert_eq!(carried(h, 32769).await?, 0);

    assert_eq!(
        use_object(owner, admitted, coffin, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        use_object(owner, admitted, coffin, 1).await?,
        UseDisposition::Rejected
    );
    for (item, quantity) in [(36543, 1), (36544, 1), (36545, 5)] {
        assert_eq!(carried(h, item).await?, quantity);
    }

    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    for (family, stands) in [
        (
            "m07/too-hot-column",
            &[cell(32593, 31842), cell(32600, 31841), cell(32610, 31842)][..],
        ),
        (
            "too-hot/sarcophagus",
            &[
                cell(32594, 31844),
                cell(32594, 31850),
                cell(32594, 31854),
                cell(32612, 31850),
                cell(32612, 31854),
            ][..],
        ),
        (
            "too-hot/marble",
            &[
                cell(32596, 31845),
                cell(32601, 31843),
                cell(32606, 31842),
                cell(32609, 31844),
            ][..],
        ),
    ] {
        for (index, &stand) in stands.iter().enumerate() {
            let key = format!("oteryn:quest-callback/{family}-{:02}", index + 1);
            let repair = callbacks
                .iter()
                .find(|row| row.key == key)
                .ok_or("repair callback")?;
            travel(owner, admitted, stand).await?;
            assert_eq!(
                use_object(owner, admitted, repair, 0).await?,
                UseDisposition::Committed,
                "{key}"
            );
            if index == 0 {
                assert_eq!(
                    use_object(owner, admitted, repair, 1).await?,
                    UseDisposition::Rejected,
                    "{key} repeated"
                );
            }
        }
    }
    for item in [36543, 36544, 36545] {
        assert_eq!(carried(h, item).await?, 1);
    }

    travel(owner, admitted, COFFIN_STAND).await?;
    talk(owner, admitted, keeper, 515, "hi").await?;
    assert_eq!(progress(owner, admitted, authority, STAGE).await?, 3);
    assert_eq!(carried(h, 32769).await?, 1);
    let _ = super::qengine_rewards_fixtures::talk(owner, admitted, keeper, 516, "report").await?;
    assert_eq!(carried(h, 32769).await?, 1);
    assert_eq!(
        use_object(owner, admitted, coffin, 1).await?,
        UseDisposition::Rejected
    );
    Ok(())
}
