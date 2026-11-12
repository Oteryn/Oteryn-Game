//! The last confirmed step/use refusal, cleared by the next decoded result.
use super::*;
impl<S: SessionStream> Session<S> {
    pub const fn last_refusal_detail(&self) -> Option<item_movement::RefusalDetail> {
        self.last_refusal_detail
    }
}
