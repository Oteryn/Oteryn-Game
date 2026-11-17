use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::market::market_details as wire;
use oteryn_client_ui::market::details::{Filter, Query, Reply, Statistics, Ware};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer;
impl Stepper for Peer {
    fn market_selected(&self) -> bool {
        true
    }
    fn feature_selected(&self, id: u32) -> bool {
        id == 63
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, oteryn_session::SessionError> {
        assert_eq!(
            (command.capability, command.kind),
            (63, wire::COMMAND_TYPE_MARKET_DETAILS)
        );
        let query = wire::decode_query(&command.payload)
            .map_err(|_| oteryn_session::SessionError::SessionUnusable)?;
        let reply = match query {
            Query::Catalogue { page, .. } => Reply::Catalogue {
                items: vec![Ware {
                    definition: 42,
                    name: "Test blade".into(),
                    categories: vec!["Swords".into()],
                    vocations: Some(vec![2]),
                    level: Some(30),
                    forge_class: Some(2),
                    depot_units: 4,
                }],
                categories: vec!["Swords".into()],
                page,
                has_next: false,
            },
            Query::Book { definition, page } => {
                let mut offer = [7; 16];
                offer[6] = 0x70;
                offer[8] = 0x80;
                Reply::Book {
                    definition,
                    page,
                    rows: vec![1, 2]
                        .into_iter()
                        .map(|side| {
                            let mut offer = offer;
                            offer[0] = side;
                            oteryn_client_runtime::market::market::MarketRow {
                                offer,
                                definition,
                                side,
                                amount: 2,
                                price: 90 + u64::from(side) * 10,
                                state: 1,
                                anonymous: true,
                            }
                        })
                        .collect(),
                    statistics: Statistics {
                        buy: wire::PriceSummary {
                            trades: 1,
                            units: 2,
                            minimum: Some(100),
                            maximum: Some(100),
                            average: Some(100),
                        },
                        sell: wire::PriceSummary {
                            trades: 1,
                            units: 2,
                            minimum: Some(200),
                            maximum: Some(200),
                            average: Some(200),
                        },
                        definition,
                        as_of_us: 2592000000001,
                        since_us: 1,
                        trades: 2,
                        units: 4,
                        minimum: Some(100),
                        maximum: Some(200),
                        average: Some(150),
                    },
                    has_next: false,
                }
            }
            Query::Statistics { definition } => Reply::Statistics(Statistics {
                buy: wire::PriceSummary {
                    trades: 1,
                    units: 2,
                    minimum: Some(100),
                    maximum: Some(100),
                    average: Some(100),
                },
                sell: wire::PriceSummary {
                    trades: 1,
                    units: 2,
                    minimum: Some(200),
                    maximum: Some(200),
                    average: Some(200),
                },
                definition,
                as_of_us: 2592000000001,
                since_us: 1,
                trades: 2,
                units: 4,
                minimum: Some(100),
                maximum: Some(200),
                average: Some(150),
            }),
        };
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: wire::encode_reply(&reply)
                .map_err(|_| oteryn_session::SessionError::SessionUnusable)?,
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
        Vec::new()
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
fn t124_catalogue_and_completed_prices_reach_the_real_session_caller() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), Error> {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let observer = outbox.clone();
                let task = tokio::spawn(run_session(Peer, incoming, outbox));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while !link.market_panel().selected {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.market_query(Query::Catalogue {
                    filter: Filter {
                        search: "Test".into(),
                        ..Default::default()
                    },
                    page: 0
                }));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while link.market_panel().details.pending.is_some() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert_eq!(link.market_panel().details.items[0].name, "Test blade");
                assert!(link.market_details("42", "1", "9", true, false));
                assert!(link.market_query(Query::Book {
                    definition: 42,
                    page: 0
                }));
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while link.market_panel().details.pending.is_some() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(
                    oteryn_client_ui::market::panel_text(&link.market_panel())
                        .contains("average 150")
                );
                let text = oteryn_client_ui::market::panel_text(&link.market_panel());
                assert!(
                    text.contains(
                        "Buy offers: 1 trades · 2 units · min 100 · max 100 · average 100"
                    )
                );
                assert!(
                    text.contains(
                        "Sell offers: 1 trades · 2 units · min 200 · max 200 · average 200"
                    )
                );
                assert_eq!(link.market_panel().rows.len(), 2);
                observer.logged_out();
                assert!(link.market_panel().details.statistics.is_none());
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "market caller thread")??;
    Ok(())
}
