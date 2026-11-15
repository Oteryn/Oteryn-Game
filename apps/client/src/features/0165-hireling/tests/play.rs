use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::hireling::{CommandStatus, HirelingCommand, HirelingResponse, hireling};
use oteryn_client_ui::hireling::{Intent, panel_text};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    commands: Arc<Mutex<Vec<hireling::Request>>>,
}
impl Stepper for Peer {
    fn hireling_selected(&self) -> bool {
        true
    }
    async fn hireling(
        &mut self,
        request: &HirelingCommand,
    ) -> Result<HirelingResponse, SessionError> {
        let decoded =
            hireling::Request::decode(&request.0.encode()).ok_or(SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(decoded);
        Ok(HirelingResponse {
            status: CommandStatus::Accepted,
            active: hireling::decode_reply(&hireling::encode_reply(!matches!(
                decoded,
                hireling::Request::Dismiss
            ))),
        })
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}

async fn ready(link: &PlayLink) {
    while !link.hireling_panel().selected || link.hireling_panel().pending.is_some() {
        tokio::task::yield_now().await;
    }
}

#[test]
fn hireling_controls_use_real_codec_requests_and_apply_replies() -> Result<(), Error> {
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let (link, incoming, outbox) = play_channel();
        let task = tokio::spawn(run_session(
            Peer {
                commands: Arc::clone(&commands),
            },
            incoming,
            outbox,
        ));
        ready(&link).await;
        link.hireling_request(Intent::Refresh);
        ready(&link).await;
        link.hireling_target(7, 9, -1);
        link.hireling_request(Intent::Summon);
        ready(&link).await;
        assert!(panel_text(&link.hireling_panel()).contains("summoned"));
        link.hireling_request(Intent::Dismiss);
        ready(&link).await;
        assert_eq!(
            *commands.lock().unwrap_or_else(PoisonError::into_inner),
            vec![
                hireling::Request::Query,
                hireling::Request::Summon {
                    x: 7,
                    y: 9,
                    floor: -1
                },
                hireling::Request::Dismiss
            ]
        );
        drop(link);
        task.await?;
        Ok::<(), Error>(())
    })??;
    Ok(())
}

#[path = "../native.rs"]
mod native_notifications;
#[test]
fn native_hireling_notifications_dispatch_real_requests() -> Result<(), Error> {
    #[cfg(windows)]
    let _open: fn(&PlayLink) = native_notifications::open_window;
    let (link, mut commands, _) = play_channel();
    let mut m = crate::play::mailbox::lock(&link.mailbox);
    m.hireling.selected = true;
    m.hireling.target = Some((7, 9, -1));
    for (id, expected) in [
        (
            101,
            hireling::Request::Summon {
                x: 7,
                y: 9,
                floor: -1,
            },
        ),
        (102, hireling::Request::Dismiss),
        (103, hireling::Request::Query),
    ] {
        let mut actions = Vec::new();
        native_notifications::record_notification(&mut actions, id | (1 << 16));
        assert!(actions.is_empty());
        native_notifications::record_notification(&mut actions, id);
        native_notifications::dispatch_notifications(&mut m, &link.commands, actions);
        assert!(
            matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Hireling(c) if c.0 == expected)
        );
        m.hireling.pending = None;
    }
    Ok(())
}

#[test]
fn native_startup_failure_releases_hireling_and_tracker_open_flags() {
    let (link, _, _) = play_channel();
    let mut m = crate::play::mailbox::lock(&link.mailbox);
    m.hireling.open = true;
    m.cyclopedia_tracker.open = true;
    assert!(!crate::play::mailbox::native_startup_result(
        &mut m.hireling.open,
        false
    ));
    assert!(!m.hireling.open);
    assert!(!crate::play::mailbox::native_startup_result(
        &mut m.cyclopedia_tracker.open,
        false
    ));
    assert!(!m.cyclopedia_tracker.open);
    m.hireling.open = true;
    assert!(crate::play::mailbox::native_startup_result(
        &mut m.hireling.open,
        true
    ));
    assert!(m.hireling.open);
}
