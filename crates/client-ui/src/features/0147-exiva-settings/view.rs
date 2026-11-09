use super::*;
pub fn panel_text(panel: &ExivaPanel) -> String {
    if !panel.selected {
        return tr!("Exiva settings unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(list) = &panel.list else {
        return tr!("Loading Exiva permissions…").into();
    };
    lfmt!(
        "Exiva allow list ({}/100)\r\n{}",
        list.players.len(),
        list.players
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>()
            .join("\r\n")
    )
}
