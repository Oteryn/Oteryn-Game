//! Capability 66 presentation. Text comes from the selected server detail, never a local
//! guess about distance/capacity. Canary tools.cpp getReturnMessage is pinned by the codec.
use super::{Outbox, mailbox::lock};
#[cfg(test)]
use oteryn_session::refusal_detail::RefusalDetail;
impl Outbox {
    pub(super) fn action_feedback(&self, line: Option<&'static str>) {
        lock(&self.0).action_feedback = Some(line);
    }
}
#[cfg(test)]
#[path = "refusal_details/tests.rs"]
mod driver_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_server_detail_reaches_the_native_status_without_rewording()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _, outbox) = super::super::play_channel();
        let mut view = super::super::tests::view()?;
        for detail in RefusalDetail::ALL {
            outbox.action_feedback(Some(detail.reference_message()));
            // Applying the mailbox event is the same path used by the shell tick.
            view.tick(&link).map_err(|_| "native tick failed")?;
            assert!(view.combat_status().contains(detail.reference_message()));
        }
        outbox.action_feedback(None);
        view.tick(&link).map_err(|_| "native tick failed")?;
        assert!(!view.combat_status().contains("locked"));
        Ok(())
    }
}
