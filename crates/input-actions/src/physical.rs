use crate::{InputError, TextCommit};

/// Largest accepted stable physical key code.
pub const MAX_KEY_CODE: u16 = 4_095;
/// Largest accepted normalized mouse button number.
pub const MAX_MOUSE_BUTTON: u8 = 16;
/// Largest absolute accepted pointer coordinate.
pub const MAX_POINTER_COORDINATE: i32 = 1_000_000;
/// Largest absolute accepted pointer delta.
pub const MAX_POINTER_DELTA: i32 = 100_000;
/// Largest absolute accepted wheel delta on one axis.
pub const MAX_WHEEL_DELTA: i32 = 120_000;

/// Stable physical keyboard position code, independent of localized labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyCode(u16);

impl KeyCode {
    /// Physical `A` position on a standard keyboard.
    pub const KEY_A: Self = Self(4);
    /// Physical `B` position on a standard keyboard.
    pub const KEY_B: Self = Self(5);
    /// Physical `C` position on a standard keyboard.
    pub const KEY_C: Self = Self(6);
    /// Physical `D` position on a standard keyboard.
    pub const KEY_D: Self = Self(7);
    /// Physical `E` position on a standard keyboard.
    pub const KEY_E: Self = Self(8);
    /// Physical `F` position on a standard keyboard.
    pub const KEY_F: Self = Self(9);
    /// Physical `W` position on a standard keyboard.
    pub const KEY_W: Self = Self(26);
    /// Physical `S` position on a standard keyboard.
    pub const KEY_S: Self = Self(22);
    /// Physical escape key.
    pub const ESCAPE: Self = Self(41);
    /// Physical enter key.
    pub const ENTER: Self = Self(40);
    /// Physical space key.
    pub const SPACE: Self = Self(44);
    /// Physical tab key.
    pub const TAB: Self = Self(43);
    /// Physical up-arrow key.
    pub const ARROW_UP: Self = Self(82);
    /// Physical down-arrow key.
    pub const ARROW_DOWN: Self = Self(81);
    /// Physical left-arrow key.
    pub const ARROW_LEFT: Self = Self(80);
    /// Physical right-arrow key.
    pub const ARROW_RIGHT: Self = Self(79);

    /// Validate a platform-neutral physical key code.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::ZeroKeyCode`] or [`InputError::KeyCodeOutOfRange`].
    pub fn new(value: u16) -> Result<Self, InputError> {
        if value == 0 {
            return Err(InputError::ZeroKeyCode);
        }
        if value > MAX_KEY_CODE {
            return Err(InputError::KeyCodeOutOfRange);
        }
        Ok(Self(value))
    }

    /// Return the stable numeric physical code.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Stable normalized mouse button number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MouseButton(u8);

impl MouseButton {
    /// Primary pointer button.
    pub const PRIMARY: Self = Self(1);
    /// Secondary pointer button.
    pub const SECONDARY: Self = Self(2);
    /// Middle pointer button.
    pub const MIDDLE: Self = Self(3);

    /// Validate a normalized mouse button number.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::ZeroMouseButton`] or
    /// [`InputError::MouseButtonOutOfRange`].
    pub fn new(value: u8) -> Result<Self, InputError> {
        if value == 0 {
            return Err(InputError::ZeroMouseButton);
        }
        if value > MAX_MOUSE_BUTTON {
            return Err(InputError::MouseButtonOutOfRange);
        }
        Ok(Self(value))
    }

    /// Return the normalized numeric button.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Normalized modifier key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    /// Shift modifier.
    Shift,
    /// Control modifier.
    Control,
    /// Alt/option modifier.
    Alt,
    /// Super/command/Windows modifier.
    Super,
}

impl Modifier {
    const fn bit(self) -> u8 {
        match self {
            Self::Shift => 1 << 0,
            Self::Control => 1 << 1,
            Self::Alt => 1 << 2,
            Self::Super => 1 << 3,
        }
    }
}

/// Canonically ordered normalized modifier set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Modifiers(u8);

impl Modifiers {
    const KNOWN_BITS: u8 = 0b1111;

