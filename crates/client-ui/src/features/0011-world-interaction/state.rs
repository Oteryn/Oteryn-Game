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
    pub fn with_targeting(mut self, enabled: bool) -> Self {
        if enabled && matches!(self.target, Target::Item(_)) {
            self.entries.push((tr!("Use with…"), Action::UseWith));
            self.entries.push((tr!("Use rune…"), Action::UseRune));
        }
        self
    }

    pub fn new(target: Target, look: bool, bed: bool) -> Self {
        let mut entries = Vec::new();
        if look {
            entries.push((tr!("Look"), Action::Look));
        }
        if !matches!(target, Target::Creature(_)) {
            entries.extend([(tr!("Use"), Action::Use), (tr!("Open"), Action::Open)]);
        }
        // Client item data has no bed semantics, and an opaque handle does not prove a base item.
        if bed && matches!(target, Target::Object(_)) {
            entries.push((tr!("Bed"), Action::BedInteract));
        }
        Self {
            tile: None,
            binding: None,
            target,
            entries,
        }
    }
}
/// Stable composition of the world, NPC, trade and party actions for one clicked target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAction {
    World(Action),
    Talk,
    NpcTrade,
    PlayerTrade,
    Invite,
    Party,
}

pub fn context_entries(
    world: Option<&Menu>,
    npc: bool,
    trade: bool,
    party: bool,
) -> Vec<(&'static str, ContextAction)> {
    let mut entries = Vec::new();
    if let Some(world) = world {
        entries.extend(
            world
                .entries
                .iter()
                .map(|(label, action)| (*label, ContextAction::World(*action))),
        );
    }
    if npc {
        entries.extend([
            (tr!("Talk"), ContextAction::Talk),
            (tr!("Trade"), ContextAction::NpcTrade),
        ]);
    }
    if trade {
        entries.push((tr!("Trade with this player"), ContextAction::PlayerTrade));
    }
    if party {
        entries.extend([
            (tr!("Invite to party"), ContextAction::Invite),
            (tr!("Party (Ctrl+Shift+G)"), ContextAction::Party),
        ]);
    }
    entries
}

#[derive(Debug, Default)]
pub struct State {
    pub targeting: Option<oteryn_client_runtime::inventory::ItemHandle>,
    pub rune_targeting: bool,
    pub use_with: bool,
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
        let fresh = self.feedback_at.is_some_and(|at| {
            now.saturating_duration_since(at) < std::time::Duration::from_secs(10)
        });
        if !fresh {
            return self
                .targeting
                .map(|_| crate::item_use::view::targeting_hint());
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

#[cfg(test)]
mod targeting_tests {
    use super::*;
    #[test]
    fn targeting_can_be_cancelled_and_consumed_only_once() {
        let mut state = State::default();
        let source = std::num::NonZeroU64::MIN;
        state.begin_targeting(source);
        assert_eq!(state.targeting, Some(source));
        assert!(
            state
                .feedback(std::time::Instant::now() + std::time::Duration::from_secs(30))
                .is_some_and(|text| text.contains("Escape"))
        );
        assert!(state.cancel_targeting());
        assert!(!state.cancel_targeting());
        state.begin_targeting(source);
        state.set_feedback("Use refused".into(), std::time::Instant::now());
        assert_eq!(
            state.feedback(std::time::Instant::now()).as_deref(),
            Some("Use refused")
        );
        crate::item_use::source_current(&mut state, false);
        assert_eq!(state.targeting, None);
    }
}

#[cfg(test)]
mod use_with_menu_tests {
    use super::*;
    #[test]
    fn use_with_requires_negotiation_and_a_handle_source() {
        let item = Target::Item(std::num::NonZeroU64::MIN);
        assert!(
            Menu::new(item.clone(), false, false)
                .with_targeting(true)
                .entries
                .contains(&("Use with…", Action::UseWith))
        );
        assert!(
            !Menu::new(item, false, false)
                .with_targeting(false)
                .entries
                .iter()
                .any(|(_, action)| *action == Action::UseWith)
        );
        assert!(
            Menu::new(
                Target::Creature(oteryn_client_runtime::world_interaction::EntityRef {
                    identity: [1; 16],
                    generation: 1
                }),
                false,
                false
            )
            .with_targeting(true)
            .entries
            .is_empty()
        );
    }
}
