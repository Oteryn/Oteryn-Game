use crate::play::play_channel;
struct Peer;
impl crate::play::Stepper for Peer {
    fn server_mechanic_selected(&self, cap: u32) -> bool {
        cap == 154
    }
    async fn server_mechanic(
        &mut self,
        r: &oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
    ) -> Result<oteryn_session::GatedCommandResult, oteryn_session::SessionError> {
        use oteryn_client_runtime::coin_transfer as wire;
        use oteryn_session::{CommandStatus, SessionError};
        assert_eq!((r.capability, r.command_type), (154, wire::COMMAND));
        let r = wire::decode(&r.payload).map_err(|_| SessionError::SessionUnusable)?;
        let h = r
            .operation
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let op = format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        );
        let view = if r.action == 1 {
            serde_json::json!({"result":"ok","quote":"019a65f0-0000-7000-8000-000000000001","recipient":r.recipient,"amount":r.amount.to_string(),"transferable":"100","non_transferable":"20"})
        } else {
            serde_json::json!({"result":"ok","operation":op,"pending":false,"receipt":{"result":"ok","receipt":{"operation_id":op,"kind":"transfer","amount":r.amount.to_string(),"transferable_after":"75","non_transferable_after":"20","occurred_at":"2026-11-09T00:00:00Z"}}})
        };
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&view.to_string())
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(
        &mut self,
        duration: std::time::Duration,
    ) -> Result<(), oteryn_session::SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        vec![]
    }
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }
}

#[test]
fn coin_gift_session_loop_applies_only_the_platform_receipt()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(crate::play::run_session(Peer, incoming, outbox));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while !link.store_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let fields = ["Alice".into(), "25".into()];
                assert!(link.coin_gift_control(0, fields.clone()));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while link.store_panel().gift.pending.is_some() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.store_panel().gift.receipt.is_empty());
                assert!(link.coin_gift_control(1, fields));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while link.store_panel().gift.pending.is_some() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let p = link.store_panel();
                assert!(p.gift.confirmed.is_none());
                assert!(p.gift.receipt.contains("Balance after: 95"));
                assert!(p.gift.receipt.contains("Gift receipt:"));
                drop(link);
                task.await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })?
        })?
        .join()
        .map_err(|_| "gift session thread")?
}
