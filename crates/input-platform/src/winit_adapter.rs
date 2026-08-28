use crate::adapter::{PlatformButtonState, PlatformEvent, PlatformWheelUnit};
use crate::{InputPlatformAdapter, InputPlatformError};
use oteryn_input_actions::NormalizedInputEvent;
use winit::event::{
    DeviceEvent, ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent,
};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};

impl InputPlatformAdapter {
    /// Normalize one supported winit window event.
    ///
    /// Physical identity is derived only from `KeyEvent::physical_key`; logical-key data never
    /// becomes a physical binding identity. Unsupported window-event families are ignored.
    /// Unsupported physical keys and mouse buttons normalize to a merged device reset, preventing
    /// a missing release from stranding held input without exposing a native identifier.
    ///
    /// # Errors
    ///
    /// Returns [`InputPlatformError`] when an accepted numeric, text or lifecycle value violates
    /// the merged physical-event bounds.
    pub fn process_window_event(
        &mut self,
        event: &WindowEvent,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.process_platform_event(PlatformEvent::Key {
                code: physical_key_code(event.physical_key),
                state: button_state(event.state),
                repeat: event.repeat,
                text: event.text.as_deref(),
                synthetic: *is_synthetic,
            }),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.process_platform_event(PlatformEvent::Modifiers {
                    bits: modifier_bits(modifiers.state()),
                })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.process_platform_event(PlatformEvent::MouseButton {
                    button: mouse_button(*button),
                    state: button_state(*state),
                })
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.process_platform_event(PlatformEvent::PointerPosition {
                    x: position.x,
                    y: position.y,
                })
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(horizontal, vertical) => {
                    self.process_platform_event(PlatformEvent::Wheel {
                        horizontal: f64::from(*horizontal),
                        vertical: f64::from(*vertical),
                        unit: PlatformWheelUnit::Lines,
                    })
                }
                MouseScrollDelta::PixelDelta(position) => {
                    self.process_platform_event(PlatformEvent::Wheel {
                        horizontal: position.x,
                        vertical: position.y,
                        unit: PlatformWheelUnit::Pixels,
                    })
                }
            },
            WindowEvent::Ime(event) => match event {
                Ime::Enabled => self.process_platform_event(PlatformEvent::ImeEnabled(true)),
                Ime::Disabled => self.process_platform_event(PlatformEvent::ImeEnabled(false)),
                Ime::Preedit(_, _) => self.process_platform_event(PlatformEvent::ImePreedit),
                Ime::Commit(text) => {
                    self.process_platform_event(PlatformEvent::TextCommit(text.as_str()))
                }
            },
            WindowEvent::Focused(focused) => {
                self.process_platform_event(PlatformEvent::Focus(*focused))
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Normalize supported winit device lifecycle and relative-motion events.
    ///
    /// Device identifiers are deliberately ignored and never retained. Background relative motion
    /// is ignored unless application-owned focus and capture are both active.
    ///
    /// # Errors
    ///
    /// Returns [`InputPlatformError`] when accepted relative motion is non-finite or outside the
    /// merged physical-event bounds.
    pub fn process_device_event(
        &mut self,
        event: &DeviceEvent,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.process_platform_event(PlatformEvent::PointerMotion {
                    x: delta.0,
                    y: delta.1,
                })
            }
            DeviceEvent::Removed => self.process_platform_event(PlatformEvent::DeviceLost),
            _ => Ok(Vec::new()),
        }
    }
}

const fn button_state(state: ElementState) -> PlatformButtonState {
    match state {
        ElementState::Pressed => PlatformButtonState::Pressed,
        ElementState::Released => PlatformButtonState::Released,
    }
}

fn modifier_bits(state: winit::keyboard::ModifiersState) -> u8 {
    let mut bits = 0;
    if state.shift_key() {
        bits |= 1 << 0;
    }
    if state.control_key() {
        bits |= 1 << 1;
    }
    if state.alt_key() {
        bits |= 1 << 2;
    }
    if state.super_key() {
        bits |= 1 << 3;
    }
    bits
}

