use super::*;
#[test]
fn empty_server_store_page_clears_the_previously_selected_purchase_quote()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = StorePanel {
        selected: true,
        catalogue: Some(store::StoreCataloguePage {
            catalog_revision: "old".into(),
            offers: vec![],
            next_page_cursor: String::new(),
            total_coins: 1,
            transferable_coins: 0,
        }),
        ..Default::default()
    };
    assert!(browse(&mut panel, "featured", "no matches", 0, 0).is_some());
    let page = b::Page {
        categories: vec![b::Category {
            key: "featured".into(),
            parent: String::new(),
            title: "Home".into(),
        }],
        products: vec![],
        next: String::new(),
        total: 1,
        transferable: 0,
        transferable_known: false,
    };
    apply_browse(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: page.encode().map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(panel.catalogue.is_none());
    assert!(!panel.pending);
    assert!(!select_product(&mut panel, 0));
    Ok(())
}

#[test]
fn unresolved_purchase_freezes_every_catalogue_entry_point()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::store::{catalogue, categories, command, purchase, request, select_category};
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut panel = StorePanel {
        selected: true,
        purchase_confirmed: true,
        purchase_request_id: id,
        catalogue: Some(store::StoreCataloguePage {
            catalog_revision: "A".into(),
            offers: vec![store::StoreOffer {
                offer_key: "mount".into(),
                title: "Mount A".into(),
                price: 25,
                currency: "TIBIA_COIN".into(),
            }],
            next_page_cursor: String::new(),
            total_coins: 100,
            transferable_coins: 0,
        }),
        ..Default::default()
    };
    let first = purchase(&mut panel).ok_or("purchase")?;
    crate::store::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Rejected,
            payload: vec![],
        },
    );
    let quote = panel.catalogue.clone();
    assert!(browse(&mut panel, "featured", "", 0, 0).is_none());
    let category = categories().into_iter().next().ok_or("category")?;
    assert!(select_category(&mut panel, &category).is_none());
    panel.category = category;
    assert!(catalogue(&mut panel).is_none());
    for kind in [183, b::COMMAND_TYPE] {
        assert!(request(&mut panel, command(kind, vec![]).ok_or("command")?).is_none());
    }
    assert_eq!(panel.catalogue, quote);
    panel.purchase_confirmed = true;
    assert_eq!(purchase(&mut panel).ok_or("retry")?.payload, first.payload);
    Ok(())
}
