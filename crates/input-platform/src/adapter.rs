use crate::InputPlatformError;
use oteryn_input_actions::{
    ButtonState, InputError, KeyCode, MAX_POINTER_COORDINATE, MAX_POINTER_DELTA, MAX_TEXT_BYTES,
    MAX_WHEEL_DELTA, Modifiers, MouseButton, NormalizedInputEvent, PointerCoordinate, PointerDelta,
    PointerMotion, PointerPosition, TextCommit, WheelDelta,
};

const WHEEL_LINE_UNITS: f64 = 120.0;
const MODIFIER_SHIFT_BIT: u8 = 1 << 0;
const MODIFIER_CONTROL_BIT: u8 = 1 << 1;
const MODIFIER_ALT_BIT: u8 = 1 << 2;
const MODIFIER_SUPER_BIT: u8 = 1 << 3;

const SIDE_CONTROL_LEFT: u8 = 1 << 0;
const SIDE_CONTROL_RIGHT: u8 = 1 << 1;
const SIDE_SHIFT_LEFT: u8 = 1 << 2;
const SIDE_SHIFT_RIGHT: u8 = 1 << 3;
const SIDE_ALT_LEFT: u8 = 1 << 4;
const SIDE_ALT_RIGHT: u8 = 1 << 5;
const SIDE_SUPER_LEFT: u8 = 1 << 6;
const SIDE_SUPER_RIGHT: u8 = 1 << 7;

const GROUP_CONTROL: u8 = 1 << 0;
const GROUP_SHIFT: u8 = 1 << 1;
const GROUP_ALT: u8 = 1 << 2;
const GROUP_SUPER: u8 = 1 << 3;

// Constructed by the Windows-only winit adapter; intentionally dormant in Linux builds.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformButtonState {
    Pressed,
    Released,
}

