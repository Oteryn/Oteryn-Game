//! An authored direct child remains under its original source chest's once claim.
use super::{CandidatePlacement, Value, only};

pub(super) fn apply(placement: &mut CandidatePlacement, recipe: &Value) -> bool {
    let Some(selection) = recipe.get("selected_leaf") else {
        return true;
    };
    if !only(selection, &["source_unique_id", "path", "item", "count"])
        || selection.get("source_unique_id").and_then(Value::as_u64) != placement.crystal_unique_id
        || placement.crystal_unique_id.is_none()
    {
        return false;
    }
    // Existing native container proofs describe at most twenty direct children.
    // A zero-based single ordinal selects a leaf; it never recursively expands a bag.
    let Some(path) = selection.get("path").and_then(Value::as_array) else {
        return false;
    };
    let [ordinal] = path.as_slice() else {
        return false;
    };
    let Some(index) = ordinal.as_u64().and_then(|n| usize::try_from(n).ok()) else {
        return false;
    };
    let Some(source) = recipe
        .get("contents")
        .and_then(Value::as_array)
        .and_then(|v| v.get(index))
    else {
        return false;
    };
    if selection.get("item") != source.get("item") || selection.get("count") != source.get("count")
    {
        return false;
    }
    let Some(child) = placement.contents.get(index).cloned() else {
        return false;
    };
    placement.item = child.item;
    placement.count = child.count;
    placement.key_number = child.key_number;
    placement.root_text = child.text;
    placement.contents.clear();
    true
}
