pub fn panel_text(p: &super::HotkeyEquipPanel) -> String {
    let mut s = format!("Hotkey Equip\nSelected item: {}", p.item_definition);
    if let Some(v) = p.outcome {
        s.push_str(&format!("\nResult: {v:?}"))
    }
    if let Some(f) = &p.feedback {
        s.push('\n');
        s.push_str(f)
    }
    s
}
