#[test]
fn memorial_tabs_preview_without_grant_and_require_explicit_npc_confirmation() {
    use super::{MemorialAction as Action, Panel};
    let mut panel = Panel::default();
    let fields = panel.fields.clone();
    let confirmed = panel.confirmed.clone();
    assert_eq!(panel.memorial.offer().key, "oteryn:outfit.golden");
    assert_eq!(panel.memorial.phrase(Action::Ask), Some("outfit"));
    panel.memorial.navigate(Action::Tokens);
    assert_eq!(panel.memorial.offer().key, "oteryn:outfit.soil_guardian");
    assert_eq!(
        panel.memorial.phrase(Action::Ask),
        Some("basic soil guardian outfit")
    );
    panel.memorial.navigate(Action::Next);
    assert_eq!(
        panel.memorial.phrase(Action::Ask),
        Some("addition to the soil guardian outfit")
    );
    panel.memorial.navigate(Action::Gender);
    assert_eq!(panel.memorial.look(), 514);
    assert_eq!(panel.fields, fields);
    assert_eq!(panel.confirmed, confirmed);
    assert_eq!(panel.memorial.phrase(Action::Yes), Some("yes"));
    assert_eq!(panel.memorial.phrase(Action::No), Some("no"));
}