impl PlatformButtonState {
    const fn normalized(self) -> ButtonState {
        match self {
            Self::Pressed => ButtonState::Pressed,
            Self::Released => ButtonState::Released,
        }
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlatformWheelUnit {
    Lines,
    Pixels,
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum PlatformEvent<'a> {
    Key {
        code: Option<u16>,
        state: PlatformButtonState,
        repeat: bool,
        text: Option<&'a str>,
        synthetic: bool,
    },
    Modifiers {
        bits: u8,
    },
    MouseButton {
        button: Option<u8>,
        state: PlatformButtonState,
    },
    PointerPosition {
        x: f64,
        y: f64,
    },
    PointerMotion {
        x: f64,
        y: f64,
    },
    Wheel {
        horizontal: f64,
        vertical: f64,
        unit: PlatformWheelUnit,
    },
    ImeEnabled(bool),
    ImePreedit,
    TextCommit(&'a str),
    Focus(bool),
    Capture(bool),
    DeviceLost,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ModifierTracker {
    reported_bits: u8,
    observed_groups: u8,
    pressed_sides: u8,
}

impl ModifierTracker {
    fn report(&mut self, bits: u8) -> Result<(), InputPlatformError> {
        let modifiers = Modifiers::from_bits(bits).map_err(InputPlatformError::from)?;
        self.reported_bits = modifiers.bits();
        Ok(())
    }

    fn update_key(&mut self, code: u16, state: PlatformButtonState) {
        let Some((side, group)) = modifier_side(code) else {
            return;
        };
        self.observed_groups |= group;
        match state {
            PlatformButtonState::Pressed => self.pressed_sides |= side,
            PlatformButtonState::Released => self.pressed_sides &= !side,
        }
    }

    fn effective(self) -> Result<Modifiers, InputPlatformError> {
        let mut bits = self.reported_bits;
        bits = override_group(
            bits,
            self.observed_groups,
            GROUP_SHIFT,
            MODIFIER_SHIFT_BIT,
            self.pressed_sides & (SIDE_SHIFT_LEFT | SIDE_SHIFT_RIGHT) != 0,
        );
        bits = override_group(
            bits,
            self.observed_groups,
            GROUP_CONTROL,
            MODIFIER_CONTROL_BIT,
            self.pressed_sides & (SIDE_CONTROL_LEFT | SIDE_CONTROL_RIGHT) != 0,
        );
        bits = override_group(
            bits,
            self.observed_groups,
            GROUP_ALT,
            MODIFIER_ALT_BIT,
            self.pressed_sides & (SIDE_ALT_LEFT | SIDE_ALT_RIGHT) != 0,
        );
        bits = override_group(
            bits,
            self.observed_groups,
            GROUP_SUPER,
            MODIFIER_SUPER_BIT,
            self.pressed_sides & (SIDE_SUPER_LEFT | SIDE_SUPER_RIGHT) != 0,
        );
        Modifiers::from_bits(bits).map_err(InputPlatformError::from)
    }

    const fn clear(&mut self) {
        self.reported_bits = 0;
        self.observed_groups = 0;
        self.pressed_sides = 0;
    }
}

const fn override_group(
    bits: u8,
    observed_groups: u8,
    group: u8,
    modifier_bit: u8,
    pressed: bool,
) -> u8 {
    if observed_groups & group == 0 {
        bits
    } else if pressed {
        bits | modifier_bit
    } else {
        bits & !modifier_bit
    }
}

const fn modifier_side(code: u16) -> Option<(u8, u8)> {
    match code {
        224 => Some((SIDE_CONTROL_LEFT, GROUP_CONTROL)),
        225 => Some((SIDE_SHIFT_LEFT, GROUP_SHIFT)),
        226 => Some((SIDE_ALT_LEFT, GROUP_ALT)),
        227 => Some((SIDE_SUPER_LEFT, GROUP_SUPER)),
        228 => Some((SIDE_CONTROL_RIGHT, GROUP_CONTROL)),
        229 => Some((SIDE_SHIFT_RIGHT, GROUP_SHIFT)),
        230 => Some((SIDE_ALT_RIGHT, GROUP_ALT)),
        231 => Some((SIDE_SUPER_RIGHT, GROUP_SUPER)),
        _ => None,
    }
}

/// Stateful adapter from platform events into merged framework-neutral physical events.
///
/// Events are processed strictly in event-loop receipt order. Duplicate focus and capture
/// transitions emit no output. Loss transitions are cleanup-idempotent. Unsupported key or button
/// values normalize to a merged device-reset event so they cannot strand held state. Relative
/// device motion is accepted only while focused and captured and after one validated absolute
/// cursor position establishes a non-identifying baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPlatformAdapter {
    modifiers: ModifierTracker,
    pointer_position: Option<PointerPosition>,
    focused: bool,
    captured: bool,
    ime_enabled: bool,
}

impl InputPlatformAdapter {
    /// Construct an adapter matching the merged router's initial focused state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            modifiers: ModifierTracker {
                reported_bits: 0,
                observed_groups: 0,
                pressed_sides: 0,
            },
            pointer_position: None,
            focused: true,
            captured: false,
            ime_enabled: false,
        }
    }

    /// Record the result of application-owned cursor capture negotiation.
    ///
    /// Call this only after the window API has confirmed the requested capture state.
    ///
    /// # Errors
    ///
    /// Returns [`InputPlatformError::CaptureWhileUnfocused`] when capture gain is reported while
    /// the application is unfocused.
    pub fn set_capture_state(
        &mut self,
        captured: bool,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        self.process_platform_event(PlatformEvent::Capture(captured))
    }

    /// Emit deterministic device-loss cleanup without retaining a device identifier.
    #[must_use]
    pub fn notify_device_lost(&mut self) -> Vec<NormalizedInputEvent> {
        self.process_device_loss()
    }

    /// Return whether focused input is currently accepted.
    #[must_use]
    pub const fn focused(&self) -> bool {
        self.focused
    }

    /// Return whether application-owned pointer capture is active.
    #[must_use]
    pub const fn captured(&self) -> bool {
        self.captured
    }

    /// Return the latest bounded effective modifier snapshot.
    ///
    /// # Errors
    ///
    /// Returns a merged contract error only if internal modifier normalization is invalid.
    pub fn modifiers(&self) -> Result<Modifiers, InputPlatformError> {
        self.modifiers.effective()
    }

