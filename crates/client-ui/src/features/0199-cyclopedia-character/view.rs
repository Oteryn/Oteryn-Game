use super::CyclopediaCharacterPanel;
#[must_use]
pub fn panel_text(p: &CyclopediaCharacterPanel) -> String {
    let state = p
        .result
        .as_deref()
        .unwrap_or("No server character view yet");
    let heading = format!(
        "Cyclopedia character — {:?}, offset {}, limit {}",
        p.tab, p.offset, p.limit
    );
    match p.feedback {
        Some(f) => format!("{heading}\r\n{state}\r\n{f}"),
        None => format!("{heading}\r\n{state}"),
    }
}
