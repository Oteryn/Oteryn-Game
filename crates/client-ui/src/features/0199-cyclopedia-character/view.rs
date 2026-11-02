use super::CyclopediaCharacterPanel;
#[must_use]
pub fn panel_text(p: &CyclopediaCharacterPanel) -> String {
    let state = p
        .result
        .as_deref()
        .unwrap_or("No server character view yet");
    match p.feedback {
        Some(f) => format!("Cyclopedia character\n{state}\n{f}"),
        None => format!("Cyclopedia character\n{state}"),
    }
}
