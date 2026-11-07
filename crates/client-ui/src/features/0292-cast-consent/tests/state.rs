use super::*;
use crate::cast_control::{Message, Panel};
#[test]
fn s17a_character_consent_is_read_before_fenced_toggle_and_needs_no_cast_control()
-> Result<(), &'static str> {
    let mut panel = Panel {
        consent_selected: true,
        ..Default::default()
    };
    assert!(!panel.allow_shown);
    assert!(!panel.consent_known);
    assert_eq!(protocol::request(&panel, None)?, Message::ConsentQuery);
    assert!(protocol::request(&panel, Some(true)).is_err());
    let bytes = Message::ConsentState {
        revision: 7,
        allowed: false,
    }
    .encode()
    .map_err(|_| "encode")?;
    protocol::apply(&mut panel, &bytes)?;
    assert_eq!(
        protocol::request(&panel, Some(true))?,
        Message::ConsentSet {
            revision: 7,
            allowed: true
        }
    );
    assert!(!panel.allow_shown, "no optimistic consent");
    protocol::apply(
        &mut panel,
        &Message::ConsentState {
            revision: 8,
            allowed: true,
        }
        .encode()
        .map_err(|_| "encode")?,
    )?;
    assert!(panel_text(&panel).contains("shown in casts: on"));
    assert!(protocol::apply(&mut panel, &Message::End.encode().map_err(|_| "encode")?).is_err());
    Ok(())
}