    pub(crate) fn process_platform_event(
        &mut self,
        event: PlatformEvent<'_>,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        match event {
            PlatformEvent::Key {
                code,
                state,
                repeat,
                text,
                synthetic,
            } => self.process_key(code, state, repeat, text, synthetic),
            PlatformEvent::Modifiers { bits } => {
                if self.focused {
                    self.modifiers.report(bits)?;
                }
                Ok(Vec::new())
            }
            PlatformEvent::MouseButton { button, state } => {
                self.process_mouse_button(button, state)
            }
            PlatformEvent::PointerPosition { x, y } => self.process_pointer_position(x, y),
            PlatformEvent::PointerMotion { x, y } => self.process_pointer_motion(x, y),
            PlatformEvent::Wheel {
                horizontal,
                vertical,
                unit,
            } => self.process_wheel(horizontal, vertical, unit),
            PlatformEvent::ImeEnabled(enabled) => {
                if self.focused {
                    self.ime_enabled = enabled;
                }
                Ok(Vec::new())
            }
            PlatformEvent::ImePreedit => {
                if self.focused {
                    self.ime_enabled = true;
                }
                Ok(Vec::new())
            }
            PlatformEvent::TextCommit(text) => self.process_text(text),
            PlatformEvent::Focus(focused) => self.process_focus(focused),
            PlatformEvent::Capture(captured) => self.process_capture(captured),
            PlatformEvent::DeviceLost => Ok(self.process_device_loss()),
        }
    }

    fn process_key(
        &mut self,
        code: Option<u16>,
        state: PlatformButtonState,
        repeat: bool,
        text: Option<&str>,
        synthetic: bool,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused || synthetic {
            return Ok(Vec::new());
        }
        let Some(code) = code else {
            return Ok(self.process_device_loss());
        };
        let Ok(key) = KeyCode::new(code) else {
            return Ok(self.process_device_loss());
        };
        let text_commit = if state == PlatformButtonState::Pressed && !self.ime_enabled {
            text.map(normalize_text).transpose()?.flatten()
        } else {
            None
        };

        self.modifiers.update_key(code, state);
        let modifiers = self.modifiers.effective()?;
        let mut events = vec![NormalizedInputEvent::Key {
            code: key,
            state: state.normalized(),
            modifiers,
            repeat,
        }];
        if let Some(commit) = text_commit {
            events.push(NormalizedInputEvent::TextCommitted(commit));
        }
        Ok(events)
    }

    fn process_mouse_button(
        &mut self,
        button: Option<u8>,
        state: PlatformButtonState,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused {
            return Ok(Vec::new());
        }
        let Some(button) = button else {
            return Ok(self.process_device_loss());
        };
        let Ok(button) = MouseButton::new(button) else {
            return Ok(self.process_device_loss());
        };
        Ok(vec![NormalizedInputEvent::MouseButton {
            button,
            state: state.normalized(),
            modifiers: self.modifiers.effective()?,
        }])
    }

    fn process_pointer_position(
        &mut self,
        x: f64,
        y: f64,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused {
            return Ok(Vec::new());
        }
        let position = PointerPosition::new(normalize_coordinate(x)?, normalize_coordinate(y)?);
        let motion = match self.pointer_position {
            Some(previous) => bounded_or_rebased_motion(previous, position)?,
            None => zero_motion()?,
        };
        self.pointer_position = Some(position);
        Ok(vec![NormalizedInputEvent::PointerMoved {
            position,
            motion,
        }])
    }

    fn process_pointer_motion(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused || !self.captured {
            return Ok(Vec::new());
        }
        let position = self
            .pointer_position
            .ok_or(InputPlatformError::RelativeMotionUnavailable)?;
        let motion = PointerMotion::new(normalize_delta(x)?, normalize_delta(y)?);
        Ok(vec![NormalizedInputEvent::PointerMoved {
            position,
            motion,
        }])
    }

    fn process_wheel(
        &self,
        horizontal: f64,
        vertical: f64,
        unit: PlatformWheelUnit,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused {
            return Ok(Vec::new());
        }
        let scale = match unit {
            PlatformWheelUnit::Lines => WHEEL_LINE_UNITS,
            PlatformWheelUnit::Pixels => 1.0,
        };
        let horizontal = normalize_wheel_axis(horizontal, scale)?;
        let vertical = normalize_wheel_axis(vertical, scale)?;
        if horizontal == 0 && vertical == 0 {
            return Ok(Vec::new());
        }
        Ok(vec![NormalizedInputEvent::Wheel {
            delta: WheelDelta::new(horizontal, vertical).map_err(InputPlatformError::from)?,
            modifiers: self.modifiers.effective()?,
        }])
    }

    fn process_text(&self, text: &str) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if !self.focused {
            return Ok(Vec::new());
        }
        match normalize_text(text)? {
            Some(commit) => Ok(vec![NormalizedInputEvent::TextCommitted(commit)]),
            None => Ok(Vec::new()),
        }
    }

    fn process_focus(
        &mut self,
        focused: bool,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if self.focused == focused {
            return Ok(Vec::new());
        }
        if focused {
            self.focused = true;
            self.modifiers.clear();
            self.pointer_position = None;
            self.ime_enabled = false;
            return Ok(vec![NormalizedInputEvent::FocusChanged { focused: true }]);
        }

        let mut events = Vec::with_capacity(2);
        if self.captured {
            events.push(NormalizedInputEvent::CaptureChanged { captured: false });
        }
        events.push(NormalizedInputEvent::FocusChanged { focused: false });
        self.focused = false;
        self.captured = false;
        self.modifiers.clear();
        self.pointer_position = None;
        self.ime_enabled = false;
        Ok(events)
    }

    fn process_capture(
        &mut self,
        captured: bool,
    ) -> Result<Vec<NormalizedInputEvent>, InputPlatformError> {
        if captured && !self.focused {
            return Err(InputPlatformError::CaptureWhileUnfocused);
        }
        if self.captured == captured {
            return Ok(Vec::new());
        }
        self.captured = captured;
        if !captured {
            self.pointer_position = None;
        }
        Ok(vec![NormalizedInputEvent::CaptureChanged { captured }])
    }

    fn process_device_loss(&mut self) -> Vec<NormalizedInputEvent> {
        self.captured = false;
        self.modifiers.clear();
        self.pointer_position = None;
        self.ime_enabled = false;
        vec![NormalizedInputEvent::DeviceLost]
    }
}