const fn mouse_button(button: WinitMouseButton) -> Option<u8> {
    match button {
        WinitMouseButton::Left => Some(1),
        WinitMouseButton::Right => Some(2),
        WinitMouseButton::Middle => Some(3),
        WinitMouseButton::Back => Some(4),
        WinitMouseButton::Forward => Some(5),
        WinitMouseButton::Other(_) => None,
    }
}

const fn physical_key_code(key: PhysicalKey) -> Option<u16> {
    let PhysicalKey::Code(code) = key else {
        return None;
    };
    match code {
        WinitKeyCode::KeyA => Some(4),
        WinitKeyCode::KeyB => Some(5),
        WinitKeyCode::KeyC => Some(6),
        WinitKeyCode::KeyD => Some(7),
        WinitKeyCode::KeyE => Some(8),
        WinitKeyCode::KeyF => Some(9),
        WinitKeyCode::KeyG => Some(10),
        WinitKeyCode::KeyH => Some(11),
        WinitKeyCode::KeyI => Some(12),
        WinitKeyCode::KeyJ => Some(13),
        WinitKeyCode::KeyK => Some(14),
        WinitKeyCode::KeyL => Some(15),
        WinitKeyCode::KeyM => Some(16),
        WinitKeyCode::KeyN => Some(17),
        WinitKeyCode::KeyO => Some(18),
        WinitKeyCode::KeyP => Some(19),
        WinitKeyCode::KeyQ => Some(20),
        WinitKeyCode::KeyR => Some(21),
        WinitKeyCode::KeyS => Some(22),
        WinitKeyCode::KeyT => Some(23),
        WinitKeyCode::KeyU => Some(24),
        WinitKeyCode::KeyV => Some(25),
        WinitKeyCode::KeyW => Some(26),
        WinitKeyCode::KeyX => Some(27),
        WinitKeyCode::KeyY => Some(28),
        WinitKeyCode::KeyZ => Some(29),
        WinitKeyCode::Digit1 => Some(30),
        WinitKeyCode::Digit2 => Some(31),
        WinitKeyCode::Digit3 => Some(32),
        WinitKeyCode::Digit4 => Some(33),
        WinitKeyCode::Digit5 => Some(34),
        WinitKeyCode::Digit6 => Some(35),
        WinitKeyCode::Digit7 => Some(36),
        WinitKeyCode::Digit8 => Some(37),
        WinitKeyCode::Digit9 => Some(38),
        WinitKeyCode::Digit0 => Some(39),
        WinitKeyCode::Enter => Some(40),
        WinitKeyCode::Escape => Some(41),
        WinitKeyCode::Backspace => Some(42),
        WinitKeyCode::Tab => Some(43),
        WinitKeyCode::Space => Some(44),
        WinitKeyCode::Minus => Some(45),
        WinitKeyCode::Equal => Some(46),
        WinitKeyCode::BracketLeft => Some(47),
        WinitKeyCode::BracketRight => Some(48),
        WinitKeyCode::Backslash => Some(49),
        WinitKeyCode::Semicolon => Some(51),
        WinitKeyCode::Quote => Some(52),
        WinitKeyCode::Backquote => Some(53),
        WinitKeyCode::Comma => Some(54),
        WinitKeyCode::Period => Some(55),
        WinitKeyCode::Slash => Some(56),
        WinitKeyCode::CapsLock => Some(57),
        WinitKeyCode::F1 => Some(58),
        WinitKeyCode::F2 => Some(59),
        WinitKeyCode::F3 => Some(60),
        WinitKeyCode::F4 => Some(61),
        WinitKeyCode::F5 => Some(62),
        WinitKeyCode::F6 => Some(63),
        WinitKeyCode::F7 => Some(64),
        WinitKeyCode::F8 => Some(65),
        WinitKeyCode::F9 => Some(66),
        WinitKeyCode::F10 => Some(67),
        WinitKeyCode::F11 => Some(68),
        WinitKeyCode::F12 => Some(69),
        WinitKeyCode::PrintScreen => Some(70),
        WinitKeyCode::ScrollLock => Some(71),
        WinitKeyCode::Pause => Some(72),
        WinitKeyCode::Insert => Some(73),
        WinitKeyCode::Home => Some(74),
        WinitKeyCode::PageUp => Some(75),
        WinitKeyCode::Delete => Some(76),
        WinitKeyCode::End => Some(77),
        WinitKeyCode::PageDown => Some(78),
        WinitKeyCode::ArrowRight => Some(79),
        WinitKeyCode::ArrowLeft => Some(80),
        WinitKeyCode::ArrowDown => Some(81),
        WinitKeyCode::ArrowUp => Some(82),
        WinitKeyCode::NumLock => Some(83),
        WinitKeyCode::NumpadDivide => Some(84),
        WinitKeyCode::NumpadMultiply => Some(85),
        WinitKeyCode::NumpadSubtract => Some(86),
        WinitKeyCode::NumpadAdd => Some(87),
        WinitKeyCode::NumpadEnter => Some(88),
        WinitKeyCode::Numpad1 => Some(89),
        WinitKeyCode::Numpad2 => Some(90),
        WinitKeyCode::Numpad3 => Some(91),
        WinitKeyCode::Numpad4 => Some(92),
        WinitKeyCode::Numpad5 => Some(93),
        WinitKeyCode::Numpad6 => Some(94),
        WinitKeyCode::Numpad7 => Some(95),
        WinitKeyCode::Numpad8 => Some(96),
        WinitKeyCode::Numpad9 => Some(97),
        WinitKeyCode::Numpad0 => Some(98),
        WinitKeyCode::NumpadDecimal => Some(99),
        WinitKeyCode::IntlBackslash => Some(100),
        WinitKeyCode::ContextMenu => Some(101),
        WinitKeyCode::NumpadEqual => Some(103),
        WinitKeyCode::F13 => Some(104),
        WinitKeyCode::F14 => Some(105),
        WinitKeyCode::F15 => Some(106),
        WinitKeyCode::F16 => Some(107),
        WinitKeyCode::F17 => Some(108),
        WinitKeyCode::F18 => Some(109),
        WinitKeyCode::F19 => Some(110),
        WinitKeyCode::F20 => Some(111),
        WinitKeyCode::F21 => Some(112),
        WinitKeyCode::F22 => Some(113),
        WinitKeyCode::F23 => Some(114),
        WinitKeyCode::F24 => Some(115),
        WinitKeyCode::ControlLeft => Some(224),
        WinitKeyCode::ShiftLeft => Some(225),
        WinitKeyCode::AltLeft => Some(226),
        WinitKeyCode::SuperLeft => Some(227),
        WinitKeyCode::ControlRight => Some(228),
        WinitKeyCode::ShiftRight => Some(229),
        WinitKeyCode::AltRight => Some(230),
        WinitKeyCode::SuperRight => Some(231),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{mouse_button, physical_key_code};
    use winit::event::MouseButton;
    use winit::keyboard::{KeyCode, NativeKeyCode, PhysicalKey};

    #[test]
    fn stable_keyboard_subset_uses_usb_hid_usage_codes() {
        assert_eq!(physical_key_code(PhysicalKey::Code(KeyCode::KeyA)), Some(4));
        assert_eq!(
            physical_key_code(PhysicalKey::Code(KeyCode::KeyW)),
            Some(26)
        );
        assert_eq!(
            physical_key_code(PhysicalKey::Code(KeyCode::ArrowUp)),
            Some(82)
        );
        assert_eq!(
            physical_key_code(PhysicalKey::Code(KeyCode::ControlLeft)),
            Some(224)
        );
        assert_eq!(
            physical_key_code(PhysicalKey::Unidentified(NativeKeyCode::Unidentified)),
            None
        );
    }

    #[test]
    fn named_mouse_buttons_are_stable_and_other_is_unsupported() {
        assert_eq!(mouse_button(MouseButton::Left), Some(1));
        assert_eq!(mouse_button(MouseButton::Right), Some(2));
        assert_eq!(mouse_button(MouseButton::Middle), Some(3));
        assert_eq!(mouse_button(MouseButton::Back), Some(4));
        assert_eq!(mouse_button(MouseButton::Forward), Some(5));
        assert_eq!(mouse_button(MouseButton::Other(9)), None);
    }
}
