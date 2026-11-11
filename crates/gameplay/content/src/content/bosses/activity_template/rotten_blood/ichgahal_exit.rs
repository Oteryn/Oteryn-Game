//! Crystal rotten_ichgahal.lua:18 -> actual pinned OTBM Teleport34111 attribute8.
//! The raw exit stays blocked; only its proven arrival can compose the return.
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

fn matches_capture(capture: &serde_json::Value) -> bool {
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
    let teleport = raw["items"]
        == serde_json::json!([
            {"server_item_id": 42852, "depth": 0, "attributes": {}},
            {"server_item_id": 34111, "depth": 0,
             "attributes": {"dest": [FINAL.x, FINAL.y, FINAL.z]}}
        ]);
    let stack = target["items"].as_array().is_some_and(|items| {
        items.len() == 2
            && items[0]["server_item_id"] == 36344
            && items[1]["server_item_id"] == 292
            && items
                .iter()
                .all(|item| item["depth"] == 0 && item["attributes"] == serde_json::json!({}))
    });
    teleport
        && stack
        && capture["source_revision"] == CRYSTAL_REVISION
        && capture["source_closure"].as_array().is_some_and(|closure| {
            closure.iter().any(|source| {
                source["path"] == "data-global/world/world.otbm"
                    && source["git_blob"] == "e61658ec0ab7b70ec35025ae969426d6fc57ae75"
            })
        })
}

fn capture_matches() -> bool {
    static VERIFIED: OnceLock<bool> = OnceLock::new();
    *VERIFIED.get_or_init(|| {
        serde_json::from_slice::<serde_json::Value>(PinnedActivitySourceRoom::Ichgahal.bytes())
            .is_ok_and(|capture| matches_capture(&capture))
    })
}

pub(super) fn qualify(
    room: &QualifiedNativeEntryRoom,
    raw: LogicalCell,
    resolved: LogicalCell,
) -> Result<LogicalCell, ActivityTemplateRefusal> {
    use ActivityTemplateRefusal as R;
    if raw != RAW || resolved != FINAL || !capture_matches() {
        return Err(R::InvalidSourceCapture);
    }
    // The sealed room proves the actual source stack and attributes. Its
    // separate destination must also be KNOWN walkable in the current Bundle.
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

    #[test]
    fn pinned_ichgahal_arrival_rejects_changed_destination_stack_and_source() -> Result<(), String>
    {
        let capture: serde_json::Value =
            serde_json::from_slice(PinnedActivitySourceRoom::Ichgahal.bytes())
                .map_err(|error| error.to_string())?;
        assert!(matches_capture(&capture));
        assert!(capture_matches());
        let mut changed = capture.clone();
        let raw = changed["tiles"]
            .as_array_mut()
            .and_then(|tiles| {
                tiles.iter_mut().find(|tile| {
                    tile["source_position"] == serde_json::json!([RAW.x, RAW.y, RAW.z])
                })
            })
            .ok_or("actual Ichgahal raw tile")?;
        let teleport = raw["items"]
            .as_array_mut()
            .and_then(|items| {
                items
                    .iter_mut()
                    .find(|item| item["server_item_id"] == 34111)
            })
            .ok_or("actual Ichgahal Teleport")?;
        teleport["attributes"]["dest"] = serde_json::json!([FINAL.x + 1, FINAL.y, FINAL.z]);
        assert!(!matches_capture(&changed));
        let mut changed = capture.clone();
        let target = changed["tiles"]
            .as_array_mut()
            .and_then(|tiles| {
                tiles.iter_mut().find(|tile| {
                    tile["source_position"] == serde_json::json!([FINAL.x, FINAL.y, FINAL.z])
                })
            })
            .ok_or("actual Ichgahal arrival tile")?;
        target["items"][1]["server_item_id"] = serde_json::json!(42671);
        assert!(
            !matches_capture(&changed),
            "Vemiath's stack is not Ichgahal's"
        );
        let mut changed = capture.clone();
        let target = changed["tiles"]
            .as_array_mut()
            .and_then(|tiles| {
                tiles.iter_mut().find(|tile| {
                    tile["source_position"] == serde_json::json!([FINAL.x, FINAL.y, FINAL.z])
                })
            })
            .ok_or("actual Ichgahal arrival tile")?;
        target["items"][0]["attributes"] = serde_json::json!({"15": 2});
        assert!(!matches_capture(&changed));
        let mut changed = capture;
        changed["source_revision"] = serde_json::json!("unverified");
        assert!(!matches_capture(&changed));
        Ok(())
    }
}