impl Default for InputPlatformAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_text(text: &str) -> Result<Option<TextCommit>, InputPlatformError> {
    if text.is_empty() {
        return Ok(None);
    }
    if text.len() > MAX_TEXT_BYTES {
        return Err(InputError::TextTooLong {
            max: MAX_TEXT_BYTES,
            actual: text.len(),
        }
        .into());
    }
    TextCommit::new(text.to_owned())
        .map(Some)
        .map_err(InputPlatformError::from)
}

fn normalize_coordinate(value: f64) -> Result<PointerCoordinate, InputPlatformError> {
    let value = normalize_axis(
        value,
        1.0,
        MAX_POINTER_COORDINATE,
        InputError::CoordinateOutOfRange,
    )?;
    PointerCoordinate::new(value).map_err(InputPlatformError::from)
}

fn normalize_delta(value: f64) -> Result<PointerDelta, InputPlatformError> {
    let value = normalize_axis(value, 1.0, MAX_POINTER_DELTA, InputError::DeltaOutOfRange)?;
    PointerDelta::new(value).map_err(InputPlatformError::from)
}

fn normalize_wheel_axis(value: f64, scale: f64) -> Result<i32, InputPlatformError> {
    normalize_axis(
        value,
        scale,
        MAX_WHEEL_DELTA,
        InputError::WheelDeltaOutOfRange,
    )
}

fn normalize_axis(
    value: f64,
    scale: f64,
    absolute_bound: i32,
    range_error: InputError,
) -> Result<i32, InputPlatformError> {
    let scaled = value * scale;
    if !scaled.is_finite() {
        return Err(InputPlatformError::NonFiniteValue);
    }
    let rounded = scaled.round();
    let bound = f64::from(absolute_bound);
    if rounded < -bound || rounded > bound {
        return Err(range_error.into());
    }
    Ok(rounded as i32)
}

fn bounded_or_rebased_motion(
    previous: PointerPosition,
    current: PointerPosition,
) -> Result<PointerMotion, InputPlatformError> {
    let x = i64::from(current.x().get()) - i64::from(previous.x().get());
    let y = i64::from(current.y().get()) - i64::from(previous.y().get());
    if x.unsigned_abs() > MAX_POINTER_DELTA as u64 || y.unsigned_abs() > MAX_POINTER_DELTA as u64 {
        return zero_motion();
    }
    Ok(PointerMotion::new(
        PointerDelta::new(x as i32).map_err(InputPlatformError::from)?,
        PointerDelta::new(y as i32).map_err(InputPlatformError::from)?,
    ))
}

fn zero_motion() -> Result<PointerMotion, InputPlatformError> {
    Ok(PointerMotion::new(
        PointerDelta::new(0).map_err(InputPlatformError::from)?,
        PointerDelta::new(0).map_err(InputPlatformError::from)?,
    ))
}
