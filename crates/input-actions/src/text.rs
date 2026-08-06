use crate::InputError;
use std::fmt::{self, Debug, Formatter};

/// Maximum UTF-8 byte length of one normalized text/IME commit.
pub const MAX_TEXT_BYTES: usize = 4_096;

/// Bounded normalized text committed by a platform adapter.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextCommit(String);

impl TextCommit {
    /// Validate one committed text payload.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::EmptyText`] or [`InputError::TextTooLong`].
    pub fn new(value: String) -> Result<Self, InputError> {
        if value.is_empty() {
            return Err(InputError::EmptyText);
        }
        if value.len() > MAX_TEXT_BYTES {
            return Err(InputError::TextTooLong {
                max: MAX_TEXT_BYTES,
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }

    /// Borrow the accepted UTF-8 text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return the accepted UTF-8 byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return whether the accepted text is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }
}

impl Debug for TextCommit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextCommit")
            .field("bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}
