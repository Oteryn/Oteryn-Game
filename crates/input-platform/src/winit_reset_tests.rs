//! Session generation boundaries consume OS focus without replaying queued actions.
use super::*;
use crate::adapter::{PlatformButtonState, PlatformEvent};
use oteryn_input_actions::Modifiers;
use std::error::Error;
use winit::{dpi::PhysicalPosition, event::DeviceId, keyboard::ModifiersState};

fn pointer(x: f64, y: f64) -> WindowEvent {
    WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(x, y),
    }
}

fn transient_events() -> [WindowEvent; 4] {
    [
        WindowEvent::ModifiersChanged(ModifiersState::CONTROL.into()),
        pointer(40., 50.),
        WindowEvent::Ime(Ime::Enabled),
        WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: ElementState::Pressed,
            button: WinitMouseButton::Left,
        },
    ]
}

fn assert_transients_cleared(adapter: &mut InputPlatformAdapter) -> Result<(), Box<dyn Error>> {
    assert!(!adapter.captured());
    assert_eq!(adapter.modifiers()?, Modifiers::NONE);
    let moved = adapter.process_window_event(&pointer(80., 90.))?;
    assert!(
        matches!(moved.as_slice(), [NormalizedInputEvent::PointerMoved { motion, .. }]
        if motion.x().get() == 0 && motion.y().get() == 0)
    );
    // Winit keeps KeyEvent's extra fields private. Observe IME clearing through the same
    // internal key normalizer used by process_window_event rather than constructing it unsafely.
    let key = adapter.process_platform_event(PlatformEvent::Key {
        code: Some(4),
        state: PlatformButtonState::Pressed,
        repeat: false,
        text: Some("a"),
        synthetic: false,
    })?;
    assert!(
        matches!(key.as_slice(), [NormalizedInputEvent::Key { modifiers, .. },
        NormalizedInputEvent::TextCommitted(_)] if *modifiers == Modifiers::NONE)
    );
    Ok(())
}

#[test]
fn reset_after_window_events_retains_focus_gain_without_replaying_actions()
-> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.process_window_event(&WindowEvent::Focused(false))?;
    let events = [WindowEvent::Focused(true)]
        .into_iter()
        .chain(transient_events())
        .collect::<Vec<_>>();
    assert_eq!(
        adapter.reset_after_window_events(&events),
        vec![NormalizedInputEvent::DeviceLost]
    );
    assert!(
        adapter.focused(),
        "generation reset lost the queued OS focus gain"
    );
    assert_transients_cleared(&mut adapter)
}

#[test]
fn reset_after_window_events_retains_focus_loss() -> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.set_capture_state(true)?;
    let events = transient_events()
        .into_iter()
        .chain([WindowEvent::Focused(false)])
        .collect::<Vec<_>>();
    assert_eq!(
        adapter.reset_after_window_events(&events),
        vec![NormalizedInputEvent::DeviceLost]
    );
    assert!(!adapter.focused());
    assert!(!adapter.captured());
    assert_eq!(adapter.modifiers()?, Modifiers::NONE);
    assert!(adapter.process_window_event(&pointer(80., 90.))?.is_empty());
    Ok(())
}

#[test]
fn reset_after_window_events_clears_transients_when_focus_is_unchanged()
-> Result<(), Box<dyn Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.set_capture_state(true)?;
    assert_eq!(
        adapter.reset_after_window_events(&transient_events()),
        vec![NormalizedInputEvent::DeviceLost]
    );
    assert!(adapter.focused());
    assert_transients_cleared(&mut adapter)
}
