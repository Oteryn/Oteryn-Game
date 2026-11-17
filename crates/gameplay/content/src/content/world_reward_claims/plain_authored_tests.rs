//! RCAUTH: authored (non-donor) written text and a filled container on a plain claim.
use super::*;

fn item(number: u32) -> Value {
    serde_json::json!({"family": "Item", "key": format!("oteryn:item.tibia.i{number}"),
        "revision": "definition-r1"})
}

fn plain(reward: Value) -> Value {
    serde_json::json!({
        "claim": {"per": "character", "repeat": {"kind": "once"}},
        "identity": {"key": "oteryn:reward-claim.kv.rcauth", "revision": "reward-claim-r1"},
        "placements": [{
            "appearance_tibia_id": 2472,
            "reward": reward,
            "source_binding": {
                "legacy_unique_ids": [{"server": "crystalserver", "unique_id": 6125}],
                "project_position": {"x": 100, "y": 100, "z": 7}
            }
        }],
        "provenance": {"pilot_key": "fixture", "pilot_revision": "fixture"},
        "quest": {"family": "Quest", "key": "canary:quest/fixture", "revision": "fixture"},
        "readiness": "ready"
    })
}

struct Base;
struct Tile;
impl RewardClaimBase for Base {
    type Tile<'a> = Tile;
    fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
        (x == 100 && y == 100 && floor == -7).then_some(Tile)
    }
}
impl RewardClaimTile for Tile {
    fn depths(&self) -> &[u8] {
        &[0, 0]
    }
    fn unique(&self, ordinal: u8) -> Option<UniqueEntry> {
        (ordinal == 1).then_some(UniqueEntry {
            unique: 6125,
            appearance: Some(2472),
        })
    }
}

fn served(definition: &Value) -> Option<BoundPlacement> {
    let shard = serde_json::json!({"records": [{"definition": definition}]}).to_string();
    let report = bind_reward_claims(&Base, &[&shard], &|_| true).ok()?;
    let [claim] = report.served.as_slice() else {
        return None;
    };
    claim.placements.first().cloned()
}

