//! Atomic source map removals/creations, including consumed and repeatable triggers.
use super::{TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn source_object_mutations_are_atomic_bounded_and_idempotent() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| -> Result<(), String> {
        let started = std::time::Instant::now();
        for name in ["object-mutation-regression", "object-mutation-refusal", "object-mutation-repeat"] {
            super::quest_world_callbacks::scenario(name, Position {x:33293,y:31680,floor:-15}, false)
                .map_err(|error| error.to_string())?;
        }
        println!("POSTGRES_CASE_COMPLETED source_object_mutations_are_atomic_bounded_and_idempotent duration={:?}", started.elapsed());
        Ok(())
    })?.join().map_err(|_| "object mutation scenario panicked")??;
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    h: &super::fixture::Harness,
    name: &str,
    trigger: Position,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let root = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("root placement")?;
    let removed = oteryn_world_bundle::bundle::placement_key(-15, 33295, 31677, 1)
        .ok_or("removed placement")?;
    let at = crate::map::overlay::TilePos {
        x: trigger.x + 1,
        y: trigger.y,
        floor: trigger.floor,
    };
    let count = || -> TestResult<usize> {
        Ok(crate::map::view::compose_tile(&map.overlay, &map.facts, at)
            .map_err(debug)?
            .ok_or("created tile")?
            .entries
            .iter()
            .filter(|entry| entry.facts.appearance_id == 1270)
            .count())
    };
    assert_eq!(count()?, 0);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&root.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let updated = WorldObjectTarget {
        placement: target.placement.clone(),
        expected_revision: 1,
    };
    let commands = if name == "object-mutation-repeat" {
        vec![(1, &target), (2, &updated)]
    } else {
        vec![(1, &target)]
    };
    let results = super::quest_callbacks::wire_use_targets(owner, *admitted, &commands).await?;
    if name == "object-mutation-refusal" {
        assert_eq!(results[0].1, UseDisposition::Rejected);
        assert!(!map.facts.base_hidden(root));
        assert!(!map.facts.base_hidden(removed));
        assert_eq!(map.facts.object_revision(root), 0);
        assert_eq!(count()?, 0);
    } else {
        assert!(
            results
                .iter()
                .all(|(_, result)| *result == UseDisposition::Committed)
        );
        assert_eq!(
            map.facts.base_hidden(root),
            name != "object-mutation-repeat"
        );
        assert!(map.facts.base_hidden(removed));
        assert_eq!(count()?, 1, "repeat must not duplicate the created wall");
        assert!(
            map.facts
                .quest_callback_blocked(map.overlay.base(), at, false)
        );
    }
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        receipts, 0,
        "static objects must not write durable quest receipts"
    );
    Ok(())
}
