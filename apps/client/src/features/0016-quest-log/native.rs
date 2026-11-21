//! Windows system controls provide a readable panel without a new rendering dependency.
//! Handles and the window-procedure context stay on this thread. The context outlives HWND.
#![allow(unsafe_code)]









pub(in crate::play) fn window_should_close(mailbox: &crate::play::mailbox::Mailbox) -> bool {
    mailbox.quest_shutdown || !mailbox.quest.open
}

pub(in crate::play) fn inline_tracker_text(
    panel: &oteryn_client_ui::quest_log::QuestPanel,
) -> String {
    oteryn_client_ui::quest_log::tracker_text(panel)
}

pub(in crate::play) fn quest_rows_changed(
    old: Option<&oteryn_client_ui::quest_log::QuestPanel>,
    panel: &oteryn_client_ui::quest_log::QuestPanel,
) -> bool {
    old.is_none_or(|old| {
        oteryn_client_ui::quest_log::quest_rows(old)
            != oteryn_client_ui::quest_log::quest_rows(panel)
    })
}

#[cfg(test)]
#[path = "tests/native_projection.rs"]
mod tests;
