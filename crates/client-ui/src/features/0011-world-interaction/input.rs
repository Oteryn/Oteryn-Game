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
    /// Alt+click on an opponent: toggles attacking it.
    pub toggle_attack: bool,
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
}
impl MouseActions {
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("gameplay".to_owned())?;
        let text = ContextId::new("text".to_owned())?;
        let modal = ContextId::new("modal".to_owned())?;
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
                ContextDefinition::new(modal.clone(), ContextKind::Modal, 20),
            ],
            vec![
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
                        Modifiers::one(oteryn_input_actions::Modifier::Alt),
                        vec![InputAtom::Mouse(MouseButton::PRIMARY)],
                    )?,
                    ActionId::new(ATTACK_TOGGLE_ACTION.to_owned())?,
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
            ],
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
        })
    }
    /// Native popup loops can consume the release; clear the held secondary button on dismissal.
    pub fn context_closed(&mut self) {
        self.router.process(&NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state: oteryn_input_actions::ButtonState::Released,
            modifiers: Modifiers::NONE,
        });
    }
    pub fn set_text_active(&mut self, active: bool) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, active).map(drop)
    }
    pub fn set_modal_active(&mut self, active: bool) -> Result<(), InputError> {
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
            match event {
                NormalizedInputEvent::PointerMoved { position, .. } => {
                    self.pointer =
                        Some((f64::from(position.x().get()), f64::from(position.y().get())))
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => self.pointer = None,
                NormalizedInputEvent::Key { .. }
                | NormalizedInputEvent::Wheel { .. }
                | NormalizedInputEvent::TextCommitted(_) => continue,
                _ => {}
            }
            for action in self.router.process(event) {
                if matches!(
                    action.action().as_str(),
                    CLICK_ACTION | ATTACK_TOGGLE_ACTION | "client.context"
                ) && action.context() == &self.gameplay
                    && action.phase() == ActionPhase::Started
                    && let Some((x, y)) = self.pointer
                {
                    clicks.push(ClickAction {
                        x,
                        y,
                        secondary: action.action().as_str() == "client.context",
                        toggle_attack: action.action().as_str() == ATTACK_TOGGLE_ACTION,
                    });
                }
            }
        }
        clicks
    }
}
