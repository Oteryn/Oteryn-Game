use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::FreshAdmissionAuthority;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::{
    classic_talk::ROTTIN_WALLCOUNT,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "rottin-repair-42501";
const TARGET: Position = Position {
    x: 32647,
    y: 32216,
    floor: -7,
};
pub(super) const START: Position = Position {
    x: 32647,
    y: 32217,
    floor: -7,
};

#[test]
fn rottin_hammer_repairs_counts_and_resets_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("rottin-walls", || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(
                "../../../../../../content/world/placements/region-z07-x127-y125.b3"
            ),
            region: (7, 127, 125),
            bounds: (32646, 32216, 32648, 32218),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (TARGET.x, TARGET.y),
                ids: vec![11732, 4144, 12183],
                counted_entry: None,
            }],
        }],
        &[],
        &[6474],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TARGET,
            item: "oteryn:item.tibia.i3460".into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    let position = crate::map::overlay::TilePos {
        x: TARGET.x,
        y: TARGET.y,
        floor: TARGET.floor,
    };
    let key = placement_key(position, 2).ok_or("wall key")?;
    let source_id = *owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or("wall tile")?
        .ids()
        .get(2)
        .ok_or("wall source")?;
    for count in 1..=7 {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let map = owner.bundle_map().ok_or("map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
            use_with: None,
            use_with_position: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            })),
        })
        .map_err(debug)?;
        let reference = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|n| n.item_index())
            .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3460", "definition-r1"))
            .ok_or("hammer reference")?;
        let (next, result, _) = super::item_craft::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(reference),
        )
        .await?;
        *admitted = next;
        if count == 7 {
            assert_ne!(result, UseDisposition::Committed);
        } else {
            assert_eq!(result, UseDisposition::Committed);
        }
        let state = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(ROTTIN_WALLCOUNT), Some(&count.min(6)));
        if count == 7 {
            break;
        }
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(
            map.facts
                .base_entry(position, 2, source_id)
                .ok_or("repaired wall")?
                .appearance_id,
            6474
        );
        let now = owner.owner_now().get();
        let due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
        assert!((119_000_000..=120_000_000).contains(&due.saturating_sub(now)));
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        assert_eq!(map.facts.expire_movement_items(due), 1);
        assert_eq!(
            map.facts
                .base_entry(position, 2, source_id)
                .ok_or("restored wall")?
                .appearance_id,
            12183
        );
        assert_eq!(map.facts.object_revision(key), u64::try_from(count)? * 2);
    }
    let hammer: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3460' AND lifecycle=1")
        .fetch_one(&h.pool).await?;
    assert_eq!(hammer, 1);
    Ok(())
}
