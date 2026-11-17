#[test]
fn coin_history_real_caller_preserves_signed_ledger_kinds_filters_and_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    use super::feature_0098_quick_loot::peer;
    use oteryn_client_runtime::{cui3::store::*, server_mechanic::ServerMechanicCommand};
    peer::scenario(|link, push, wait| {
        wait(&|| link.store_panel().selected);
        let page = CoinHistoryPage {
            entries: vec![
                CoinHistoryEntry {
                    entry_id: 9,
                    kind: "COIN_PACKAGE_CREDIT".into(),
                    currency: "COINS".into(),
                    available_delta: 250,
                    reserved_delta: 0,
                    order_id: None,
                    offer_key: None,
                    quantity: None,
                    created_at: 100,
                },
                CoinHistoryEntry {
                    entry_id: 8,
                    kind: "AUCTION_RESERVE".into(),
                    currency: "COINS".into(),
                    available_delta: -50,
                    reserved_delta: 50,
                    order_id: None,
                    offer_key: None,
                    quantity: None,
                    created_at: 99,
                },
            ],
            next_page_cursor: "8".into(),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 154,
                command_type: 184,
                payload: encode_coin_history_request(&StoreHistoryRequest {
                    page_cursor: String::new(),
                })
                .map_err(|e| format!("{e:?}"))?,
            },
            payload: encode_coin_history_page(&page).map_err(|e| format!("{e:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.coin_history(false));
        wait(&|| link.store_panel().coin_history.page.is_some());
        let text = oteryn_client_ui::store::coin_history_text(&link.store_panel().coin_history);
        assert!(text.contains("+250"));
        assert!(text.contains("-50"));
        assert!(text.contains("AUCTION_RESERVE"));
        assert!(link.coin_history_filter("COIN_PACKAGE_CREDIT", "100", "100"));
        let text = oteryn_client_ui::store::coin_history_text(&link.store_panel().coin_history);
        assert!(text.contains("COIN_PACKAGE_CREDIT"));
        assert!(!text.contains("AUCTION_RESERVE"));
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 154,
                command_type: 184,
                payload: encode_coin_history_request(&StoreHistoryRequest {
                    page_cursor: "8".into(),
                })
                .map_err(|e| format!("{e:?}"))?,
            },
            payload: encode_coin_history_page(&CoinHistoryPage {
                entries: vec![],
                next_page_cursor: String::new(),
            })
            .map_err(|e| format!("{e:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.coin_history(true));
        wait(&|| {
            link.store_panel()
                .coin_history
                .page
                .as_ref()
                .is_some_and(|p| p.entries.is_empty())
        });
        assert!(!link.coin_history(true));
        Ok(())
    })
}
