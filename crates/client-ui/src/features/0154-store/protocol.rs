use super::{StorePage, StorePanel};
use oteryn_client_runtime::cui3::store::{self, *};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};

pub fn command(command_type: u32, payload: Vec<u8>) -> Option<ServerMechanicCommand> {
    (183..=186)
        .contains(&command_type)
        .then_some(ServerMechanicCommand {
            capability: 154,
            command_type,
            payload,
        })
}
fn encoded(
    panel: &mut StorePanel,
    kind: u32,
    payload: Result<Vec<u8>, StoreWireError>,
) -> Option<ServerMechanicCommand> {
    let payload = payload.ok().or_else(|| {
        panel.feedback = Some("Enter valid store details".into());
        None
    })?;
    request(panel, command(kind, payload)?)
}
pub fn catalogue(panel: &mut StorePanel) -> Option<ServerMechanicCommand> {
    panel.page = StorePage::Catalogue;
    encoded(
        panel,
        COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        store::encode_catalogue_request(&StoreCatalogueRequest {
            category: panel.category.clone(),
            page_cursor: String::new(),
        }),
    )
}
pub fn history(panel: &mut StorePanel) -> Option<ServerMechanicCommand> {
    panel.page = StorePage::History;
    encoded(
        panel,
        COMMAND_TYPE_STORE_HISTORY_PAGE,
        store::encode_history_request(&StoreHistoryRequest {
            page_cursor: String::new(),
        }),
    )
}
pub fn purchase(panel: &mut StorePanel) -> Option<ServerMechanicCommand> {
    if !panel.purchase_confirmed {
        panel.feedback = Some("Confirm this purchase first".into());
        return None;
    }
    let offer = panel
        .catalogue
        .as_ref()
        .and_then(|page| page.offers.get(panel.selected_offer))
        .cloned()
        .or_else(|| {
            panel.feedback = Some("Select an offer from the server catalogue".into());
            None
        })?;
    let revision = panel.catalogue.as_ref()?.catalog_revision.clone();
    panel.purchase_confirmed = false;
    encoded(
        panel,
        COMMAND_TYPE_STORE_PURCHASE,
        store::encode_purchase_request(&StorePurchaseRequest {
            purchase_request_id: panel.purchase_request_id,
            offer_key: offer.offer_key,
            catalog_revision: revision,
            price: offer.price,
            currency: offer.currency,
        }),
    )
}
pub fn claim(panel: &mut StorePanel) -> Option<ServerMechanicCommand> {
    panel.page = StorePage::Inbox;
    encoded(
        panel,
        COMMAND_TYPE_STORE_INBOX_CLAIM,
        store::encode_inbox_claim_request(&StoreInboxClaimRequest {
            fulfilment_unit_id: panel.fulfilment_unit_id.clone(),
        }),
    )
}
pub fn request(
    panel: &mut StorePanel,
    command: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.pending_action = command.command_type;
    panel.feedback = None;
    Some(command)
}
pub fn apply_result(panel: &mut StorePanel, result: &ServerMechanicResponse) {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(format!("Server refused store action: {:?}", result.status));
        return;
    }
    let decoded = match panel.pending_action {
        COMMAND_TYPE_STORE_CATALOGUE_PAGE => {
            store::decode_catalogue_page(&result.payload).map(|v| panel.catalogue = Some(v))
        }
        COMMAND_TYPE_STORE_HISTORY_PAGE => {
            store::decode_history_page(&result.payload).map(|v| panel.history = Some(v))
        }
        COMMAND_TYPE_STORE_INBOX_CLAIM => {
            store::decode_claim_receipt(&result.payload).map(|v| panel.claim = Some(v))
        }
        _ => store::decode_purchase_result(&result.payload).map(|v| panel.purchase = Some(v)),
    };
    panel.feedback = decoded
        .err()
        .map(|_| "The server returned invalid store data".into());
}
#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = StorePanel {
            selected: true,
            category: "featured".into(),
            ..Default::default()
        };
        let c = catalogue(&mut p).unwrap();
        assert_eq!(
            store::decode_catalogue_request(&c.payload)
                .unwrap()
                .category,
            "featured"
        );
        let wire = store::encode_catalogue_page(&StoreCataloguePage {
            catalog_revision: "rev-1".into(),
            offers: vec![StoreOffer {
                offer_key: "mount".into(),
                title: "Mount".into(),
                price: 25,
                currency: "oteryn-coins".into(),
            }],
            next_page_cursor: String::new(),
            total_coins: 50,
            transferable_coins: 10,
        })
        .unwrap();
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: wire,
            },
        );
        assert_eq!(p.catalogue.as_ref().unwrap().total_coins, 50);
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 2,
                status: CommandStatus::Rejected,
                server_sequence: 2,
                payload: vec![],
            },
        );
        assert!(p.feedback.unwrap().contains("refused"));
    }
}
