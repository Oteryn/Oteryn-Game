//! Uses real run_session and the exact borrowed native callbacks; old148 writer remains unchanged.
use super::*;
use crate::play::feature_0148_wheel_allocation::input::gem_presentation::{
    Action, activate, query,
};
use oteryn_client_runtime::{
    gem_presentation as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
struct Live(Peer);
impl Stepper for Live {
    fn wheel_selected(&self) -> bool {
        true
    }
    fn feature_selected(&self, c: u32) -> bool {
        c == 148
    }
    async fn wheel(&mut self, c: &WheelCommand) -> Result<WheelResponse, SessionError> {
        self.0.wheel(c).await
    }
    async fn feature(&mut self, c: &FeatureCommand) -> Result<GatedCommandResult, SessionError> {
        assert_eq!((c.capability, c.kind), (148, 6121));
        let request = wire::Request::decode(&c.payload).ok_or(SessionError::SessionUnusable)?;
        assert_eq!(request.revision, self.0.atelier.revision);
        let reply = wire::Reply {
            grades_current: true,
            qualified_gems: vec![[17; 16]],
            request,
            catalogue: "wheel-reference-v1".into(),
            vocation: "monk".into(),
            modifiers: vec![wire::Modifier {
                kind: 0,
                id: 48,
                name: "Capacity".into(),
                effects: vec![wire::Effect {
                    name: "Capacity".into(),
                    target: String::new(),
                    unit: 1,
                    values: [500000, 550000, 600000, 750000],
                }],
            }],
        };
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().ok_or(SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        self.0.serve(d).await
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, d: StepDirection) -> Result<StepOutcome, SessionError> {
        self.0.step(d).await
    }
    async fn combat(&mut self, c: CombatCommand) -> Result<CombatReport, SessionError> {
        self.0.combat(c).await
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        self.0.logout().await
    }
}
#[test]
fn gem_presentation_actual_session_stats_selection_review_confirm_and_old_mutation()
-> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let peer = Live(Peer {
                    wheel: wheel::WheelAllocationReply {
                        refused: false,
                        revision: 4,
                        points: [0; 36],
                        vessels: None,
                    },
                    atelier: atelier::AtelierReply {
                        refused: false,
                        revision: 3,
                        gems: vec![atelier::GemView {
                            id: [17; 16],
                            quality: 2,
                            domain: 0,
                            locked: false,
                            vessel: false,
                            modifiers: vec![48, 0, 5],
                        }],
                        grades: Vec::new(),
                        quality_quotes: vec![],
                        grade_quotes: vec![atelier::GradeQuote {
                            kind: 0,
                            grade: 1,
                            gold: 999,
                            fragments: 12,
                        }],
                    },
                    commands: commands.clone(),
                });
                let (link, rx, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, rx, outbox));
                ready(&link, None).await?;
                link.wheel_request(Intent::Refresh);
                ready(&link, Some((4, 3))).await?;
                assert_eq!(link.wheel_panel().grade(0, 48), 0);
                query(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                );
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.wheel_panel().gems.reply.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let nonce = link
                    .wheel_panel()
                    .gems
                    .reply
                    .as_ref()
                    .ok_or("reply")?
                    .request
                    .nonce;
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Modifier {
                        nonce,
                        kind: 0,
                        id: 48,
                    },
                );
                assert!(
                    oteryn_client_ui::wheel_allocation::gems::view::details(&link.wheel_panel())
                        .contains("Grade 4: +750")
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Stage(Intent::GradeUp),
                );
                let count = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .len();
                assert!(
                    link.wheel_panel()
                        .gems
                        .review
                        .as_ref()
                        .ok_or("review")?
                        .text
                        .contains("999 gold and 12 basic fragments")
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Cancel,
                );
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    count
                );
                activate(
                    &mut crate::play::mailbox::lock(&link.mailbox),
                    &link.commands,
                    Action::Stage(Intent::GradeUp),
                );
                link.gem_confirm();
                link.gem_confirm();
                ready(&link, Some((4, 4))).await?;
                let paid = commands
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .iter()
                    .filter(|c| c.kind == 424)
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(paid.len(), 1);
                let request =
                    atelier::decode_mutate(&paid[0].payload).map_err(|_| "paid decode")?;
                assert_eq!(request.expected_revision, 3);
                assert_eq!(
                    request.mutation,
                    atelier::Mutation::GradeUp {
                        kind: 0,
                        mod_id: 48
                    }
                );
                assert_eq!(request.occurrence[6] >> 4, 7);
                assert_eq!(link.wheel_panel().grade(0, 48), 1);
                crate::play::mailbox::lock(&link.mailbox).cancel_activity_requests();
                assert!(link.wheel_panel().gems.reply.is_none());
                assert!(link.wheel_panel().gems.review.is_none());
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Gem presentation session thread panicked")?
}