#[test]
fn plain_claim_container_children_carry_authored_page_text() -> Result<(), String> {
    let page = "Twinkiller rune.\nAuthored page body.";
    let definition = plain(serde_json::json!({
        "container": item(2853),
        "items": [
            {"count": 1, "item": item(11609)},
            {"count": 1, "item": item(2835), "written_text": {"text": page}}
        ]
    }));
    let placement = served(&definition).ok_or("bag claim must be served")?;
    let [root] = placement.placement.items.as_slice() else {
        return Err("one root container".into());
    };
    assert_eq!(
        (root.item.key().as_str(), root.count),
        ("oteryn:item.tibia.i2853", 1)
    );
    assert!(placement.root_text.is_none());
    let children: Vec<_> = placement
        .contents
        .iter()
        .map(|child| {
            (
                child.item.key().as_str(),
                child.count,
                child.text.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        children,
        [
            ("oteryn:item.tibia.i11609", 1, None),
            ("oteryn:item.tibia.i2835", 1, Some(page)),
        ]
    );
    Ok(())
}

#[test]
fn plain_claim_leaf_carries_authored_root_text() -> Result<(), String> {
    let body = "Map of the known world.";
    let definition = plain(serde_json::json!({
        "items": [{"count": 1, "item": item(2822)}],
        "written_text": {"text": body}
    }));
    let placement = served(&definition).ok_or("text claim must be served")?;
    let [root] = placement.placement.items.as_slice() else {
        return Err("one root item".into());
    };
    assert_eq!(
        (root.item.key().as_str(), root.count),
        ("oteryn:item.tibia.i2822", 1)
    );
    assert_eq!(placement.root_text.as_deref(), Some(body));
    assert!(placement.contents.is_empty());
    Ok(())
}

#[test]
fn plain_authored_text_and_container_refuse_partial_or_ambiguous_shapes() {
    let text = serde_json::json!({"text": "body"});
    let cases = [
        // Root text on a stack, on several roots, or beside a container.
        serde_json::json!({"items": [{"count": 2, "item": item(2822)}], "written_text": text}),
        serde_json::json!({"items": [{"count": 1, "item": item(2822)},
            {"count": 1, "item": item(2823)}], "written_text": text}),
        serde_json::json!({"container": item(2853), "items": [{"count": 1, "item": item(11609)}],
            "written_text": text}),
        // Empty, oversized, NUL or source-shaped text.
        serde_json::json!({"items": [{"count": 1, "item": item(2822)}],
            "written_text": {"text": ""}}),
        serde_json::json!({"items": [{"count": 1, "item": item(2822)}],
            "written_text": {"text": "x".repeat(4001)}}),
        serde_json::json!({"items": [{"count": 1, "item": item(2822)}],
            "written_text": {"text": "a\0b"}}),
        serde_json::json!({"items": [{"count": 1, "item": item(2822)}],
            "written_text": {"text": "body", "item": null}}),
        // Child text outside a container, on a stack, or a container without children.
        serde_json::json!({"items": [{"count": 1, "item": item(2835), "written_text": text}]}),
        serde_json::json!({"container": item(2853),
            "items": [{"count": 2, "item": item(2835), "written_text": text}]}),
        serde_json::json!({"container": item(2853), "items": []}),
        serde_json::json!({"container": item(2853),
            "items": vec![serde_json::json!({"count": 1, "item": item(3031)}); 21]}),
    ];
    for (index, reward) in cases.into_iter().enumerate() {
        assert!(
            served(&plain(reward)).is_none(),
            "accepted invalid authored shape {index}"
        );
    }
    // A key variant keeps its single-key grammar.
    let mut variant = plain(serde_json::json!({
        "container": item(2853), "items": [{"count": 1, "item": item(11609)}]}));
    variant["placements"][0]["reward"]["key_binding"] = "canary:door-key/3012".into();
    assert!(matches!(
        candidate_placement(&variant["placements"][0], true),
        Ok(None)
    ));
}

#[test]
fn plain_authored_terms_change_the_claim_authority() -> Result<(), String> {
    let reward = |text: &str| {
        serde_json::json!({"container": item(2853), "items": [
            {"count": 1, "item": item(11609)},
            {"count": 1, "item": item(2835), "written_text": {"text": text}}]})
    };
    let digest = |definition: &Value| -> Result<[u8; 32], String> {
        let shard = serde_json::json!({"records": [{"definition": definition}]}).to_string();
        let report = bind_reward_claims(&Base, &[&shard], &|_| true).map_err(|e| e.to_string())?;
        Ok(report.served.first().ok_or("served")?.authority_digest)
    };
    assert_ne!(
        digest(&plain(reward("first page")))?,
        digest(&plain(reward("second page")))?
    );
    Ok(())
}

#[test]
fn purple_tome_maps_carry_their_authored_text_in_the_embedded_shards() -> Result<(), String> {
    let expected = [
        (
            "oteryn:reward-claim.kv.purpletome1",
            "oteryn:item.tibia.i2822",
            "You see a map of our world Tibia",
        ),
        (
            "oteryn:reward-claim.kv.purpletome2",
            "oteryn:item.tibia.i2823",
            "You see a map of the surface of the Fields of Glory. There are many red lines and \
             two golden points on it. You wonder what their meaning is.",
        ),
    ];
    for (key, item, text) in expected {
        let definition = REWARD_CLAIM_SHARDS
            .iter()
            .filter_map(|shard| serde_json::from_str::<Value>(shard).ok())
            .flat_map(|shard| shard["records"].as_array().cloned().unwrap_or_default())
            .map(|row| row["definition"].clone())
            .find(|definition| definition["identity"]["key"] == key)
            .ok_or_else(|| format!("missing {key}"))?;
        let placement = candidate_placement(&definition["placements"][0], false)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("{key} is not served"))?;
        assert_eq!((placement.item.key().as_str(), placement.count), (item, 1));
        assert_eq!(placement.root_text.as_deref(), Some(text), "{key}");
        assert!(placement.contents.is_empty());
    }
    Ok(())
}
