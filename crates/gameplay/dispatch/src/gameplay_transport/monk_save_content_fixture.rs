//! Default constructor retained for existing tests; the one fixture body accepts a real pin.
use super::*;
impl Dependencies {
    pub(super) fn new() -> TestResult<Self> {
        Self::new_with_pin(None)
    }
}
