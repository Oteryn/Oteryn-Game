use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult, Response,
};
use oteryn_client_ui::account_achievements::{AchievementsPanel, Intent, panel_text};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
use std::collections::VecDeque;
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    replies: VecDeque<(u32, Response)>,
}
impl Stepper for Peer {
    async fn account_achievements(&mut self, page: u32) -> Result<Response, SessionError> {
        let (wanted, reply) = self
            .replies
            .pop_front()
            .ok_or(SessionError::SessionUnusable)?;
        assert_eq!(page, wanted);
        Ok(reply)
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
        Ok(Some(LogoutResult::Accepted))
    }
}
async fn ready(
    link: &PlayLink,
    predicate: impl Fn(&AchievementsPanel) -> bool,
) -> Result<AchievementsPanel, Error> {
    Ok(tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let p = link.achievements_panel();
            if predicate(&p) {
                return p;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?)
}
#[test]
fn achievements_shell_input_pages_refusal_retry_and_logout() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let row = AccountAchievementRow {
                    key: "oteryn:achievement/explorer".into(),
                    name: "Explorer".into(),
                    description: "Discover a new place".into(),
                    grade: 2,
                    points: 5,
                    earned_at: 1234,
                    secret: true,
                };
                let first = AccountAchievementsResult {
                    total_points: 325,
                    fact_count: 65,
                    page: 0,
                    has_more: true,
                    rows: vec![row.clone(); 64],
                };
                let last = AccountAchievementsResult {
                    page: 1,
                    has_more: false,
                    rows: vec![row],
                    ..first.clone()
                };
                let peer = Peer {
                    replies: [
                        (0, Response::Page(first.clone())),
                        (1, Response::Refused),
                        (1, Response::Page(last)),
                        (0, Response::Page(first)),
                    ]
                    .into(),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(peer, incoming, outbox));
                ready(&link, |p| p.selected).await?;
                let open = NormalizedInputEvent::Key {
                    code: KeyCode::new(4)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                    repeat: false,
                };
                assert!(link.achievements_input(&[open]));
                let p = ready(&link, |p| p.page.is_some() && p.pending.is_none()).await?;
                assert!(p.open);
                let text = panel_text(&p);
                for field in [
                    "Explorer",
                    "Discover a new place",
                    "Grade 2",
                    "325 points",
                    "65 earned",
                    "1234",
                    "Secret",
                ] {
                    assert!(text.contains(field), "{field}: {text}");
                }
                link.achievements_request(Intent::Next);
                let refused = ready(&link, |p| p.error.is_some() && p.pending.is_none()).await?;
                assert_eq!(refused.page, p.page);
                link.achievements_request(Intent::Next);
                let p = ready(&link, |p| {
                    p.page.as_ref().is_some_and(|p| p.page == 1) && p.pending.is_none()
                })
                .await?;
                assert!(!p.has_next());
                link.achievements_request(Intent::Previous);
                ready(&link, |p| {
                    p.page.as_ref().is_some_and(|p| p.page == 0) && p.pending.is_none()
                })
                .await?;
                link.request_logout();
                task.await?;
                assert_eq!(link.achievements_panel(), AchievementsPanel::default());
                link.achievements_request(Intent::Refresh);
                assert!(link.achievements_panel().pending.is_none());
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "achievements scenario panicked")?
}

#[test]
fn account_browser_follows_real_page_commands_before_global_secret_filtering() -> Result<(), Error>
{
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::account_achievements::{
        AchievementFilter, catalog_next_request, catalog_rows,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.achievements_initialize();
    crate::play::mailbox::lock(&link.mailbox)
        .achievements
        .catalog
        .enabled = true;
    link.achievements_request(Intent::Refresh);
    assert!(matches!(
        commands.try_recv(),
        Ok(PlayCommand::AccountAchievements(0))
    ));
    let row = |i, secret| AccountAchievementRow {
        key: format!("oteryn:achievement/a{i}"),
        name: format!("Award {i}"),
        description: "Earned in play".into(),
        grade: 1,
        points: if secret { 100 } else { 1 },
        earned_at: 1,
        secret,
    };
    outbox.achievements_result(&Response::Page(AccountAchievementsResult {
        page: 0,
        fact_count: 65,
        total_points: 164,
        has_more: true,
        rows: (0..64).map(|i| row(i, false)).collect(),
    }));
    let next = catalog_next_request(&mut crate::play::mailbox::lock(&link.mailbox).achievements)
        .ok_or("next page")?;
    link.commands.send(PlayCommand::AccountAchievements(next))?;
    assert!(matches!(
        commands.try_recv(),
        Ok(PlayCommand::AccountAchievements(1))
    ));
    outbox.achievements_result(&Response::Page(AccountAchievementsResult {
        page: 1,
        fact_count: 65,
        total_points: 164,
        has_more: false,
        rows: vec![row(64, true)],
    }));
    let mut p = link.achievements_panel();
    assert!(p.catalog.ready);
    p.catalog.filter = AchievementFilter::Secret;
    let rows = catalog_rows(&p.catalog);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "oteryn:achievement/a64");
    Ok(())
}

#[path = "browser.rs"]
mod browser;
