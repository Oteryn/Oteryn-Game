pub fn panel_text(p: &super::HotkeyEquipPanel) -> String {
    let mut s = lfmt!("Hotkey Equip\nSelected item: {}", p.item_definition);
    s.push_str(tr!("\nSelect a key and an item, then Assign item. F1–F12 equip or toggle the item.\nWeapon, shield and ammunition swaps and jewellery cooldowns are checked by the server."));
    if !p.selected {
        s.push_str(tr!("\nItem hotkeys are unavailable in this session."));
    }
    if p.pending {
        s.push_str(tr!("\nWaiting for the server; item hotkeys are locked."));
    }
    if let Some(v) = p.outcome {
        s.push_str(&lfmt!("\nResult: {v:?}", v = v))
    }
    if let Some(f) = &p.feedback {
        s.push('\n');
        s.push_str(f)
    }
    s
}
