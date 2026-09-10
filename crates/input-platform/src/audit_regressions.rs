//! Complete adapter-to-router streams for the native UI source-audit findings.
use crate::InputPlatformAdapter;
use crate::adapter::PlatformButtonState::{Pressed, Released};
use crate::adapter::{PlatformButtonState, PlatformEvent};
use oteryn_input_actions::{
    ActionEvent, ActionId, ActionPhase, Binding, BindingMap, ContextDefinition, ContextId,
    ContextKind, InputAtom, InputChord, InputError, InputRouter, KeyCode, Modifier, Modifiers,
    RepeatPolicy,
};
use std::error::Error;

fn router(modifiers: Modifiers) -> Result<InputRouter, InputError> {
    let context = ContextId::new("audit.gameplay".to_owned())?;
    let map = BindingMap::new(
        vec![ContextDefinition::new(
            context.clone(),
            ContextKind::Gameplay,
            0,
        )],
        vec![Binding::new(
            context.clone(),
            InputChord::new(modifiers, vec![InputAtom::Key(KeyCode::KEY_A)])?,
            ActionId::new("audit.action".to_owned())?,
            RepeatPolicy::Allow,
        )],
        &[],
    )?;
    let mut router = InputRouter::new(map);
    router.set_context_active(&context, true)?;
    Ok(router)
}

fn key(
    adapter: &mut InputPlatformAdapter,
    router: &mut InputRouter,
    code: u16,
    state: PlatformButtonState,
    repeat: bool,
) -> Result<Vec<ActionEvent>, Box<dyn Error>> {
    let normalized = adapter.process_platform_event(PlatformEvent::Key {
        code: Some(code),
        state,
        repeat,
        text: None,
        synthetic: false,
    })?;
    Ok(normalized
        .iter()
        .flat_map(|event| router.process(event))
        .collect())
}

#[test]
fn complete_control_chord_starts_and_modifier_release_cancels() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router(Modifiers::one(Modifier::Control))?;
    assert!(key(&mut adapter, &mut router, 224, Pressed, false)?.is_empty());
    let started = key(&mut adapter, &mut router, 4, Pressed, false)?;
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);
    assert_eq!(started[0].action().as_str(), "audit.action");
    let cancelled = key(&mut adapter, &mut router, 224, Released, false)?;
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    assert!(key(&mut adapter, &mut router, 4, Pressed, true)?.is_empty());
    assert!(key(&mut adapter, &mut router, 4, Released, false)?.is_empty());
    assert!(router.held_inputs().is_empty());
    Ok(())
}

#[test]
fn last_shift_side_release_cancels_but_first_side_does_not() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router(Modifiers::one(Modifier::Shift))?;
    key(&mut adapter, &mut router, 225, Pressed, false)?;
    key(&mut adapter, &mut router, 229, Pressed, false)?;
    let started = key(&mut adapter, &mut router, 4, Pressed, false)?;
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);
    assert!(key(&mut adapter, &mut router, 225, Released, false)?.is_empty());
    let repeated = key(&mut adapter, &mut router, 4, Pressed, true)?;
    assert_eq!(repeated.len(), 1);
    assert_eq!(repeated[0].phase(), ActionPhase::Repeated);
    let cancelled = key(&mut adapter, &mut router, 229, Released, false)?;
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    assert!(key(&mut adapter, &mut router, 4, Pressed, true)?.is_empty());
    Ok(())
}

#[test]
fn modifier_change_does_not_rearm_held_letter() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    let mut router = router(Modifiers::one(Modifier::Control))?;
    key(&mut adapter, &mut router, 224, Pressed, false)?;
    let first = key(&mut adapter, &mut router, 4, Pressed, false)?;
    assert_eq!(first.len(), 1);
    key(&mut adapter, &mut router, 224, Released, false)?;
    assert!(key(&mut adapter, &mut router, 224, Pressed, false)?.is_empty());
    assert!(key(&mut adapter, &mut router, 4, Pressed, true)?.is_empty());
    key(&mut adapter, &mut router, 4, Released, false)?;
    let fresh = key(&mut adapter, &mut router, 4, Pressed, false)?;
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].phase(), ActionPhase::Started);
    Ok(())
}
