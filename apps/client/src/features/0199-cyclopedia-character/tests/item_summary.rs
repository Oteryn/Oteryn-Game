use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{
    cyclopedia_character::items::{Location, Page, Row},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_character::Intent;
#[test]
fn review_native_holdings_row_and_detail_use_display_names() {
    use crate::play::feature_0199_cyclopedia_character::view::native::{
        item_summary_detail_text, item_summary_row_text,
    };
    let mut items = oteryn_client_ui::cyclopedia_character::ItemSummary::default();
    let row = Row {
        definition: "oteryn:item/arrow".into(),
        quantities: [2, 3, 4, 5],
    };
    items
        .names
        .insert(row.definition.clone(), "Golden arrow".into());
    assert_eq!(
        item_summary_row_text(&items, &row),
        "Golden arrow\t2\t3\t4\t5"
    );
    assert_eq!(
        item_summary_detail_text(&items, &row),
        "Golden arrow\r\nInventory: 2 · Depot: 3 · Stash: 4 · Inboxes: 5"
    );
    items.names.clear();
    assert_eq!(
        item_summary_row_text(&items, &row),
        "oteryn:item/arrow\t2\t3\t4\t5"
    );
    assert_eq!(
        item_summary_detail_text(&items, &row),
        "oteryn:item/arrow\r\nInventory: 2 · Depot: 3 · Stash: 4 · Inboxes: 5"
    );
}
#[test]
fn t095_item_summary_caller_requests_all_locations_and_filters_confirmed_rows()
-> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Items);
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!(command.payload, [130, 0, 0]);
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: Page {
            offset: 0,
            total: 2,
            rows: vec![
                Row {
                    definition: "oteryn:item/arrow".into(),
                    quantities: [2, 3, 4, 5],
                },
                Row {
                    definition: "oteryn:item/sword".into(),
                    quantities: [1, 0, 0, 0],
                },
            ],
        }
        .encode()
        .map_err(|_| "encode")?,
    });
    let mut panel = link.cyclopedia_character_panel();
    assert_eq!(panel.items.rows().len(), 2);
    for location in [Location::Depot, Location::Stash, Location::Inbox] {
        panel.items.location = Some(location);
        let rows = panel.items.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].definition, "oteryn:item/arrow");
    }
    panel.items.location = Some(Location::Inventory);
    panel.items.search = "sword".into();
    assert_eq!(panel.items.rows()[0].quantities, [1, 0, 0, 0]);
    link.cyclopedia_character_request(Intent::ItemsNext);
    assert!(commands.try_recv().is_err());
    let confirmed = link.cyclopedia_character_panel().items.page;
    link.cyclopedia_character_request(Intent::Items);
    assert!(commands.try_recv().is_ok());
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: vec![130],
    });
    let panel = link.cyclopedia_character_panel();
    assert_eq!(panel.items.page, confirmed);
    assert!(panel.feedback.is_some());
    Ok(())
}

#[test]
fn store_inbox_opens_with_an_unfiltered_holdings_query() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::items::filtered;

    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    outbox.holdings_initialize(true);
    assert!(link.cyclopedia_holdings_filter("sword", Some(Location::Stash)));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing Stash query".into());
    };
    let stash = filtered::Query::decode(&command.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(stash.location, Some(Location::Stash));
    assert_eq!(stash.search, "sword");
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: filtered::Page {
            holdings: Page {
                offset: 0,
                total: 0,
                rows: Vec::new(),
            },
            names: Vec::new(),
        }
        .encode()
        .map_err(|error| format!("{error:?}"))?,
    });

    link.store_inbox_set_open(true);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("missing Store Inbox query".into());
    };
    assert_eq!(
        filtered::Query::decode(&command.payload).map_err(|error| format!("{error:?}"))?,
        filtered::Query {
            offset: 0,
            location: None,
            search: String::new(),
        }
    );
    Ok(())
}

#[test]
fn store_inbox_discards_an_in_flight_stash_reply_and_requests_all_locations() -> InboxResult {
    inbox_receives_superseded_stash_reply(true)
}

#[test]
fn closing_store_inbox_still_discards_the_in_flight_stash_reply() -> InboxResult {
    inbox_receives_superseded_stash_reply(false)
}

type InboxResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn inbox_receives_superseded_stash_reply(keep_open: bool) -> InboxResult {
    use crate::combat_input::{CombatCommand, CombatReport};
    use crate::play::{PlayView, Stepper, run_session};
    use oteryn_client_runtime::{
        cyclopedia_character::items::filtered, raw_feature::FeatureCommand,
    };
    use oteryn_session::{LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome};
    use std::time::Duration;
    use tokio::sync::mpsc;

    const WAIT: Duration = Duration::from_secs(2);
    struct Peer {
        seen: mpsc::UnboundedSender<FeatureCommand>,
        replies: mpsc::UnboundedReceiver<GatedCommandResult>,
    }
    impl Stepper for Peer {
        fn feature_selected(&self, capability: u32) -> bool {
            matches!(capability, 199 | 506)
        }
        async fn feature(
            &mut self,
            command: &FeatureCommand,
        ) -> Result<GatedCommandResult, SessionError> {
            self.seen
                .send(command.clone())
                .map_err(|_| SessionError::SessionUnusable)?;
            self.replies
                .recv()
                .await
                .ok_or(SessionError::SessionUnusable)
        }
        async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
            tokio::time::sleep(duration).await;
            Ok(())
        }
        fn drain(&mut self) -> Vec<SessionEvent> {
            vec![]
        }
        async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
            Err(SessionError::SessionUnusable)
        }
        async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
            Ok(None)
        }
    }

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                let (link, commands, outbox) = play_channel();
                let (seen, mut received) = mpsc::unbounded_channel();
                let (reply, replies) = mpsc::unbounded_channel();
                let task = tokio::spawn(run_session(Peer { seen, replies }, commands, outbox));
                let mut view = PlayView::new(oteryn_renderer::TileCoord::new(10, 10), -7, &[])?;
                tokio::time::timeout(WAIT, async {
                    while !link.cyclopedia_character_panel().holdings_selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                let send = |id, definition: &str, quantities| -> InboxResult {
                    reply.send(GatedCommandResult {
                        command_id: id,
                        server_sequence: id,
                        status: CommandStatus::Accepted,
                        payload: filtered::Page {
                            holdings: Page {
                                offset: 0,
                                total: 1,
                                rows: vec![Row {
                                    definition: definition.into(),
                                    quantities,
                                }],
                            },
                            names: vec![definition.into()],
                        }
                        .encode()
                        .map_err(|error| format!("{error:?}"))?,
                    })?;
                    Ok(())
                };
                assert!(link.cyclopedia_holdings_filter("sword", Some(Location::Stash)));
                let stash = tokio::time::timeout(WAIT, received.recv())
                    .await?
                    .ok_or("Stash query")?;
                assert_eq!((stash.capability, stash.kind), (199, 388));
                link.store_inbox_set_open(true);
                if !keep_open {
                    link.store_inbox_set_open(false);
                }
                send(1, "oteryn:item/sword", [0, 0, 2, 0])?;
                let wait_for_reply = || async {
                    tokio::time::timeout(WAIT, async {
                        while link.cyclopedia_character_panel().pending {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                };
                wait_for_reply().await?;
                let items = link.cyclopedia_character_panel().items;
                assert!(items.page.is_none());
                assert!(items.location.is_none());
                assert!(items.search.is_empty());
                view.tick(&link).map_err(|error| format!("{error:?}"))?;
                if keep_open {
                    let refresh = tokio::time::timeout(WAIT, received.recv())
                        .await?
                        .ok_or("Store Inbox query")?;
                    assert_eq!((refresh.capability, refresh.kind), (199, 388));
                    let query = filtered::Query::decode(&refresh.payload)
                        .map_err(|error| format!("{error:?}"))?;
                    assert_eq!(
                        query,
                        filtered::Query {
                            offset: 0,
                            location: None,
                            search: String::new()
                        }
                    );
                    send(2, "oteryn:item/arrow", [0, 0, 0, 4])?;
                    wait_for_reply().await?;
                    assert_eq!(
                        link.cyclopedia_character_panel().items.rows()[0].quantities[3],
                        4
                    );
                } else {
                    assert!(!link.cyclopedia_character_panel().pending);
                    assert!(received.try_recv().is_err());
                }
                drop(link);
                tokio::time::timeout(WAIT, task).await??;
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "Store Inbox session panicked")?
}
