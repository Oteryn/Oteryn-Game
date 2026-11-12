use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::stash::{StashCommand, StashResponse, stash};
use oteryn_client_ui::stash::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Default)]
struct Peer {
    queries: Option<std::sync::Arc<std::sync::Mutex<Vec<stash::StashFilter>>>>,
}
impl Stepper for Peer {
    fn stash_selected(&self) -> bool {
        true
    }
    async fn stash(&mut self, command: &StashCommand) -> Result<StashResponse, SessionError> {
        let (intent, filter) = stash::decode_stash_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if let Some(queries) = &self.queries && let Some(filter) = &filter {
            queries.lock().map_err(|_| SessionError::SessionUnusable)?.push(filter.clone());
        }
        if let Some(filter) = filter && !filter.search.is_empty() {
            assert_eq!(filter.search, "apple");
            assert_eq!(filter.category, "other");
            assert_eq!(filter.trader, "Sam");
            assert_eq!(intent.page, 0);
        }
        let reply = stash::StashReply {
            outcome: stash::StashOutcome::Opened,
            stowed_quantity: 0,
            transaction: None,
            page: intent.page,
            has_more: false,
            rows: vec![stash::StashRow {
                handle: [7; 16],
                definition_key: "item.apple".into(),
                definition_revision: "r1".into(),
                quantity: 12,
                migration_required: false,
            }],
        };
        let wire = stash::encode_stash_reply(&reply).map_err(|_| SessionError::SessionUnusable)?;
        Ok(StashResponse {
            status: CommandStatus::Accepted,
            reply: stash::decode_stash_reply(&wire).map_err(|_| SessionError::SessionUnusable)?,
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
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.stash_panel();
            if panel.selected && !panel.pending && !panel.rows.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn stash_panel_request_uses_codec_and_applies_encoded_reply() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(Peer::default(), incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.stash_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.stash_request(Intent::Refresh);
                ready(&link).await?;
                assert!(panel_text(&link.stash_panel()).contains("item.apple"));
                link.stash_filter("apple".into(), "other".into(), "Sam".into());
                ready(&link).await?;
                assert_eq!(link.stash_panel().trader, "Sam");
                assert!(link.stash_panel().server_filtered);
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Stash UI test thread panicked")?
}

#[test]
fn stash_market_round_trip_keeps_context() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let queries = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                let task = tokio::spawn(run_session(
                    Peer {
                        queries: Some(queries.clone()),
                    },
                    incoming,
                    outbox,
                ));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.stash_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(!link.market_stashback());
                use oteryn_input_actions::{
                    ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
                };
                assert!(link.stash_input(&[NormalizedInputEvent::Key {
                    code: KeyCode::new(22)?,
                    state: ButtonState::Pressed,
                    modifiers: Modifiers::one(Modifier::Control),
                    repeat: false,
                }]));
                ready(&link).await?;
                link.stash_filter("apple".into(), "other".into(), "Sam".into());
                ready(&link).await?;
                link.stash_sort();
                ready(&link).await?;
                link.open_market();
                assert!(!link.stash_panel().open, "Market must replace Stash");
                assert!(link.market_stashback());
                ready(&link).await?;
                let panel = link.stash_panel();
                assert_eq!(panel.sort, oteryn_client_ui::stash::Sort::Stock);
                assert!(!link.market_stashback());
                assert_eq!(
                    (&*panel.query, &*panel.category, &*panel.trader),
                    ("apple", "other", "Sam")
                );
                {
                    let observed = queries.lock().map_err(|_| "query log poisoned")?;
                    assert_eq!(observed.len(), 4);
                    let returned = observed.last().ok_or("stashback query")?;
                    assert_eq!(
                        (&*returned.search, &*returned.category, &*returned.trader),
                        ("apple", "other", "Sam")
                    );
                    assert!(returned.stock_order);
                }
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Stash round trip thread panicked")?
}

#[test]
fn stash_session_end_preserves_sort_only() {
    let (link, _, _) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.stash.sort = oteryn_client_ui::stash::Sort::Stock;
    mailbox.stash.query = "apple".into();
    mailbox.stash_end();
    assert_eq!(mailbox.stash.sort, oteryn_client_ui::stash::Sort::Stock);
    assert!(mailbox.stash.query.is_empty());
}

#[test]
fn stash_sort_is_saved_and_restored_in_a_new_session() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                use crate::play::SettingsState;
                let path = std::env::temp_dir()
                    .join(format!("oteryn-stash-sort-{}.json", std::process::id()));
                let mut settings = SettingsState::new(Some(path.clone()));
                let (link, incoming, outbox) = play_channel();
                settings.sync_stash_sort(&link);
                let task = tokio::spawn(run_session(Peer::default(), incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.stash_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.stash_sort();
                ready(&link).await?;
                settings.sync_stash_sort(&link);
                assert!(settings.preferences.stash_stock_order);
                drop(link);
                task.await?;

                let mut restored = SettingsState::new(Some(path.clone()));
                let (link, incoming, outbox) = play_channel();
                restored.sync_stash_sort(&link);
                assert_eq!(
                    link.stash_panel().sort,
                    oteryn_client_ui::stash::Sort::Stock
                );
                let task = tokio::spawn(run_session(Peer::default(), incoming, outbox));
                tokio::time::timeout(Duration::from_secs(3), async {
                    while !link.stash_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                link.open_stash();
                ready(&link).await?;
                assert_eq!(
                    link.stash_panel().sort,
                    oteryn_client_ui::stash::Sort::Stock
                );
                link.stash_sort();
                ready(&link).await?;
                restored.sync_stash_sort(&link);
                assert!(
                    !SettingsState::new(Some(path.clone()))
                        .preferences
                        .stash_stock_order
                );
                drop(link);
                task.await?;
                std::fs::remove_file(path)?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Stash preferences thread panicked")?
}
