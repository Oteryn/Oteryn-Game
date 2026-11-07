//! Input mapping for the live harness: normalized key/pointer events -> [`LiveCommand`], through
//! the shared `oteryn-input-actions` router for keys and the pure click mapping for the pointer.

use super::model::{
    LiveCommand, RenderModel, Viewport, command_for_click, step_direction_for_action,
};
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ButtonState, ContextDefinition, ContextId,
    ContextKind, InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifiers, MouseButton,
    NormalizedInputEvent, RepeatPolicy,
};

const BINDINGS: [(KeyCode, &str); 8] = [
    (KeyCode::ARROW_UP, "move.north"),
    (KeyCode::ARROW_RIGHT, "move.east"),
    (KeyCode::ARROW_DOWN, "move.south"),
    (KeyCode::ARROW_LEFT, "move.west"),
    (KeyCode::KEY_W, "move.north"),
    (KeyCode::KEY_D, "move.east"),
    (KeyCode::KEY_S, "move.south"),
    (KeyCode::KEY_A, "move.west"),
];

/// Held-key routing plus the last pointer position (a click carries no position of its own).
#[derive(Debug)]
pub struct LiveInput {
    router: InputRouter,
    pointer: Option<(i32, i32)>,
}

impl LiveInput {
    /// Arrow keys and WASD move; held keys repeat.
    ///
    /// # Errors
    ///
    /// Returns the input crate's error if the binding map is refused.
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("gameplay".to_owned())?;
        let mut bindings = Vec::new();
        for (key, action) in BINDINGS {
            bindings.push(Binding::new(
                gameplay.clone(),
                InputChord::new(Modifiers::NONE, vec![InputAtom::Key(key)])?,
                ActionId::new(action.to_owned())?,
                RepeatPolicy::Allow,
            ));
        }
        let map = BindingMap::new(
            vec![ContextDefinition::new(
                gameplay.clone(),
                ContextKind::Gameplay,
                1,
            )],
            bindings,
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self {
            router,
            pointer: None,
        })
    }

    /// The command one event asks for, if any.
    pub fn map_event(
        &mut self,
        event: &NormalizedInputEvent,
        view: Viewport,
        model: &RenderModel,
    ) -> Option<LiveCommand> {
        match event {
            NormalizedInputEvent::PointerMoved { position, .. } => {
                self.pointer = Some((position.x().get(), position.y().get()));
                None
            }
            NormalizedInputEvent::MouseButton {
                button,
                state: ButtonState::Pressed,
                ..
            } if *button == MouseButton::PRIMARY => {
                let (px, py) = self.pointer?;
                command_for_click(view, model, px, py)
            }
            NormalizedInputEvent::Key { .. } => self
                .router
                .process(event)
                .iter()
                .filter(|action| {
                    matches!(action.phase(), ActionPhase::Started | ActionPhase::Repeated)
                })
                .find_map(|action| step_direction_for_action(action.action().as_str()))
                .map(LiveCommand::Step),
            _ => None,
        }
    }
}
