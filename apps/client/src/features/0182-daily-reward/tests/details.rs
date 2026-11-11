use super::*;
use oteryn_client_runtime::raw_feature::FeatureCommand;
impl Stepper for DetailsPeer {
    fn daily_reward_selected(&self) -> bool {
        true
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        if command.kind == daily_reward::returner::QUERY {
            return Ok(oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: daily_reward::returner::Reply { state: 0, id: None }
                    .encode()
                    .map_err(|_| SessionError::SessionUnusable)?,
            });
        }
        assert_eq!((command.capability, command.kind), (182, 738));
        assert!(command.payload.is_empty());
        let mut details = daily_reward::details::Details::empty(50);
        details.days[29].premium.cosmetic_tokens = 10;
        details.history[0] = Some(daily_reward::details::Receipt {
            epoch: 49,
            reset_day: 20000,
            lane_day: 30,
            streak: 60,
            jokers_spent: 1,
            own_character: false,
        });
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: details
                .encode()
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(d).await;
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
struct DetailsPeer;
#[test]
fn daily_reward_catalogue_and_account_history_cross_real_session_caller() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let task = runtime.spawn(run_session(DetailsPeer, commands, outbox))?;
                ready(&link, false).await?;
                link.daily_reward_details_request();
                link.daily_reward_details_request();
                let panel = tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        let panel = link.daily_reward_panel();
                        // A second accepted refresh keeps the previous details while pending.
                        if panel.details.is_some() && !panel.pending {
                            break panel;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(!panel.pending);
                assert!(panel.wall.is_none());
                let text = panel_text(&panel);
                assert!(text.contains("Day 30"));
                assert!(text.contains("another account character"));
                assert!(text.contains("streak 60"));
                assert!(panel.claim_receipts.iter().all(Option::is_none));
                task.abort();
                Ok::<(), Error>(())
            })
        })?
        .join()
        .map_err(|_| "reward details scenario")??
}
