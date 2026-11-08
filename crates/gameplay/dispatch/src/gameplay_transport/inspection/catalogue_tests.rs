use super::*;
use oteryn_protocol_oteryn::inspection::catalogue::Query;
#[test]
fn catalogue_pages_advance_over_sparse_records_and_filter_public_templates()
-> Result<(), Box<dyn std::error::Error>> {
    let query = Query::new(0, "silver", "sword").map_err(|_| "query")?;
    let mut scanned = 0;
    let page = catalogue::project_page(query, |reference| {
        scanned += 1;
        Some((reference % 3 == 0).then(|| InspectedItem {
            slot: 0,
            definition_ref: reference,
            count: 1,
            tier: 0,
            name: "Silver sword".into(),
            attributes: vec![("Weapon Type".into(), "sword, two-handed".into())],
        }))
    });
    assert_eq!(page.items.len(), 10);
    assert_eq!(scanned, 30);
    assert_eq!(page.catalogue.as_ref().and_then(|page| page.next), Some(30));
    let sparse = catalogue::project_page(query, |_| Some(None));
    assert!(sparse.items.is_empty());
    assert!(sparse.catalogue.as_ref().is_some_and(|page| {
        page.categories
            .iter()
            .any(|category| category == "Spellbook")
    }));
    assert_eq!(sparse.catalogue.and_then(|page| page.next), Some(256));
    let end = catalogue::project_page(Query::new(u32::MAX, "", "").map_err(|_| "query")?, |_| None);
    assert_eq!(end.catalogue.and_then(|page| page.next), None);
    Ok(())
}
