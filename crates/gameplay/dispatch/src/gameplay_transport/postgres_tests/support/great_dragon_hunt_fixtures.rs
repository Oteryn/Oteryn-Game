use super::classic_quests_fixtures::progress;
use super::fixture::Harness;
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const EXPLORER: Position = Position {
    x: 33007,
    y: 32607,
    floor: -8,
};
pub(super) const START: Position = Position {
    x: 33008,
    ..EXPLORER
};
const SKELETON: &str = "oteryn:quest-progress/quest/u10_80/the_great_dragon_hunt/warrior_skeleton";
const GUILD: &str = "oteryn:quest-progress/quest/u9_80/adventurers_guild/quest_line";

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x128-y127.b3"
            )),
            region: (8, 128, 127),
            bounds: (33005, 32606, 33009, 32608),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (33007, 32607),
                ids: vec![351, 21412],
                counted_entry: None,
            }],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let key = oteryn_world_bundle::bundle::placement_key(-8, 33007, 32607, 1)
        .ok_or("explorer placement")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("explorer map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    assert_eq!(progress(owner, admitted, authority, SKELETON).await?, 0);
    for (command_id, expected) in [
        (1, UseDisposition::Committed),
        (2, UseDisposition::Rejected),
    ] {
        let target = WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: owner
                .bundle_map()
                .ok_or("explorer map")?
                .facts
                .object_revision(key),
        };
        let reply = owner
            .use_object(
                admitted.runtime_actor.ok_or("explorer actor")?,
                UseCommand {
                    command_id,
                    game_session_id: admitted.game_session_id,
                    item_fence: admitted.item_fence,
                },
                target,
            )
            .await;
        assert_eq!(reply.disposition, expected, "explorer USE {command_id}");
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
    }
    assert_eq!(progress(owner, admitted, authority, SKELETON).await?, 1);
    assert_eq!(progress(owner, admitted, authority, GUILD).await?, 1);
    let leather: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i5882'")
        .fetch_one(&h.pool).await?;
    assert_eq!(leather, 1);
    Ok(())
}
