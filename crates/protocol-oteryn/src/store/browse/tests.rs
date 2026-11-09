use super::*;
#[test]
fn store_browse_round_trip_rejects_bad_sort_and_truncated_pages()
-> Result<(), Box<dyn std::error::Error>> {
    let q = Query {
        category: "featured".into(),
        search: "sword".into(),
        sort: 3,
        cursor: String::new(),
    };
    assert_eq!(
        Query::decode(&q.encode().map_err(|e| format!("{e:?}"))?).map_err(|e| format!("{e:?}"))?,
        q
    );
    assert!(Query { sort: 9, ..q }.encode().is_err());
    let p = Page {
        categories: vec![Category {
            key: "featured".into(),
            parent: String::new(),
            title: "Home".into(),
        }],
        products: vec![],
        next: String::new(),
        total: 0,
        transferable: 0,
        transferable_known: false,
    };
    let b = p.encode().map_err(|e| format!("{e:?}"))?;
    assert_eq!(Page::decode(&b).map_err(|e| format!("{e:?}"))?, p);
    assert!(Page::decode(&b[..b.len() - 1]).is_err());
    Ok(())
}