    /// Empty modifier set.
    pub const NONE: Self = Self(0);

    /// Validate modifier bits supplied by an adapter.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::InvalidModifierBits`] for unknown flags.
    pub fn from_bits(bits: u8) -> Result<Self, InputError> {
        if bits & !Self::KNOWN_BITS != 0 {
            return Err(InputError::InvalidModifierBits);
        }
        Ok(Self(bits))
    }

    /// Return a set containing one modifier.
    #[must_use]
    pub const fn one(modifier: Modifier) -> Self {
        Self(modifier.bit())
    }

    /// Return a set with one modifier enabled.
    #[must_use]
    pub const fn with(self, modifier: Modifier) -> Self {
        Self(self.0 | modifier.bit())
    }

    /// Return whether one modifier is enabled.
    #[must_use]
    pub const fn contains(self, modifier: Modifier) -> bool {
        self.0 & modifier.bit() != 0
    }

    /// Return the stable bit representation.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Enumerate modifiers in stable Shift, Control, Alt, Super order.
    pub fn iter(self) -> impl Iterator<Item = Modifier> {
        [
            Modifier::Shift,
            Modifier::Control,
            Modifier::Alt,
            Modifier::Super,
        ]
        .into_iter()
        .filter(move |modifier| self.contains(*modifier))
    }
}

/// Discrete wheel direction suitable for semantic bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WheelDirection {
    /// Positive vertical direction.
    Up,
    /// Negative vertical direction.
    Down,
    /// Negative horizontal direction.
    Left,
    /// Positive horizontal direction.
    Right,
}

/// One bindable non-modifier physical input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InputAtom {
    /// Physical keyboard position.
    Key(KeyCode),
    /// Normalized mouse button.
    Mouse(MouseButton),
    /// Ephemeral wheel direction.
    Wheel(WheelDirection),
}

impl InputAtom {
    /// Return whether this input is a pointer button.
    #[must_use]
    pub const fn is_mouse_button(self) -> bool {
        matches!(self, Self::Mouse(_))
    }

    /// Return whether this input is an ephemeral wheel direction.
    #[must_use]
    pub const fn is_wheel(self) -> bool {
        matches!(self, Self::Wheel(_))
    }
}

/// Normalized press/release state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// Button transitioned to held.
    Pressed,
    /// Button transitioned to released.
    Released,
}

/// Bounded absolute pointer coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointerCoordinate(i32);

impl PointerCoordinate {
    /// Validate one coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::CoordinateOutOfRange`] outside the contract range.
    pub fn new(value: i32) -> Result<Self, InputError> {
        if value.unsigned_abs() > MAX_POINTER_COORDINATE.unsigned_abs() {
            return Err(InputError::CoordinateOutOfRange);
        }
        Ok(Self(value))
    }

    /// Return the coordinate value.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }
}

/// Bounded relative pointer delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PointerDelta(i32);

impl PointerDelta {
    /// Validate one delta.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::DeltaOutOfRange`] outside the contract range.
    pub fn new(value: i32) -> Result<Self, InputError> {
        if value.unsigned_abs() > MAX_POINTER_DELTA.unsigned_abs() {
            return Err(InputError::DeltaOutOfRange);
        }
        Ok(Self(value))
    }

    /// Return the delta value.
    #[must_use]
    pub const fn get(self) -> i32 {
        self.0
    }
}

/// Bounded absolute pointer position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerPosition {
    x: PointerCoordinate,
    y: PointerCoordinate,
}

impl PointerPosition {
    /// Construct a validated position.
    #[must_use]
    pub const fn new(x: PointerCoordinate, y: PointerCoordinate) -> Self {
        Self { x, y }
    }

