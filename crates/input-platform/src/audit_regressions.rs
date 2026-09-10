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

#[test]
fn each_modifier_side_preserves_normal_release_and_non_modifier_held_set()
-> Result<(), Box<dyn Error>> {
    for (code, modifier) in [
        (224, Modifier::Control),
        (225, Modifier::Shift),
        (226, Modifier::Alt),
        (227, Modifier::Super),
        (228, Modifier::Control),
        (229, Modifier::Shift),
        (230, Modifier::Alt),
        (231, Modifier::Super),
    ] {
        let mut adapter = InputPlatformAdapter::new();
        let mut router = router(Modifiers::one(modifier))?;
        assert!(key(&mut adapter, &mut router, code, Pressed, false)?.is_empty());
        assert!(router.held_inputs().is_empty());
        let started = key(&mut adapter, &mut router, 4, Pressed, false)?;
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].phase(), ActionPhase::Started);
        assert_eq!(router.held_inputs().len(), 1);
        assert!(router.held_inputs().contains(&InputAtom::Key(KeyCode::KEY_A)));
        let ended = key(&mut adapter, &mut router, 4, Released, false)?;
        assert_eq!(ended.len(), 1);
        assert_eq!(ended[0].phase(), ActionPhase::Ended);
        assert!(key(&mut adapter, &mut router, code, Released, false)?.is_empty());
        assert!(router.held_inputs().is_empty());
        assert_eq!(router.modifiers(), Modifiers::NONE);
    }
    Ok(())
}

#[test]
fn binding_atoms_reject_all_modifier_positions() -> Result<(), InputError> {
    for code in 224..=231 {
        let modifier = InputAtom::Key(KeyCode::new(code)?);
        for atoms in [vec![modifier], vec![InputAtom::Key(KeyCode::KEY_A), modifier]] {
            assert_eq!(
                InputChord::new(Modifiers::NONE, atoms),
                Err(InputError::ModifierChordInput)
            );
        }
    }
    Ok(())
}

#[test]
fn modifier_does_not_consume_the_four_atom_chord_limit() -> Result<(), Box<dyn Error>> {
    let context = ContextId::new("audit.four".to_owned())?;
    let atoms = vec![
        InputAtom::Key(KeyCode::KEY_A),
        InputAtom::Key(KeyCode::KEY_B),
        InputAtom::Key(KeyCode::KEY_C),
        InputAtom::Key(KeyCode::KEY_D),
    ];
    let map = BindingMap::new(
        vec![ContextDefinition::new(
            context.clone(),
            ContextKind::Gameplay,
            0,
        )],
        vec![Binding::new(
            context.clone(),
            InputChord::new(Modifiers::one(Modifier::Control), atoms)?,
            ActionId::new("audit.four".to_owned())?,
            RepeatPolicy::Ignore,
        )],
        &[],
    )?;
    let mut router = InputRouter::new(map);
    router.set_context_active(&context, true)?;
    let mut adapter = InputPlatformAdapter::new();
    key(&mut adapter, &mut router, 224, Pressed, false)?;
    for code in 4..7 {
        assert!(key(&mut adapter, &mut router, code, Pressed, false)?.is_empty());
    }
    let started = key(&mut adapter, &mut router, 7, Pressed, false)?;
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].phase(), ActionPhase::Started);
    assert_eq!(router.held_inputs().len(), 4);
    let cancelled = key(&mut adapter, &mut router, 224, Released, false)?;
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].phase(), ActionPhase::Cancelled);
    for code in 4..8 {
        assert!(key(&mut adapter, &mut router, code, Released, false)?.is_empty());
    }
    assert!(router.held_inputs().is_empty());
    Ok(())
}
