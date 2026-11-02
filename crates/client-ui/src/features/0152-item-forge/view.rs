pub fn panel_text(p: &super::ItemForgePanel) -> String {
    let mut s = format!(
        "Item Forge\nAction: {:?}\nItem: {} / {}",
        p.operation, p.first_item, p.second_item
    );
    if let Some(f) = &p.feedback {
        s.push('\n');
        s.push_str(f)
    }
    s
}
