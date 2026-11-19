use super::TestResult;

/// A legacy once claim served by its exact World callback (legacy_world_claims.rs)
/// commits through the quest writer, which needs the admitted quest session.
pub(super) fn legacy_world_owned(claim: &str) -> TestResult<bool> {
    use oteryn_gameplay_quests::quest::world_callbacks::{legacy_world_claim, load};
    for callback in load()?.iter() {
        if legacy_world_claim(callback)?.is_some_and(|binding| binding.key == claim) {
            return Ok(true);
        }
    }
    Ok(false)
}
