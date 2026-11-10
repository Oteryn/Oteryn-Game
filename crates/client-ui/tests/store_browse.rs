use oteryn_client_runtime::cui3::store::{self as wire, StoreCataloguePage, StoreOffer};
use oteryn_client_ui::store::{StorePanel, StoreSort, purchase, visible_offers};
#[test]
fn store_search_and_price_sort_preserve_the_confirmed_server_quote()
-> Result<(), Box<dyn std::error::Error>> {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut p = StorePanel {
        selected: true,
        search: "MOUNT".into(),
        sort: StoreSort::Price,
        purchase_confirmed: true,
        purchase_request_id: id,
        catalogue: Some(StoreCataloguePage {
            catalog_revision: "r1".into(),
            total_coins: 200,
            transferable_coins: 150,
            next_page_cursor: String::new(),
            offers: vec![
                StoreOffer {
                    offer_key: "mount-horse".into(),
                    title: "Mount horse".into(),
                    price: 90,
                    currency: "coins".into(),
                },
                StoreOffer {
                    offer_key: "outfit".into(),
                    title: "Outfit".into(),
                    price: 1,
                    currency: "coins".into(),
                },
                StoreOffer {
                    offer_key: "mount-tiger".into(),
                    title: "Mount tiger".into(),
                    price: 40,
                    currency: "coins".into(),
                },
            ],
        }),
        ..StorePanel::default()
    };
    assert_eq!(visible_offers(&p), vec![2, 0]);
    p.selected_offer = visible_offers(&p)[0];
    assert!(oteryn_client_ui::store::panel_text(&p).contains("Preview: Mount tiger — 40 coins"));
    let c = purchase(&mut p).ok_or("purchase")?;
    let quote = wire::decode_purchase_request(&c.payload).map_err(|_| "quote wire")?;
    assert_eq!(quote.offer_key, "mount-tiger");
    assert_eq!(quote.price, 40);
    assert_eq!(quote.catalog_revision, "r1");
    Ok(())
}
