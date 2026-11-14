#![cfg(feature = "native-winit")]

use oteryn_input_actions::{ButtonState, Modifiers, MouseButton, NormalizedInputEvent};
use oteryn_input_platform::InputPlatformAdapter;
use winit::event::{
    DeviceEvent, DeviceId, ElementState, Ime, MouseButton as NativeButton, WindowEvent,
};

#[test]
fn public_native_ingress_preserves_button_focus_reset_and_ime_commit()
-> Result<(), Box<dyn std::error::Error>> {
    let mut adapter = InputPlatformAdapter::new();
    adapter.process_window_event(&WindowEvent::Focused(true))?;
    let press = WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state: ElementState::Pressed,
        button: NativeButton::Left,
    };
    assert_eq!(
        adapter.process_window_event(&press)?,
        vec![NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
        }]
    );
    let reset = adapter.process_window_event(&WindowEvent::Focused(false))?;
    assert!(reset.contains(&NormalizedInputEvent::FocusChanged { focused: false }));
    assert!(!adapter.focused());
    assert!(adapter.process_window_event(&press)?.is_empty());
    adapter.process_window_event(&WindowEvent::Focused(true))?;
    adapter.process_window_event(&WindowEvent::Ime(Ime::Enabled))?;
    assert!(
        adapter
            .process_window_event(&WindowEvent::Ime(Ime::Preedit(
                "composing".into(),
                Some((0, 9))
            )))?
            .is_empty()
    );
    let commit = adapter.process_window_event(&WindowEvent::Ime(Ime::Commit("ą".into())))?;
    assert_eq!(commit.len(), 1);
    assert!(matches!(&commit[0], NormalizedInputEvent::TextCommitted(_)));
    assert_eq!(
        adapter.process_device_event(&DeviceEvent::Removed)?,
        vec![NormalizedInputEvent::DeviceLost]
    );
    Ok(())
}
