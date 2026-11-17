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
