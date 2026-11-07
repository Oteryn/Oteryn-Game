//! Exact source AID30008: consume trigger, two fresh walls, nine removals.
use super::{TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    trigger: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let root = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("trigger")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&root.to_be_bytes());
    let results = super::quest_callbacks::wire_uses(
        owner,
        *admitted,
        &[1, 2],
        &WorldObjectTarget {
            placement,
            expected_revision: 0,
        },
    )
    .await?;
    assert_eq!(results[0], UseDisposition::Committed);
    assert_ne!(
        results[1],
        UseDisposition::Committed,
        "consumed lever cannot be reused"
    );
    assert!(map.facts.base_hidden(root));
    for x in 32593..=32601 {
        let position = crate::map::overlay::TilePos {
            x,
            y: 32104,
            floor: -14,
        };
        let key = crate::map::view::placement_key(position, 1).ok_or("wall")?;
        assert!(map.facts.base_hidden(key));
        assert!(
            !map.facts
                .quest_callback_blocked(map.overlay.base(), position, true)
        );
    }
    for y in [32105, 32106] {
        let position = crate::map::overlay::TilePos {
            x: 32592,
            y,
            floor: -14,
        };
        let tile = crate::map::view::compose_tile(&map.overlay, &map.facts, position)
            .map_err(debug)?
            .ok_or("new wall tile")?;
        assert_eq!(
            tile.entries
                .iter()
                .filter(|e| e.facts.appearance_id == 1270)
                .count(),
            1
        );
        assert!(
            map.facts
                .quest_callback_blocked(map.overlay.base(), position, false)
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}
