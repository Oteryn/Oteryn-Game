//! Crystal rotten_ichgahal.lua raw exit -> pinned OTBM Teleport34111 attribute8.
//! The raw tile remains closed; only its genuine, qualified arrival is admitted.
use super::*;
use std::sync::OnceLock;

const RAW: LogicalCell = LogicalCell {
    x: 33004,
    y: 32343,
    z: 15,
};
const FINAL: LogicalCell = LogicalCell {
    x: 34065,
    y: 31717,
    z: 15,
};

fn capture_matches(capture: &serde_json::Value) -> bool {
    let Some(tiles) = capture["tiles"].as_array() else {
        return false;
    };
    let Some(raw) = tiles
        .iter()
        .find(|tile| tile["source_position"] == serde_json::json!([RAW.x, RAW.y, RAW.z]))
    else {
        return false;
    };
    let Some(target) = tiles
        .iter()
        .find(|tile| tile["source_position"] == serde_json::json!([FINAL.x, FINAL.y, FINAL.z]))
    else {
        return false;
    };
    raw["items"]
        == serde_json::json!([
            {"server_item_id": 42852, "depth": 0, "attributes": {}},
            {"server_item_id": 34111, "depth": 0,
             "attributes": {"dest": [FINAL.x, FINAL.y, FINAL.z]}}
        ])
        && target["items"]
            == serde_json::json!([
                {"server_item_id": 36344, "depth": 0, "attributes": {}},
                {"server_item_id": 292, "depth": 0, "attributes": {}}
            ])
        && capture["source_revision"] == CRYSTAL_REVISION
        && capture["source_closure"].as_array().is_some_and(|closure| {
            closure.iter().any(|source| {
                source["path"] == "data-global/world/world.otbm"
                    && source["git_blob"] == "e61658ec0ab7b70ec35025ae969426d6fc57ae75"
            })
        })
}

fn pinned_capture_matches() -> bool {
    static VERIFIED: OnceLock<bool> = OnceLock::new();
    *VERIFIED.get_or_init(|| {
        serde_json::from_slice(PinnedActivitySourceRoom::Ichgahal.bytes())
            .is_ok_and(|capture| capture_matches(&capture))
    })
}

pub(super) fn qualify(
    room: &QualifiedNativeEntryRoom,
    raw: LogicalCell,
    resolved: LogicalCell,
) -> Result<LogicalCell, ActivityTemplateRefusal> {
    use ActivityTemplateRefusal as R;
    if raw != RAW || resolved != FINAL || !pinned_capture_matches() {
        return Err(R::InvalidSourceCapture);
    }
    // The sealed room proves the actual source stack and Teleport attribute.
    // No UNKNOWN cell is opened; the existing Bundle resolver and this current
    // collision proof must agree on the one actual destination.
    if room
        .movement_cells()
        .index()
        .lookup(room.movement_cells().scope(), resolved)
        .ok()
        != Some(crate::content::CollisionClass::Walkable)
    {
        return Err(R::UnknownPlacement);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile_mut(
        capture: &mut serde_json::Value,
        position: LogicalCell,
    ) -> Result<&mut serde_json::Value, String> {
        capture["tiles"]
            .as_array_mut()
            .ok_or("source tiles")?
            .iter_mut()
            .find(|tile| {
                tile["source_position"] == serde_json::json!([position.x, position.y, position.z])
            })
            .ok_or_else(|| "captured source tile".into())
    }

    #[test]
    fn exact_ichgahal_teleport_and_arrival_cannot_be_substituted() -> Result<(), String> {
        let capture: serde_json::Value =
            serde_json::from_slice(PinnedActivitySourceRoom::Ichgahal.bytes())
                .map_err(|error| error.to_string())?;
        assert!(capture_matches(&capture));
        assert!(pinned_capture_matches());

        let mut changed = capture.clone();
        tile_mut(&mut changed, RAW)?["items"][1]["attributes"]["dest"] =
            serde_json::json!([FINAL.x + 1, FINAL.y, FINAL.z]);
        assert!(!capture_matches(&changed));
        let mut changed = capture.clone();
        tile_mut(&mut changed, FINAL)?["items"][1]["server_item_id"] = serde_json::json!(42671);
        assert!(!capture_matches(&changed));
        let mut changed = capture.clone();
        tile_mut(&mut changed, FINAL)?["items"][0]["attributes"] = serde_json::json!({"15": 2});
        assert!(!capture_matches(&changed));
        let mut changed = capture;
        changed["source_revision"] = serde_json::json!("unqualified-revision");
        assert!(!capture_matches(&changed));
        Ok(())
    }
}
