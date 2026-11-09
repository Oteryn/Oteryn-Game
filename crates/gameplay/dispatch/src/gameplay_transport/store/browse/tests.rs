use super::*;
#[test]
fn store_browser_filters_sorts_before_paging_and_binds_cursor_to_query()
-> Result<(), Box<dyn std::error::Error>> {
    let products: Vec<_> = (0u32..70)
        .map(|n| Product {
            category: "consumables.exercise-weapons".into(),
            revision: "catalogue-1".into(),
            offer: wire::StoreOffer {
                offer_key: format!("store:item-{n:03}"),
                title: format!("Sword {n:03}"),
                price: 100 - u64::from(n),
                currency: "COINS".into(),
            },
            artwork: Artwork {
                kind: 1,
                looks: [3031; 2],
            },
        })
        .collect();
    let q = Query {
        category: "consumables".into(),
        search: "Sword".into(),
        sort: 3,
        cursor: String::new(),
    };
    let first = project(products.clone(), &q, &BTreeMap::new()).ok_or("first")?;
    assert_eq!(first.products.len(), 32);
    assert_eq!(first.products[0].offer.price, 31);
    assert!(
        first
            .categories
            .iter()
            .any(|c| c.key == "consumables.exercise-weapons" && c.parent == "consumables")
    );
    let next = Query {
        cursor: first.next.clone(),
        ..q.clone()
    };
    let second = project(products.clone(), &next, &BTreeMap::new()).ok_or("second")?;
    assert_eq!(second.products[0].offer.price, 63);
    assert!(
        project(
            products.clone(),
            &Query {
                search: "Axe".into(),
                ..next
            },
            &BTreeMap::new()
        )
        .is_none()
    );
    let ranks = BTreeMap::from([("store:item-000".into(), 2)]);
    let popular = project(products, &Query { sort: 0, ..q }, &ranks).ok_or("popular")?;
    assert_eq!(popular.products[0].offer.offer_key, "store:item-000");
    assert_eq!(popular.products[0].revision, "catalogue-1");
    Ok(())
}
#[test]
fn store_browser_resolves_every_admitted_item_from_active_content()
-> Result<(), Box<dyn std::error::Error>> {
    let mut world_id = [0x42; 16];
    world_id[6] = 0x70;
    world_id[8] = 0x80;
    let world = crate::foundation::WorldId::decode(&world_id).map_err(|e| format!("{e:?}"))?;
    let (controller, ..) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active")?;
    let native = active.native_gameplay().ok_or("native")?;
    let catalogue = CATALOGUE.as_ref().ok_or("catalogue")?;
    for offer in catalogue.entries() {
        let art = artwork(native, catalogue, offer)
            .ok_or_else(|| format!("unresolved artwork {}", offer.key))?;
        if matches!(offer.delivery, rules::DeliveryKind::Item { .. }) {
            assert_eq!(art.kind, 1, "{}", offer.key);
        }
    }
    Ok(())
}
