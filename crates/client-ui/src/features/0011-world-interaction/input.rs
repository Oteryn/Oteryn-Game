// Pointer-click routing and screen-to-tile mapping.
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputError, InputRouter, Modifiers, MouseButton, NormalizedInputEvent,
    RepeatPolicy,
};
use oteryn_renderer::{TileCoord, TileView};

pub const CLICK_ACTION: &str = "client.click";
pub const ATTACK_TOGGLE_ACTION: &str = "client.attack_toggle";
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClickAction {
    pub x: f64,
    pub y: f64,
    pub secondary: bool,
    /// Modifier action dispatched on release; dragging cancels it.
    pub shortcut: Option<oteryn_input_actions::Modifier>,
}
#[must_use]
pub fn click_tile(view: &TileView, x: f64, y: f64) -> Option<TileCoord> {
    #[allow(clippy::cast_possible_truncation)]
    view.screen_to_tile(x as f32, y as f32)
}
/// Registration marker for the primary-pointer default.
pub fn default_click_binding() {}
/// Registration marker for the secondary-pointer menu.
pub fn default_context_binding() {}
#[derive(Debug)]
pub struct MouseActions {
    router: InputRouter,
    gameplay: ContextId,
    text: ContextId,
    modal: ContextId,
    pointer: Option<(f64, f64)>,
    pending: Option<(MouseButton, ClickAction, Modifiers)>,
}
impl MouseActions {
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("gameplay".to_owned())?;
        let text = ContextId::new("text".to_owned())?;
        let modal = ContextId::new("modal".to_owned())?;
        let mut bindings = vec![
            Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Mouse(MouseButton::PRIMARY)],
                )?,
                ActionId::new(CLICK_ACTION.to_owned())?,
                RepeatPolicy::Ignore,
            ),
            Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Mouse(MouseButton::SECONDARY)],
                )?,
                ActionId::new("client.context".to_owned())?,
                RepeatPolicy::Ignore,
            ),
        ];
        for button in [MouseButton::PRIMARY, MouseButton::SECONDARY] {
            for (modifier, action) in [
                (oteryn_input_actions::Modifier::Shift, "client.look"),
                (oteryn_input_actions::Modifier::Control, "client.use"),
                (oteryn_input_actions::Modifier::Alt, ATTACK_TOGGLE_ACTION),
            ] {
                bindings.push(Binding::new(
                    gameplay.clone(),
                    InputChord::new(Modifiers::one(modifier), vec![InputAtom::Mouse(button)])?,
                    ActionId::new(action.to_owned())?,
                    RepeatPolicy::Ignore,
                ));
            }
        }
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
                ContextDefinition::new(modal.clone(), ContextKind::Modal, 20),
            ],
            bindings,
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self {
            router,
            gameplay,
            text,
            modal,
            pointer: None,
            pending: None,
        })
    }
    /// Native popup loops can consume the release; clear the held secondary button on dismissal.
    pub fn context_closed(&mut self) {
        self.pending = None;
        self.router.process(&NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state: oteryn_input_actions::ButtonState::Released,
            modifiers: Modifiers::NONE,
        });
    }
    pub fn set_text_active(&mut self, active: bool) -> Result<(), InputError> {
        if active {
            self.pending = None;
        }
        self.router.set_context_active(&self.text, active).map(drop)
    }
    pub fn set_modal_active(&mut self, active: bool) -> Result<(), InputError> {
        if active {
            self.pending = None;
        }
        self.router
            .set_context_active(&self.modal, active)
            .map(drop)
    }
    #[must_use]
    pub const fn gameplay_context(&self) -> &ContextId {
        &self.gameplay
    }
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<ClickAction> {
        let mut clicks = Vec::new();
        for event in events {
            if let NormalizedInputEvent::PointerMoved { position, .. } = event
                && self.pending.as_ref().is_some_and(|(_, click, _)| {
                    (click.x - f64::from(position.x().get()))
                        .hypot(click.y - f64::from(position.y().get()))
                        > 4.
                })
            {
                self.pending = None;
            }
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())))
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::CaptureChanged { captured: false }
                | NormalizedInputEvent::DeviceLost => {
                    self.pointer = None;
                    self.pending = None;
                }
                NormalizedInputEvent::Key { .. }
                | NormalizedInputEvent::Wheel { .. }
                | NormalizedInputEvent::TextCommitted(_) => continue,
                _ => {}
            }
            for action in self.router.process(event) {
                if matches!(
                    action.action().as_str(),
                    CLICK_ACTION
                        | ATTACK_TOGGLE_ACTION
                        | "client.context"
                        | "client.look"
                        | "client.use"
                ) && action.context() == &self.gameplay
                    && action.phase() == ActionPhase::Started
                    && let Some((x, y)) = self.pointer
                {
                    let shortcut = match action.action().as_str() {
                        "client.look" => Some(oteryn_input_actions::Modifier::Shift),
                        "client.use" => Some(oteryn_input_actions::Modifier::Control),
                        ATTACK_TOGGLE_ACTION => Some(oteryn_input_actions::Modifier::Alt),
                        _ => None,
                    };
                    let click = ClickAction {
                        x,
                        y,
                        secondary: matches!(event, NormalizedInputEvent::MouseButton { button, .. } if *button == MouseButton::SECONDARY),
                        shortcut,
                    };
                    if shortcut.is_some() {
                        if let NormalizedInputEvent::MouseButton {
                            button, modifiers, ..
                        } = event
                        {
                            self.pending = Some((*button, click, *modifiers));
                        }
                    } else {
                        clicks.push(click);
                    }
                }
            }
            if let NormalizedInputEvent::MouseButton {
                button,
                state: oteryn_input_actions::ButtonState::Released,
                modifiers,
            } = event
                && self
                    .pending
                    .as_ref()
                    .is_some_and(|(held, _, _)| held == button)
                && let Some((_, click, held)) = self.pending.take()
                && *modifiers == held
            {
                clicks.push(click);
            }
        }
        clicks
    }
}
