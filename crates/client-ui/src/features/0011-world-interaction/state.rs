// Menu projection contains only actions supported by the negotiated session and target.
pub use oteryn_client_runtime::world_interaction::{Action, Command, Target};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub tile: Option<oteryn_renderer::TileCoord>,
    pub binding: Option<([u8; 32], [u8; 32], u64)>,
    pub target: Target,
    pub entries: Vec<(&'static str, Action)>,
}
impl Menu {
    pub fn new(target: Target, look: bool, bed: bool) -> Self {
        let mut entries = Vec::new();
        if look {
            entries.push(("Look", Action::Look));
        }
        if !matches!(target, Target::Creature(_)) {
            entries.extend([("Use", Action::Use), ("Open", Action::Open)]);
        }
        // Client item data has no bed semantics, and an opaque handle does not prove a base item.
        if bed && matches!(target, Target::Object(_)) {
            entries.push(("Bed", Action::BedInteract));
        }
        Self {
            tile: None,
            binding: None,
            target,
            entries,
        }
    }
}
#[derive(Debug, Default)]
pub struct State {
    pub look: bool,
    pub bed: bool,
    pub pending: bool,
    pub feedback: Option<String>,
    pub feedback_at: Option<std::time::Instant>,
}

impl State {
    pub fn set_feedback(&mut self, text: String, now: std::time::Instant) {
        self.feedback = Some(text);
        self.feedback_at = Some(now);
    }
    pub fn feedback(&self, now: std::time::Instant) -> Option<String> {
        let at = self.feedback_at?;
        if now.saturating_duration_since(at) >= std::time::Duration::from_secs(10) {
            return None;
        }
        // Both the HUD and its window-title projection are bounded, single-line text.
        self.feedback.as_ref().map(|text| {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(240)
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_gates_actions_by_identity_and_negotiation() {
        let creature = Target::Creature(oteryn_client_runtime::world_interaction::EntityRef {
            identity: [1; 16],
            generation: 1,
        });
        assert!(Menu::new(creature.clone(), false, true).entries.is_empty());
        assert_eq!(
            Menu::new(creature, true, true).entries,
            vec![("Look", Action::Look)]
        );
        let item = Target::Item(std::num::NonZeroU64::MIN);
        assert_eq!(Menu::new(item.clone(), true, true).entries.len(), 3);
        assert_eq!(Menu::new(item, false, false).entries.len(), 2);
    }

    #[test]
    fn secondary_pointer_routes_menu_and_respects_modal_and_focus()
    -> Result<(), oteryn_input_actions::InputError> {
        use crate::world_interaction::MouseActions;
        use oteryn_input_actions::*;
        let moved = NormalizedInputEvent::PointerMoved {
            position: PointerPosition::new(
                PointerCoordinate::new(40)?,
                PointerCoordinate::new(50)?,
            ),
            motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
        };
        let button = |state| NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state,
            modifiers: Modifiers::NONE,
        };
        let mut actions = MouseActions::new()?;
        let clicks = actions.route(&[moved, button(ButtonState::Pressed)]);
        assert_eq!(clicks.len(), 1);
        assert!(clicks[0].secondary);
        assert_eq!((clicks[0].x, clicks[0].y), (40.0, 50.0));
        actions.context_closed();
        assert_eq!(actions.route(&[button(ButtonState::Pressed)]).len(), 1);
        actions.route(&[button(ButtonState::Released)]);
        actions.set_modal_active(true)?;
        assert!(actions.route(&[button(ButtonState::Pressed)]).is_empty());
        actions.context_closed();
        assert!(actions.route(&[button(ButtonState::Pressed)]).is_empty());
        actions.route(&[button(ButtonState::Released)]);
        actions.set_modal_active(false)?;
        actions.route(&[
            NormalizedInputEvent::FocusChanged { focused: false },
            NormalizedInputEvent::FocusChanged { focused: true },
        ]);
        assert!(actions.route(&[button(ButtonState::Pressed)]).is_empty());
        Ok(())
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn handle_items_do_not_offer_bed_without_base_or_definition_facts() {
        let base = Menu::new(
            Target::Object(
                oteryn_client_runtime::world_interaction::world_object::WorldObjectTarget {
                    placement: vec![1; 40],
                    expected_revision: 0,
                },
            ),
            true,
            true,
        );
        assert_eq!(
            base.entries
                .iter()
                .map(|(label, _)| *label)
                .collect::<Vec<_>>(),
            vec!["Look", "Use", "Open", "Bed"]
        );
        let menu = Menu::new(Target::Item(std::num::NonZeroU64::MIN), true, true);
        assert_eq!(
            menu.entries
                .iter()
                .map(|(label, _)| *label)
                .collect::<Vec<_>>(),
            vec!["Look", "Use", "Open"]
        );
    }
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    #[test]
    fn context_feedback_expires_and_title_projection_is_single_line_and_bounded() {
        let now = std::time::Instant::now();
        let mut state = State::default();
        state.set_feedback(format!("Look\r\n{}\ttext", "界".repeat(500)), now);
        let text = state.feedback(now).unwrap_or_default();
        assert!(!text.contains(['\r', '\n', '\t']));
        assert_eq!(text.chars().count(), 240);
        assert_eq!(
            state.feedback(now + std::time::Duration::from_secs(10)),
            None
        );
        state.set_feedback("Waiting for server".into(), now);
        assert_eq!(state.feedback(now).as_deref(), Some("Waiting for server"));
    }
}
