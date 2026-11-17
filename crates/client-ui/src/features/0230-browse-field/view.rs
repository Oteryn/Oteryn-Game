pub fn panel_text(p: &super::BrowseFieldPanel) -> String {
    let mut s = lfmt!(
        "Browse Field\nTile: {}, {}, {}\nPage: {}",
        p.x,
        p.y,
        p.floor,
        p.index
    );
    if let Some(f) = &p.feedback {
        s.push('\n');
        s.push_str(f)
    }
    s
}
