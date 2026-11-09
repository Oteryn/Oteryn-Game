use crate::play::{mailbox::lock, play_channel};
use oteryn_client_ui::chat::{ConsoleFilters, ConsoleTab, panel_text};
use oteryn_session::{AppliedDelta, ChatLine, ChatRoomSet, ServerMessage, SessionEvent};

#[test]
fn console_filters_route_real_notices_store_results_and_private_tabs()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.push(vec![
        SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ServerMessage::Advance("The ridge event has started.".into()),
        }),
        SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: ServerMessage::StatusText("You are in a safe area.".into()),
        }),
    ]);
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Bryn".into(),
            text: "Meet at the bridge.".into(),
        }],
    );
    outbox.local_chat_result(&oteryn_client_runtime::chat_public_channels::ChatOutcome {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        disposition: oteryn_client_runtime::chat_public_channels::ChatDisposition::Muted,
        wait_seconds: 5,
        result_server_sequence: 3,
    });
    outbox.store_initialize(true);
    let request = oteryn_client_runtime::cui3::store::StorePurchaseRequest {
        purchase_request_id: [1, 144, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1],
        offer_key: "mount".into(),
        catalog_revision: "rev-7".into(),
        price: 25,
        currency: "coins".into(),
    };
    link.store_request(
        185,
        oteryn_client_runtime::cui3::store::encode_purchase_request(&request)
            .map_err(|e| format!("{e:?}"))?,
    );
    outbox.store_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 4,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_client_runtime::cui3::store::encode_purchase_result(
                &oteryn_client_runtime::cui3::store::StorePurchaseResult {
                    code: oteryn_client_runtime::cui3::store::StorePurchaseCode::OfferUnavailable,
                    order_id: None,
                    operation_id: None,
                },
            )
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    let terms = [
        "ridge event",
        "safe area",
        "Wait 5 seconds",
        "OfferUnavailable",
        "Meet at the bridge",
    ];
    let original = panel_text(&link.chat_panel());
    for term in terms {
        assert!(original.contains(term), "missing {term}: {original}");
    }
    for index in 0..5 {
        let mut filters = ConsoleFilters::default();
        match index {
            0 => filters.events = false,
            1 => filters.status = false,
            2 => filters.information = false,
            3 => filters.store = false,
            _ => filters.private_messages = false,
        };
        lock(&link.mailbox).chat.console.filters = filters;
        let text = panel_text(&link.chat_panel());
        for (other, term) in terms.iter().enumerate() {
            assert_eq!(
                text.contains(term),
                other != index,
                "filter {index}, term {term}"
            );
        }
    }
    lock(&link.mailbox).chat.console.filters = ConsoleFilters {
        private_tab: true,
        ..Default::default()
    };
    assert!(!panel_text(&link.chat_panel()).contains("Meet at the bridge"));
    lock(&link.mailbox).chat.console.select(ConsoleTab::Private);
    assert!(panel_text(&link.chat_panel()).contains("Meet at the bridge"));
    lock(&link.mailbox).chat.console.select(ConsoleTab::All);
    lock(&link.mailbox).chat.console.filters = ConsoleFilters::default();
    assert_eq!(panel_text(&link.chat_panel()), original);
    assert_eq!(link.chat_panel().lines.len(), 1);
    use oteryn_client_runtime::cui3::store::{
        StoreCataloguePage, StoreCatalogueRequest, encode_catalogue_page, encode_catalogue_request,
    };
    link.store_request(
        183,
        encode_catalogue_request(&StoreCatalogueRequest {
            category: "cosmetics".into(),
            page_cursor: String::new(),
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    outbox.store_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 3,
            server_sequence: 5,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_catalogue_page(&StoreCataloguePage {
                catalog_revision: "rev-8".into(),
                offers: Vec::new(),
                next_page_cursor: String::new(),
                total_coins: 100,
                transferable_coins: 0,
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    let refreshed = panel_text(&link.chat_panel());
    assert!(refreshed.contains("Store catalogue updated."));
    assert_eq!(refreshed.matches("OfferUnavailable").count(), 1);

    Ok(())
}
