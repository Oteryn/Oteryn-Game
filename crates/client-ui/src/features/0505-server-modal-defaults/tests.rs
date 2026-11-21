use super::*;
#[test]
fn default_and_escape_use_server_ids() {
    let mut panel = crate::server_modal::Panel {
        selected: true,
        defaults: true,
        ..Default::default()
    };
    assert_eq!(keyboard_button(&panel, false), None);
    panel.active = Some(crate::server_modal::Dialog {
        id: 1,
        title: "Choose".into(),
        message: "Choose a destination".into(),
        choices: vec![(9, "Depot".into())],
        buttons: vec![
            (3, "Help".into()),
            (8, "Travel".into()),
            (2, "Cancel".into()),
        ],
        default_button: Some(8),
        cancel_button: Some(2),
        image: None,
    });
    assert_eq!(keyboard_button(&panel, false), Some(8));
    assert_eq!(keyboard_button(&panel, true), Some(2));
}
