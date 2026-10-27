// Spell-slot key intent mapping.
use oteryn_input_actions::{
    ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId, ContextKind,
    InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifiers, NormalizedInputEvent,
    RepeatPolicy,
};
use std::num::NonZeroU32;
pub const SPELL_SLOTS: u16 = 4;
const FIRST_DIGIT_KEY: u16 = 30;
const ACTION_PREFIX: &str = "client.spell.";
/// Registration marker for spell-slot defaults.
pub fn default_spell_binding() {}
#[derive(Debug)]
pub struct SpellHotkeys {
    router: InputRouter,
    text: ContextId,
}
impl SpellHotkeys {
    pub fn new() -> Result<Self, InputError> {
        let gameplay = ContextId::new("spell-gameplay".to_owned())?;
        let text = ContextId::new("spell-text".to_owned())?;
        let mut bindings = Vec::new();
        for slot in 1..=SPELL_SLOTS {
            bindings.push(Binding::new(
                gameplay.clone(),
                InputChord::new(
                    Modifiers::NONE,
                    vec![InputAtom::Key(KeyCode::new(FIRST_DIGIT_KEY + slot - 1)?)],
                )?,
                ActionId::new(format!("{ACTION_PREFIX}{slot}"))?,
                RepeatPolicy::Ignore,
            ));
        }
        let map = BindingMap::new(
            vec![
                ContextDefinition::new(gameplay.clone(), ContextKind::Gameplay, 0),
                ContextDefinition::new(text.clone(), ContextKind::Text, 10),
            ],
            bindings,
            &[],
        )?;
        let mut router = InputRouter::new(map);
        router.set_context_active(&gameplay, true)?;
        Ok(Self { router, text })
    }
    pub fn set_text_active(&mut self, active: bool) -> Result<(), InputError> {
        self.router.set_context_active(&self.text, active).map(drop)
    }
    pub fn route(&mut self, events: &[NormalizedInputEvent]) -> Vec<NonZeroU32> {
        events
            .iter()
            .flat_map(|event| self.router.process(event))
            .filter(|action| action.phase() == ActionPhase::Started)
            .filter_map(|action| {
                action
                    .action()
                    .as_str()
                    .strip_prefix(ACTION_PREFIX)?
                    .parse()
                    .ok()
            })
            .collect()
    }
}
