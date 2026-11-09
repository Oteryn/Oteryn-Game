use super::{StorePage, StorePanel};
use oteryn_client_runtime::cui3::store::{self, *};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};

pub fn command(command_type: u32, payload: Vec<u8>) -> Option<ServerMechanicCommand> {
    ((183..=186).contains(&command_type) || command_type == store::browse::COMMAND_TYPE).then_some(
        ServerMechanicCommand {
            capability: 154,
            command_type,
            payload,
        },
    )
}
fn encoded(
    panel: &mut StorePanel,
    kind: u32,
    payload: Result<Vec<u8>, StoreWireError>,
) -> Option<ServerMechanicCommand> {
    let payload = payload.ok().or_else(|| {
        panel.feedback = Some(tr!("Enter valid store details").into());
        None
    })?;
    request(panel, command(kind, payload)?)
}
// CLIGAP-W09 accepted transport difference: follow Game's opaque catalogue cursor,
// reset it on category changes; Crystal sends category offers without this cursor.
pub fn catalogue(panel: &mut StorePanel) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.page = StorePage::Catalogue;
    if panel.category != panel.catalogue_category {
        panel.catalogue = None;
        panel.selected_offer = 0;
        panel.catalogue_category = panel.category.clone();
    }
    let cursor = panel
        .catalogue
        .as_ref()
        .map_or_else(String::new, |p| p.next_page_cursor.clone());
    encoded(
        panel,
        COMMAND_TYPE_STORE_CATALOGUE_PAGE,
        store::encode_catalogue_request(&StoreCatalogueRequest {
            category: panel.category.clone(),
            page_cursor: cursor,
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
    if !panel.selected || panel.pending {
        return None;
    }
    if !panel.purchase_confirmed {
        panel.feedback = Some(tr!("Confirm this purchase first").into());
        return None;
    }
    if let Some(payload) = panel.confirmed_purchase.clone() {
        panel.purchase_confirmed = false;
        return encoded(panel, COMMAND_TYPE_STORE_PURCHASE, Ok(payload));
    }
    let offer = panel
        .catalogue
        .as_ref()
        .and_then(|page| page.offers.get(panel.selected_offer))
        .cloned()
        .or_else(|| {
            panel.feedback = Some(tr!("Select an offer from the server catalogue").into());
            None
        })?;
    let revision = panel.catalogue.as_ref()?.catalog_revision.clone();
    panel.purchase_confirmed = false;
    // Crystal 201189f, data/modules/scripts/gamestore/gamestore.lua:6302-6332:
    // hire / rename / sex-change identities. init.lua:1877-1900,1951-1972,1990-2020:
    // hire takes name+sex, rename takes target+name, sex-change toggles target.
    // CLIGAP-W09 uses the existing Game UUID customization instead of donor modals;
    // capability-250 negotiation belongs to W00.
    // Prices, currency and revision always come from the selected server row.
    let customization = match offer.offer_key.as_str() {
        "store:hireling:25440" => Some(StoreHirelingCustomization {
            hireling_id: None,
            name: Some(panel.hireling_name.clone()),
            sex: Some(panel.hireling_sex),
        }),
        "store:hireling:25438" | "store:hireling:25437" => {
            let id = panel.hireling_id.or_else(|| {
                panel.feedback = Some(tr!("Enter the Hireling identity").into());
                None
            })?;
            Some(StoreHirelingCustomization {
                hireling_id: Some(id),
                name: (offer.offer_key == "store:hireling:25438")
                    .then(|| panel.hireling_name.clone()),
                sex: None,
            })
        }
        _ => None,
    };
    let quote = StorePurchaseRequest {
        purchase_request_id: panel.purchase_request_id,
        offer_key: offer.offer_key,
        catalog_revision: revision,
        price: offer.price,
        currency: offer.currency,
    };
    let payload = match customization {
        Some(c) => store::encode_hireling_purchase_request(&quote, &c),
        None => store::encode_purchase_request(&quote),
    };
    let command = encoded(panel, COMMAND_TYPE_STORE_PURCHASE, payload)?;
    panel.confirmed_purchase = Some(command.payload.clone());
    Some(command)
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
    if panel.pending_action == store::browse::COMMAND_TYPE {
        super::browser::apply_browse(panel, result);
        return;
    }

    panel.pending = false;
    let coin_history = std::mem::take(&mut panel.coin_history.pending);
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some(lfmt!("Server refused store action: {:?}", result.status));
        return;
    }
    let decoded = match panel.pending_action {
        COMMAND_TYPE_STORE_CATALOGUE_PAGE => {
            store::decode_catalogue_page(&result.payload).map(|v| {
                panel.selected_offer = 0;
                panel.catalogue = Some(v);
            })
        }
        COMMAND_TYPE_STORE_HISTORY_PAGE if coin_history => {
            store::decode_coin_history_page(&result.payload)
                .map(|page| panel.coin_history.page = Some(page))
        }
        COMMAND_TYPE_STORE_HISTORY_PAGE => {
            store::decode_history_page(&result.payload).map(|v| panel.history = Some(v))
        }
        COMMAND_TYPE_STORE_INBOX_CLAIM => {
            store::decode_claim_receipt(&result.payload).map(|v| panel.claim = Some(v))
        }
        _ => store::decode_purchase_result(&result.payload).map(|v| {
            panel.confirmed_purchase = None;
            panel.purchase = Some(v);
        }),
    };
    panel.feedback = decoded
        .err()
        .map(|_| tr!("The server returned invalid store data").into());
}

pub fn browse_request(
    panel: &mut StorePanel,
    category: &str,
    search: &str,
    sort: u8,
    step: i8,
) -> Option<ServerMechanicCommand> {
    super::browser::browse(panel, category, search, sort, step)
}

#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn gap_store_hireling_fields_follow_offer_kind() {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        for (key, expected) in [
            (
                "store:hireling:25440",
                Some((None, Some("Alex".into()), Some(2))),
            ),
            (
                "store:hireling:25438",
                Some((Some(id), Some("Alex".into()), None)),
            ),
            ("store:hireling:25437", Some((Some(id), None, None))),
            ("store:hireling:1001", None),
            ("mount", None),
        ] {
            let mut p = StorePanel {
                selected: true,
                purchase_confirmed: true,
                purchase_request_id: id,
                hireling_name: "Alex".into(),
                hireling_sex: 2,
                hireling_id: Some(id),
                catalogue: Some(StoreCataloguePage {
                    catalog_revision: "server-revision".into(),
                    offers: vec![StoreOffer {
                        offer_key: key.into(),
                        title: "Server title".into(),
                        price: 73,
                        currency: "coins".into(),
                    }],
                    next_page_cursor: String::new(),
                    total_coins: 100,
                    transferable_coins: 0,
                }),
                ..Default::default()
            };
            let c = purchase(&mut p).unwrap();
            let (quote, custom) = store::decode_purchase_request_with_hireling(&c.payload).unwrap();
            assert_eq!(
                (
                    quote.offer_key.as_str(),
                    quote.catalog_revision.as_str(),
                    quote.price,
                    quote.currency.as_str()
                ),
                (key, "server-revision", 73, "coins")
            );
            assert_eq!(custom.map(|c| (c.hireling_id, c.name, c.sex)), expected);
            p.pending = false;
            p.purchase_confirmed = true;
            p.hireling_name = "Changed".into();
            p.catalogue = None;
            assert_eq!(purchase(&mut p).unwrap().payload, c.payload);
        }
        let mut p = StorePanel {
            selected: true,
            purchase_confirmed: true,
            purchase_request_id: id,
            catalogue: Some(StoreCataloguePage {
                catalog_revision: "rev".into(),
                offers: vec![StoreOffer {
                    offer_key: "store:hireling:25438".into(),
                    title: "Rename".into(),
                    price: 25,
                    currency: "coins".into(),
                }],
                next_page_cursor: String::new(),
                total_coins: 0,
                transferable_coins: 0,
            }),
            ..Default::default()
        };
        assert!(purchase(&mut p).is_none());
        assert!(!p.pending);
        assert!(p.confirmed_purchase.is_none());
    }

    #[test]
    fn gap_store_catalogue_follows_cursor_and_resets_category() {
        let mut p = StorePanel {
            selected: true,
            category: "featured".into(),
            ..Default::default()
        };
        catalogue(&mut p).unwrap();
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: store::encode_catalogue_page(&StoreCataloguePage {
                    catalog_revision: "rev-1".into(),
                    offers: vec![],
                    next_page_cursor: "server-cursor".into(),
                    total_coins: 0,
                    transferable_coins: 0,
                })
                .unwrap(),
            },
        );
        let next = catalogue(&mut p).unwrap();
        assert_eq!(
            store::decode_catalogue_request(&next.payload)
                .unwrap()
                .page_cursor,
            "server-cursor"
        );
        p.pending = false;
        p.category = "houses.hirelings".into();
        let changed = catalogue(&mut p).unwrap();
        assert!(
            store::decode_catalogue_request(&changed.payload)
                .unwrap()
                .page_cursor
                .is_empty()
        );
        assert!(p.catalogue.is_none());
    }

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
