use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn task_board_panel(&self) -> oteryn_client_ui::task_board::TaskBoardPanel {
        lock(&self.mailbox).task_board.clone()
    }
    pub fn open_task_board(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).task_board.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(windows)]
#[path = "soulseals_window.rs"]
mod soulseals_window;
impl PlayLink {
    pub fn open_soulseals(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        self.soulseal_catalogue_request(0);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            soulseals_window::open(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).task_board.soulseals_open = true;
        }
    }
}

#[cfg(windows)]
#[path = "weekly_results_window.rs"]
pub(in crate::play) mod weekly_results_window;

/// Snapshot-bound controls used by the shared frontend.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub(crate) enum TaskBoardControl {
    Close,
    Catalogue,
    CloseResults,
    CancelPayment,
    Request(oteryn_client_ui::task_board::FullIntent),
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn task_board_session_identity(&self) -> usize {
        std::sync::Arc::as_ptr(&self.mailbox) as usize
    }
    pub(crate) fn task_board_snapshot_valid(
        &self,
        expected: &oteryn_client_ui::task_board::TaskBoardPanel,
    ) -> bool {
        let mailbox = lock(&self.mailbox);
        !self.commands.is_closed()
            && !mailbox.world_input_blocked()
            && !mailbox.server_modal.is_active()
            && mailbox.task_board == *expected
            && expected.selected
            && !expected.pending
            && (expected.open || expected.weekly_results_open || expected.soulseals_open)
    }
    pub(crate) fn task_board_action_bound(
        &self,
        expected: &oteryn_client_ui::task_board::TaskBoardPanel,
        action: TaskBoardControl,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() {
            return false;
        }
        if matches!(
            action,
            TaskBoardControl::Close | TaskBoardControl::CloseResults
        ) {
            if matches!(action, TaskBoardControl::Close) {
                mailbox.task_board.open = false;
                mailbox.task_board.soulseals_open = false;
            } else {
                mailbox.task_board.weekly_results_open = false;
            }
            return true;
        }
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || mailbox.task_board != *expected
            || !expected.selected
            || !(expected.open || expected.weekly_results_open || expected.soulseals_open)
            || expected.pending
        {
            return false;
        }
        match action {
            TaskBoardControl::CancelPayment => return true,
            TaskBoardControl::Catalogue => {
                super::input::soulseals::send(
                    &mut mailbox,
                    &self.commands,
                    oteryn_client_ui::task_board::fights::FightIntent::Catalogue { after: 0 },
                );
            }
            TaskBoardControl::CloseResults => mailbox.task_board.weekly_results_open = false,
            TaskBoardControl::Close => mailbox.task_board.open = false,
            TaskBoardControl::Request(intent) => {
                let Some(command) =
                    oteryn_client_ui::task_board::full_request(&mut mailbox.task_board, intent)
                else {
                    return false;
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::Feature(command))
                    .is_err()
                {
                    mailbox.task_board.pending = false;
                    mailbox.task_board.feedback = Some(tr!("The session has ended"));
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use super::*;
    use oteryn_client_ui::task_board::FullIntent;
    #[test]
    fn task_board_actions_rejected_when_session_replaced() {
        let (link, mut incoming, _) = crate::play::play_channel();
        {
            let mut m = lock(&link.mailbox);
            m.task_board.selected = true;
            m.task_board.open = true;
        }
        let snapshot = link.task_board_panel();
        lock(&link.mailbox).task_board.open = false;
        let before = link.task_board_panel();
        for action in [
            TaskBoardControl::Catalogue,
            TaskBoardControl::Request(FullIntent::Refresh),
            TaskBoardControl::Request(FullIntent::Reroll),
            TaskBoardControl::Request(FullIntent::ClaimBounty),
        ] {
            assert!(!link.task_board_action_bound(&snapshot, action));
            assert_eq!(link.task_board_panel(), before);
        }
        assert!(incoming.try_recv().is_err());
    }
    #[test]
    fn task_board_bound_request_waits_for_server_and_rejects_duplicate()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut incoming, _) = crate::play::play_channel();
        {
            let mut m = lock(&link.mailbox);
            m.task_board.selected = true;
            m.task_board.open = true;
        }
        let snapshot = link.task_board_panel();
        assert!(
            link.task_board_action_bound(&snapshot, TaskBoardControl::Request(FullIntent::Refresh))
        );
        assert!(link.task_board_panel().pending);
        assert_eq!(link.task_board_panel().weekly, snapshot.weekly);
        assert!(
            !link
                .task_board_action_bound(&snapshot, TaskBoardControl::Request(FullIntent::Refresh))
        );
        assert!(matches!(
            incoming.try_recv()?,
            crate::combat_input::PlayCommand::Feature(_)
        ));
        assert!(incoming.try_recv().is_err());
        Ok(())
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl PlayLink {
    pub(crate) fn task_board_test_panel(
        &self,
        panel: oteryn_client_ui::task_board::TaskBoardPanel,
        blocked: bool,
    ) {
        let mut m = lock(&self.mailbox);
        m.task_board = panel;
        m.death_reset_commands = blocked;
    }
}
