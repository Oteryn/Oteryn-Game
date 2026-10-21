//! Shared source traversal for gameplay ownership and restricted-call guards.

use super::portable_relative_path;
use std::fs;
use std::path::Path;

/// Every `.rs` file under `dir` as (portable path relative to `root`, contents).
pub(super) fn collect_rust_sources(
    root: &Path,
    dir: &Path,
    files: &mut Vec<(String, String)>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|error| format!("cannot read {}: {error}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot read {}: {error}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_rust_sources(root, &path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("{} is outside the workspace", path.display()))?;
            let contents = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            files.push((portable_relative_path(relative)?, contents));
        }
    }
    Ok(())
}
