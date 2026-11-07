use oteryn_input_actions::InputError;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Privacy-safe adapter failures.
///
/// Raw native key codes, device identifiers, window handles and text contents are intentionally
/// absent from every variant and formatted message. Unsupported keys and buttons are not errors:
/// they normalize to a merged `DeviceLost` reset so they cannot strand held input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPlatformError {
    /// A floating-point platform value was not finite.
    NonFiniteValue,
    /// Relative pointer motion arrived without focus, capture or an absolute baseline.
    RelativeMotionUnavailable,
    /// Pointer capture cannot become active while the application is unfocused.
    CaptureWhileUnfocused,
    /// A merged physical-event value rejected the normalized adapter value.
    Contract(InputError),
}

impl Display for InputPlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => formatter.write_str("platform input value is not finite"),
            Self::RelativeMotionUnavailable => formatter.write_str(
                "relative pointer motion requires focus, capture and an absolute pointer baseline",
            ),
            Self::CaptureWhileUnfocused => {
                formatter.write_str("pointer capture cannot activate while input is unfocused")
            }
            Self::Contract(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for InputPlatformError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Contract(error) => Some(error),
            Self::NonFiniteValue
            | Self::RelativeMotionUnavailable
            | Self::CaptureWhileUnfocused => None,
        }
    }
}

impl From<InputError> for InputPlatformError {
    fn from(error: InputError) -> Self {
        Self::Contract(error)
    }
}
