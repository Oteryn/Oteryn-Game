use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::Stepper;
use oteryn_client_runtime::vip::{VipCommand, VipResponse, vip};
use oteryn_client_ui::vip::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;

struct Peer {
    added: bool,
}
impl Stepper for Peer {
    fn vip_selected(&self) -> bool {
        true
    }
    async fn vip(&mut self, command: &VipCommand) -> Result<VipResponse, SessionError> {
        let request = vip::decode_request(command.kind, &command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if request.operation == vip::Operation::Add {
            assert_eq!(request.text, "Alice");
            self.added = true;
        } else {
            assert_eq!(request.operation, vip::Operation::Query);
        }
        let payload = vip::encode_result(
            1,
            4,
            0,
            0,
            if request.operation == vip::Operation::Add { "" } else if self.added {
                r#"{"entries":[{"entry":1,"name":"Alice","description":"","confirmed":false}],"groups":[],"consents":[]}"#
            } else { r#"{"entries":[],"groups":[],"consents":[]}"# },
        )
        .map_err(|_| SessionError::SessionUnusable)?;
        Ok(VipResponse {
            status: CommandStatus::Accepted,
            result: vip::decode_result(&payload).map_err(|_| SessionError::SessionUnusable)?,
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

#[test]
fn vip_query_uses_the_wire_codec_and_applies_the_server_view()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let runtime = oteryn_client_runtime::ClientRuntime::new()?;
    runtime.block_on(async {
        let mut panel = oteryn_client_ui::vip::VipPanel {
            selected: true,
            ..Default::default()
        };
        let command =
            oteryn_client_ui::vip::request(&mut panel, Intent::Query(0)).ok_or("command")?;
        let response = Peer { added: true }.vip(&command).await?;
        oteryn_client_ui::vip::apply_result(&mut panel, &response);
        assert!(panel_text(&panel).contains("Alice"));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })?
}

#[test]
fn vip_management_projects_named_rows_and_hides_handles() -> Result<(), Box<dyn std::error::Error>>
{
    let view = crate::play::feature_0043_vip::view::management_view(Some(
        r#"{"entries":[{"entry":73,"name":"Alice","description":"Friend","icon":2,"notify":true,"groups":[1],"confirmed":true}],"groups":[{"group":1,"name":"Friends"}],"consents":[{"handle":99,"name":"Bob","accepted":false}],"next_entry":73}"#,
    ))?;
    assert_eq!(view.entries[0].number, 73);
    assert_eq!(view.entries[0].label, "Alice — Friend (confirmed)");
    assert_eq!(view.groups[0].label, "Friends");
    assert_eq!(view.consents[0].label, "Bob (pending)");
    assert_eq!(view.next_entry, 73);
    Ok(())
}

#[test]
fn vip_shell_shortcut_runs_through_the_session_loop()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                use crate::play::{play_channel, run_session};
                use oteryn_input_actions::{
                    ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer { added: false }, incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.vip_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let event = |repeat, modifiers| NormalizedInputEvent::Key {
                    code: KeyCode::new(25).unwrap_or(KeyCode::KEY_A),
                    state: ButtonState::Pressed,
                    modifiers,
                    repeat,
                };
                assert!(!link.vip_input(&[event(true, Modifiers::one(Modifier::Control))]));
                assert!(!link.vip_input(&[event(false, Modifiers::NONE)]));
                assert!(!link.vip_input(&[event(
                    false,
                    Modifiers::one(Modifier::Control).with(Modifier::Shift)
                )]));
                assert!(link.vip_input(&[event(false, Modifiers::one(Modifier::Control))]));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.vip_panel().pending || link.vip_panel().view.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.vip_panel().open);
                assert!(!panel_text(&link.vip_panel()).contains("Alice"));
                link.vip_request(Intent::Add("Alice".into()));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.vip_panel().pending
                        || !panel_text(&link.vip_panel()).contains("Alice")
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(panel_text(&link.vip_panel()).contains("Alice"));
                drop(link);
                task.await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })?
        })?
        .join()
        .map_err(|_| "VIP scenario thread panicked")?
}

#[test]
fn vip_refreshes_only_successful_mutations_and_gates_pending_requests()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::feature_0043_vip::protocol::refresh_after;
    use oteryn_client_ui::vip::{VipPanel, request};
    let mut panel = VipPanel::default();
    assert!(request(&mut panel, Intent::Add("Alice".into())).is_none());
    panel.selected = true;
    let add = request(&mut panel, Intent::Add("Alice".into())).ok_or("add")?;
    assert!(request(&mut panel, Intent::Query(0)).is_none());
    let mut response = VipResponse {
        status: CommandStatus::Accepted,
        result: vip::ResultView {
            code: 1,
            revision: 1,
            number: 1,
            wait: 0,
            view: String::new(),
        },
    };
    let refresh = refresh_after(&add, &response).ok_or("refresh")?;
    assert_eq!(
        vip::decode_request(refresh.kind, &refresh.payload)
            .map_err(|_| "invalid refresh")?
            .operation,
        vip::Operation::Query
    );
    assert!(refresh_after(&refresh, &response).is_none());
    response.result.code = 4;
    assert!(refresh_after(&add, &response).is_none());
    oteryn_client_ui::vip::apply_result(&mut panel, &response);
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("VIP list is full"));
    Ok(())
}

#[test]
fn vip_window_closes_when_the_play_link_is_dropped() {
    let (link, _incoming, _outbox) = crate::play::play_channel();
    let shared = std::sync::Arc::clone(&link.mailbox);
    crate::play::mailbox::lock(&shared).vip.open = true;
    drop(link);
    assert!(!crate::play::mailbox::lock(&shared).vip.open);
}
