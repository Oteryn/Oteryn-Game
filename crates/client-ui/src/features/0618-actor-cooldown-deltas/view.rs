/// oteryn-choice: show the server's reported remaining cooldown, without claiming local cast authority.
#[must_use]
pub fn panel_text(
    panel: &super::state::ActionBarPanel,
    catalog: &oteryn_client_runtime::spell_catalog::CatalogState,
) -> String {
    use std::fmt::Write;
    let now = std::time::Instant::now();
    let mut text = String::from(tr!(
        "Assign server cast indices (empty clears a slot). Ctrl+Shift+U: catalogue\r\n"
    ));
    for (slot, field) in panel.bar.fields().iter().enumerate() {
        let entry = field.parse::<u32>().ok().and_then(|id| {
            catalog
                .entries()
                .iter()
                .find(|entry| entry.cast_index == id)
        });
        let remaining = field.parse().ok().map_or(0, |spell| {
            panel.remaining_ms_at(
                spell,
                entry.map_or(&[], |entry| entry.cooldown_groups.as_slice()),
                now,
            )
        });
        let _ = lwrite!(
            text,
            "{}: {} {} | Server cooldown: {} ms\r\n",
            slot + 1,
            field,
            entry.map_or("", |entry| entry.name.as_str()),
            if panel.bar.last_cooldown_observation().is_some() {
                remaining.to_string()
            } else {
                tr!("unknown").into()
            }
        );
    }
    text.push_str(&panel.feedback);
    text
}
