//! Crystal rotten_vemiath.lua:18 -> actual pinned OTBM Teleport34111 attribute8.
//! Both coordinates stay explicit. A caller cannot supply another destination.
use super::*;
use std::sync::OnceLock;
const RAW: LogicalCell = LogicalCell {
    x: 33043,
    y: 32344,
    z: 15,
};
const FINAL: LogicalCell = LogicalCell {
    x: 34001,
    y: 31820,
    z: 15,
};
fn capture_matches() -> bool {
    static VERIFIED: OnceLock<bool> = OnceLock::new();
    *VERIFIED.get_or_init(|| {
        let Ok(capture) =
            serde_json::from_slice::<serde_json::Value>(PinnedActivitySourceRoom::Vemiath.bytes())
        else {
            return false;
        };
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
        let teleport = raw["items"].as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item["server_item_id"] == 34111
                    && item["depth"] == 0
                    && item["attributes"]["dest"] == serde_json::json!([FINAL.x, FINAL.y, FINAL.z])
            })
        });
        let stack = target["items"].as_array().is_some_and(|items| {
            items.len() == 2
                && items[0]["server_item_id"] == 36344
                && items[1]["server_item_id"] == 42671
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
    // The room's sealed role already proves every source tile/stack/attribute
    // against its actual Bundle. The destination additionally must be KNOWN
    // walkable; the raw unsupported Teleport tile remains honestly blocked.
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
