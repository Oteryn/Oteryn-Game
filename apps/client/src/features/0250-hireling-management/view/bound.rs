#[cfg(test)]
mod tests {
    #[test]
    fn hireling_management_rejected_when_session_replaced_or_target_changed() {
        let (link, mut commands, _) = crate::play::play_channel();
        let snapshot = link.hireling_management_snapshot();
        assert!(!link.hireling_management_dispatch_bound(&snapshot, super::HirelingAction::Refresh));
        assert!(commands.try_recv().is_err());
    }
}
