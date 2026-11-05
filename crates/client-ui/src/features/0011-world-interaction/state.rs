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
            if bed {
                entries.push(("Bed interaction", Action::BedInteract));
            }
        }
        if bed && matches!(target, Target::Item(_)) {
            entries.extend([
                ("Bed: fist", Action::Bed(1)),
                ("Bed: club", Action::Bed(2)),
                ("Bed: sword", Action::Bed(3)),
                ("Bed: axe", Action::Bed(4)),
                ("Bed: distance", Action::Bed(5)),
                ("Bed: magic level", Action::Bed(6)),
            ]);
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
    pub look_text: Option<String>,
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
        assert_eq!(Menu::new(item.clone(), true, true).entries.len(), 10);
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
