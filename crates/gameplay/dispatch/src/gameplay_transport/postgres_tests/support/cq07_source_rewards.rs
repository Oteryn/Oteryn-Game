use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    callback: &Callback,
) -> TestResult {
    let (ordinal, expected_root, expected_title, expected_children): (
        u8,
        &str,
        Option<&str>,
        &[u16],
    ) = match callback.key.as_str() {
        "oteryn:quest-callback/m07/realm-draconia-book" => {
            (1, "oteryn:item.tibia.i6533", Some("secret"), &[])
        }
        "oteryn:quest-callback/m07/realm-reward" => (
            2,
            "oteryn:item.tibia.i5949",
            None,
            &[16244, 3659, 9034, 3027, 5882, 5791, 2995, 6570],
        ),
        "oteryn:quest-callback/crystal-wand-twinkiller" => {
            (1, "oteryn:item.tibia.i2835", Some("Twinkiller Rune"), &[])
        }
        _ => return Err("unexpected source reward".into()),
    };
    let at = callback.selector.position.ok_or("source position")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        ordinal,
    )
    .ok_or("source object")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let mut target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let first = super::quest_callbacks_fixtures::wire_use_targets_retained(
        owner,
        admitted,
        &[(1, &target)],
    )
    .await?;
    assert_eq!(first[0].1, UseDisposition::Committed);
    target.expected_revision = map.facts.object_revision(key);
    let second = super::quest_callbacks_fixtures::wire_use_targets_retained(
        owner,
        admitted,
        &[(2, &target)],
    )
    .await?;
    assert_eq!(second[0].1, UseDisposition::Rejected);
    let roots: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity::bigint FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1",
    ).bind(expected_root).fetch_all(&harness.pool).await?;
    assert_eq!(roots, [(expected_root.to_owned(), 1)]);
    let children: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity::bigint FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_instances bag ON bag.item_instance_id=e.parent_item_instance_id WHERE bag.definition_production_key=$1 ORDER BY e.placement_ordinal",
    ).bind(expected_root).fetch_all(&harness.pool).await?;
    assert_eq!(
        children,
        expected_children
            .iter()
            .map(|id| (format!("oteryn:item.tibia.i{id}"), 1))
            .collect::<Vec<_>>()
    );
    if let Some(expected_title) = expected_title {
        let texts: Vec<String> = sqlx::query_scalar(
            "SELECT t.text FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1",
        ).bind(expected_root).fetch_all(&harness.pool).await?;
        assert_eq!(texts.len(), 1);
        assert!(texts[0].contains(expected_title));
    }
    let receipts: Vec<bool> = sqlx::query_scalar(
        "SELECT completes FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&callback.key)
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(receipts, [expected_root == "oteryn:item.tibia.i5949"]);
    Ok(())
}
