use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    #[must_use]
    pub fn task_board_panel(&self) -> oteryn_client_ui::task_board::TaskBoardPanel {
        lock(&self.mailbox).task_board.clone()
    }
    pub fn open_task_board(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        #[cfg(windows)]
        open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).task_board.open = true;
        }
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

impl PlayLink {
    pub fn open_soulseals(&self) {
        self.task_board_full_request(oteryn_client_ui::task_board::FullIntent::Refresh);
        #[cfg(windows)]
        {
            use crate::play::feature_0115_mail::view::native::{Form, open};
            open(
                self,
                Form {
                    title: "Soul Seals — Ctrl+Alt+Shift+E",
                    labels: &[],
                    buttons: &["Refresh balance"],
                    snapshot: |m| {
                        (
                            m.task_board.soulseals_open,
                            m.task_board.pending,
                            vec![],
                            oteryn_client_ui::task_board::soulseals_text(&m.task_board),
                        )
                    },
                    set_open: |m, open| m.task_board.soulseals_open = open,
                    submit: |m, commands, _, _| {
                        if let Some(command) = oteryn_client_ui::task_board::full_request(
                            &mut m.task_board,
                            oteryn_client_ui::task_board::FullIntent::Refresh,
                        ) && commands
                            .send(crate::combat_input::PlayCommand::Feature(command))
                            .is_err()
                        {
                            m.task_board.pending = false;
                            m.task_board.feedback = Some("The session has ended");
                        }
                    },
                },
            );
        }
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).task_board.soulseals_open = true;
        }
    }
}
