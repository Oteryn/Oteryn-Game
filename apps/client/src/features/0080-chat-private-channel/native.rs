//! Win32 FFI for private-channel controls, used only on the chat window's creating thread.
#![allow(unsafe_code)]
use oteryn_client_ui::chat_private_channel::ChatPanel;





fn clear_submitted_draft(panel: &ChatPanel, submitted: Option<&str>, control: &str) -> bool {
    !panel.pending
        && panel.feedback.is_none()
        && panel.text.is_empty()
        && submitted == Some(control)
}
#[cfg(test)]
mod regression {
    #[test]
    fn private_native_control_clears_only_the_successfully_submitted_text() {
        let mut panel = super::ChatPanel::default();
        assert!(super::clear_submitted_draft(&panel, Some("hello"), "hello"));
        assert!(!super::clear_submitted_draft(
            &panel,
            Some("hello"),
            "new text"
        ));
        panel.pending = true;
        assert!(!super::clear_submitted_draft(
            &panel,
            Some("hello"),
            "hello"
        ));
        panel.pending = false;
        panel.feedback = Some("refused".into());
        assert!(!super::clear_submitted_draft(
            &panel,
            Some("hello"),
            "hello"
        ));
    }
}
