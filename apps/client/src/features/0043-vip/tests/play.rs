use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::Stepper;
use oteryn_client_runtime::vip::{VipCommand, VipResponse, vip};
use oteryn_client_ui::vip::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;

struct Peer;
impl Stepper for Peer {
    fn vip_selected(&self) -> bool {
        true
    }
    async fn vip(&mut self, command: &VipCommand) -> Result<VipResponse, SessionError> {
        let request = vip::decode_request(command.kind, &command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        assert_eq!(request.operation, vip::Operation::Query);
        let payload = vip::encode_result(
            1,
            4,
            0,
            0,
            r#"{"entries":[{"number":1,"name":"Alice"}],"groups":[],"consents":[]}"#,
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
        let response = Peer.vip(&command).await?;
        oteryn_client_ui::vip::apply_result(&mut panel, &response);
        assert!(panel_text(&panel).contains("Alice"));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    })?
}
