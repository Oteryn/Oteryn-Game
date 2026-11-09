use super::ForgeMarker;

/// The creature's existing name label is the native window for this passive projection.
pub fn label(name: &str, marker: Option<ForgeMarker>) -> String {
    match marker {
        Some(ForgeMarker::Fiendish) => format!("[Fiendish] {name}"),
        Some(ForgeMarker::Influenced(stacks)) => {
            lfmt!("[Influenced {stacks}] {name}", name = name, stacks = stacks)
        }
        None => name.to_owned(),
    }
}
