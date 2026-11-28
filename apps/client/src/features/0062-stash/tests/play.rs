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
    reverse_pages: bool,
    emptied_last: bool,
    grow: bool,
    fail_probe: bool,
    queries: Option<std::sync::Arc<std::sync::Mutex<Vec<stash::StashFilter>>>>,
}
impl Stepper for Peer {
    fn stash_selected(&self) -> bool {
        true
    }
    async fn stash(&mut self, command: &StashCommand) -> Result<StashResponse, SessionError> {
        let (intent, filter) = stash::decode_stash_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if let Some(queries) = &self.queries
            && let Some(filter) = &filter
        {
            queries
                .lock()
                .map_err(|_| SessionError::SessionUnusable)?
                .push(filter.clone());
        }
        if let Some(filter) = filter
            && !filter.search.is_empty()
        {
            assert_eq!(filter.search, "apple");
            assert_eq!(filter.category, "other");
            assert_eq!(filter.trader, "Sam");
            assert_eq!(intent.page, 0);
        }
        let mut reply = stash::StashReply {
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
        if self.reverse_pages {
            if self.fail_probe && self.emptied_last && intent.action == stash::StashAction::Browse {
                return Err(SessionError::SessionUnusable);
            }
            let last = if self.grow {
                4
            } else if self.emptied_last {
                2
            } else {
                3
            };
            reply.has_more = intent.page < last;
            if intent.page > last {
                reply.rows.clear();
            } else {
                reply.rows[0].handle = [intent.page as u8 + 1; 16];
            }
            if matches!(intent.action, stash::StashAction::Stow(_)) {
                self.grow = true;
                reply.outcome = stash::StashOutcome::Committed;
                reply.transaction = Some([8; 16]);
            }
            if let stash::StashAction::Withdraw { row, .. } = intent.action {
                if intent.page == 3 {
                    assert!(!self.emptied_last, "withdrawal must execute once");
                    assert_eq!(row, [4; 16]);
                    self.emptied_last = true;
                    reply.rows.clear();
                    reply.outcome = stash::StashOutcome::Committed;
                    reply.transaction = Some([9; 16]);
                } else {
                    assert_eq!(intent.page, 2);
                    assert_eq!(row, [3; 16]);
                }
            }
        }
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
                        ..Default::default()
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
                let fixture = tempfile::tempdir()?;
                let path = fixture.path().join("oteryn-stash-sort.json");
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

#[test]
fn reverse_stash_pages_start_at_last_and_keep_withdraw_handle() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        reverse_pages: true,
                        ..Default::default()
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
                link.stash_reverse();
                ready(&link).await?;
                let page = link.stash_panel();
                assert_eq!(page.page, 0);
                assert!(page.has_more);
                assert_eq!(page.rows[0].handle, [4; 16]);
                link.stash_request(Intent::Next);
                ready(&link).await?;
                let page = link.stash_panel();
                assert_eq!(page.page, 1);
                assert_eq!(page.rows[0].handle, [3; 16]);
                link.stash_request(Intent::Withdraw { row: 0, amount: 1 });
                ready(&link).await?;
                link.stash_request(Intent::Previous);
                ready(&link).await?;
                link.stash_request(Intent::Withdraw { row: 0, amount: 12 });
                ready(&link).await?;
                let refreshed = link.stash_panel();
                assert_eq!(refreshed.page, 0);
                assert_eq!(refreshed.rows[0].handle, [3; 16]);
                // The editor limits characters, while the protocol limits UTF-8 bytes.
                link.stash_filter("é".repeat(65), String::new(), String::new());
                link.stash_reverse();
                link.stash_sort();
                let rejected = link.stash_panel();
                assert_eq!(rejected.reverse, refreshed.reverse);
                assert_eq!(rejected.last_server_page, refreshed.last_server_page);
                assert_eq!(rejected.sort, refreshed.sort);
                assert_eq!(rejected.sort_edit, refreshed.sort_edit);
                assert_eq!(rejected.rows, refreshed.rows);
                assert!(!rejected.pending);
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "reverse stash session panicked")?
}

#[tokio::test]
async fn reversed_mutation_rediscovers_end_on_a_nonzero_page() -> Result<(), Error> {
    let (link, _, outbox) = play_channel();
    {
        let mut m = crate::play::mailbox::lock(&outbox.0);
        m.stash.reverse = true;
        m.stash.last_server_page = Some(3);
    }
    let mut peer = Peer {
        reverse_pages: true,
        ..Default::default()
    };
    let command = StashCommand::filtered(
        stash::StashIntent {
            page: 1,
            action: stash::StashAction::Stow(1u64.try_into()?),
        },
        None,
    )
    .ok_or("command")?;
    let response = outbox.stash_execute(&mut peer, &command).await?;
    assert_eq!(response.reply.transaction, Some([8; 16]));
    assert_eq!(link.stash_panel().last_server_page, Some(4));
    assert_eq!(response.reply.page, 1);
    assert_eq!(response.reply.rows[0].handle, [4; 16]);
    Ok(())
}

#[tokio::test]
async fn reversed_withdrawal_preserves_receipt_on_failed_page_probe() -> Result<(), Error> {
    let (_, _, outbox) = play_channel();
    {
        let mut m = crate::play::mailbox::lock(&outbox.0);
        m.stash.reverse = true;
        m.stash.last_server_page = Some(3);
    }
    let mut peer = Peer {
        reverse_pages: true,
        fail_probe: true,
        ..Default::default()
    };
    let command = StashCommand::filtered(
        stash::StashIntent {
            page: 0,
            action: stash::StashAction::Withdraw {
                row: [4; 16],
                amount: 12,
            },
        },
        None,
    )
    .ok_or("command")?;
    let response = outbox.stash_execute(&mut peer, &command).await?;
    assert_eq!(response.reply.outcome, stash::StashOutcome::Committed);
    assert_eq!(response.reply.transaction, Some([9; 16]));
    Ok(())
}

#[test]
fn supply_stash_refuses_non_stackable_stow_as_unsupported() -> Result<(), Error> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::depot::ItemEntry;
    let (link, mut commands, outbox) = play_channel();
    outbox.stash_initialize(true);
    // Both entries have a count of one. Appearance 3031 is gold, 3272 is a sword.
    let sword = ItemEntry {
        appearance_id: 3272,
        handle: 1u64.try_into()?,
        item_definition_ref: 3272u32.try_into()?,
        count: 1u32.try_into()?,
        sub_type: 0,
    };
    let gold = ItemEntry {
        appearance_id: 3031,
        handle: 2u64.try_into()?,
        item_definition_ref: 3031u32.try_into()?,
        ..sword
    };
    {
        let mut mailbox = crate::play::mailbox::lock(&outbox.0);
        mailbox.stash.open = true;
        mailbox.stash.carried = vec![sword, gold];
    }
    use oteryn_client_assets::{
        AppearanceIndex, AssetStore, Catalog,
        world::{MANIFEST_PATH, MANIFEST_SHA256},
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(MANIFEST_PATH),
        MANIFEST_SHA256,
    )?;
    let appearances = AppearanceIndex::load(&store, &Catalog::load(&store)?)?;
    let stackable = |appearance| {
        appearances
            .get(u32::from(appearance))
            .map(|item| item.stackable)
    };
    assert_eq!(stackable(sword.appearance_id), Some(false));
    assert_eq!(stackable(gold.appearance_id), Some(true));
    link.stash_stow_selected(&link.stash_panel(), 0, stackable);
    let panel = link.stash_panel();
    assert_eq!(panel.outcome, Some(stash::StashOutcome::Unsupported));
    assert!(panel_text(&panel).contains("Only stackable items"));
    assert!(!panel.pending);
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));

    link.stash_stow_selected(&panel, 1, stackable);
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("stackable stow missing".into());
    };
    assert_eq!(command.intent.action, stash::StashAction::Stow(gold.handle));
    assert!(link.stash_panel().pending);
    Ok(())
}
