use super::*;
/// Registered command-22 adapter used by list, line and tracking controls.
pub fn request(panel: &mut QuestPanel, query: &QuestLogQuery) -> Result<Vec<u8>, QuestRefusal> {
    let result = panel.state.command(query);
    panel.state.refusal = result.as_ref().err().copied();
    result
}