    /// Return the horizontal coordinate.
    #[must_use]
    pub const fn x(self) -> PointerCoordinate {
        self.x
    }

    /// Return the vertical coordinate.
    #[must_use]
    pub const fn y(self) -> PointerCoordinate {
        self.y
    }
}

/// Bounded relative pointer motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerMotion {
    x: PointerDelta,
    y: PointerDelta,
}

impl PointerMotion {
    /// Construct validated relative motion.
    #[must_use]
    pub const fn new(x: PointerDelta, y: PointerDelta) -> Self {
        Self { x, y }
    }

    /// Return horizontal motion.
    #[must_use]
    pub const fn x(self) -> PointerDelta {
        self.x
    }

    /// Return vertical motion.
    #[must_use]
    pub const fn y(self) -> PointerDelta {
        self.y
    }
}

/// Bounded two-axis wheel delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WheelDelta {
    horizontal: i32,
    vertical: i32,
}

impl WheelDelta {
    /// Validate a wheel delta.
    ///
    /// # Errors
    ///
    /// Returns a stable zero or range error.
    pub fn new(horizontal: i32, vertical: i32) -> Result<Self, InputError> {
        if horizontal == 0 && vertical == 0 {
            return Err(InputError::ZeroWheelDelta);
        }
        if horizontal.unsigned_abs() > MAX_WHEEL_DELTA.unsigned_abs()
            || vertical.unsigned_abs() > MAX_WHEEL_DELTA.unsigned_abs()
        {
            return Err(InputError::WheelDeltaOutOfRange);
        }
        Ok(Self {
            horizontal,
            vertical,
        })
    }

    /// Return horizontal delta.
    #[must_use]
    pub const fn horizontal(self) -> i32 {
        self.horizontal
    }

    /// Return vertical delta.
    #[must_use]
    pub const fn vertical(self) -> i32 {
        self.vertical
    }

    /// Enumerate non-zero directions in stable vertical-then-horizontal order.
    pub fn directions(self) -> impl Iterator<Item = WheelDirection> {
        let vertical = if self.vertical > 0 {
            Some(WheelDirection::Up)
        } else if self.vertical < 0 {
            Some(WheelDirection::Down)
        } else {
            None
        };
        let horizontal = if self.horizontal < 0 {
            Some(WheelDirection::Left)
        } else if self.horizontal > 0 {
            Some(WheelDirection::Right)
        } else {
            None
        };
        vertical.into_iter().chain(horizontal)
    }
}

/// Complete framework-neutral normalized input event vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedInputEvent {
    /// Physical keyboard state transition.
    Key {
        /// Stable physical code.
        code: KeyCode,
        /// Press/release state.
        state: ButtonState,
        /// Canonical modifier snapshot after the transition.
        modifiers: Modifiers,
        /// Whether this press is a platform repeat notification.
        repeat: bool,
    },
    /// Mouse button state transition.
    MouseButton {
        /// Stable normalized button.
        button: MouseButton,
        /// Press/release state.
        state: ButtonState,
        /// Canonical modifier snapshot.
        modifiers: Modifiers,
    },
    /// Absolute and relative pointer movement.
    PointerMoved {
        /// Current bounded position.
        position: PointerPosition,
        /// Bounded relative motion.
        motion: PointerMotion,
    },
    /// Bounded wheel movement.
    Wheel {
        /// Two-axis delta.
        delta: WheelDelta,
        /// Canonical modifier snapshot.
        modifiers: Modifiers,
    },
    /// Committed text or IME result.
    TextCommitted(TextCommit),
    /// Application focus changed.
    FocusChanged {
        /// Whether the application accepts input.
        focused: bool,
    },
    /// Pointer capture changed.
    CaptureChanged {
        /// Whether pointer capture is active.
        captured: bool,
    },
    /// Adapter-reported device loss or reset.
    DeviceLost,
}
