use oteryn_client_ui::vip::{FriendsFilter, VipPanel, friends_text};
#[test]
fn friends_view_separates_confirmed_contacts_and_pending_invitations() {
    let mut panel = VipPanel { selected: true, view: Some(r#"{"entries":[{"entry":1,"name":"Alice","confirmed":true},{"entry":2,"name":"Bob","confirmed":false}],"consents":[{"handle":9,"name":"Cara","accepted":false},{"handle":10,"name":"Dan","accepted":true}]}"#.into()), ..Default::default() };
    panel.friends_filter = FriendsFilter::Confirmed;
    let text = friends_text(&panel);
    assert!(text.contains("Alice"));
    assert!(!text.contains("Bob"));
    panel.friends_filter = FriendsFilter::Invitations;
    let text = friends_text(&panel);
    assert!(text.contains("Cara"));
    assert!(!text.contains("Dan"));
    panel.view = Some("invalid".into());
    assert!(friends_text(&panel).contains("invalid"));
}
