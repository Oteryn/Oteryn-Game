mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::{Intent, Panel};
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::raw_feature::house_rotation::{Catalogue, Entry, Target};
    #[test]
    fn explicit_rotation_chord_requires_shift() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::{
            ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
        };
        let chord = Modifiers::one(Modifier::Control).with(Modifier::Alt);
        let code = KeyCode::new(9).map_err(|error| format!("{error:?}"))?;
        let event = |modifiers| NormalizedInputEvent::Key {
            code,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        };
        assert!(!open_key(&[event(chord)]));
        assert!(open_key(&[event(chord.with(Modifier::Shift))]));
        Ok(())
    }
    #[test]
    fn named_rotation_choices_present_authoritative_targets_and_gate_requests() {
        let mut panel = Panel::default();
        assert!(!request(&mut panel, vec![0, 0, 0]));
        panel.selected = true;
        panel.catalogue = Some(Catalogue {
            page: 0,
            more: false,
            entries: vec![Entry {
                label: "Zaoan cabinet".into(),
                target: Target::Item {
                    item: [7; 16],
                    expected_revision: 9,
                },
            }],
        });
        let text = panel_text(&panel);
        assert!(text.contains("1. Zaoan cabinet — Rotate"));
        assert!(!text.contains("07070707"));
        assert!(request(&mut panel, vec![0, 0, 0]));
        assert!(!request(&mut panel, vec![0, 0, 1]));
        assert_eq!(panel.pending_payload, vec![0, 0, 0]);
    }
}
