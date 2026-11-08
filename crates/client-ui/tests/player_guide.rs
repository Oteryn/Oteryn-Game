use oteryn_client_ui::guide::{Guide, Page};
#[test]
fn player_guide_switches_from_controls_to_recommendations() {
    let mut guide = Guide::default();
    assert!(guide.text().contains("arrow keys"));
    guide.select(Page::Recommendations);
    assert!(guide.text().contains("supplies"));
    assert!(guide.text().contains("server-save"));
    guide.select(Page::Basics);
    assert!(guide.text().contains("right-click"));
}
