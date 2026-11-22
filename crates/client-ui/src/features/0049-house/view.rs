use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    browser_text(panel, None, None, None)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HouseOrder {
    Name,
    Area,
}

pub fn matches_filters(
    row: &oteryn_client_runtime::house::Row,
    guildhalls: Option<bool>,
    rented: Option<bool>,
) -> bool {
    guildhalls.is_none_or(|wanted| row.kind == if wanted { 3 } else { 1 })
        && rented.is_none_or(|wanted| {
            if wanted {
                (3..=5).contains(&row.state)
            } else {
                row.state == 2
            }
        })
}

pub fn browser_text(
    panel: &Panel,
    guildhalls: Option<bool>,
    rented: Option<bool>,
    order: Option<HouseOrder>,
) -> String {
    let mut rows: Vec<_> = panel
        .page
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| matches_filters(row, guildhalls, rented))
        .collect();
    match order {
        Some(HouseOrder::Name) => {
            rows.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name).then_with(|| a.house.cmp(&b.house)))
        }
        Some(HouseOrder::Area) => rows.sort_by_key(|(_, row)| row.size),
        None => return render(panel, rows),
    }
    render(panel, rows)
}
fn render(panel: &Panel, rows: Vec<(usize, &oteryn_client_runtime::house::Row)>) -> String {
    let mut text = lfmt!("Houses\r\n{}", panel.feedback);
    if rows.is_empty() {
        text.push_str(tr!(
            "\r\nNo houses on this page. Choose a town and Query to refresh."
        ));
    }
    for (index, row) in rows {
        let kind = match row.kind {
            2 => "shop",
            3 => "guildhall",
            _ => "house",
        };
        text.push_str(&format!(
            "\r\n{}. {} [{kind}; {}]",
            index + 1,
            row.house,
            if row.owned {
                tr!("owned by you")
            } else {
                tr!("not owned by you")
            }
        ));
        if let Some((x, y, floor)) = row.entrance {
            text.push_str(&format!(
                "\r\n{} — {} m², {} beds; entrance {x},{y},{floor}",
                row.name, row.size, row.beds
            ));
        }
        if let Some(ends) = row.ends {
            text.push_str(&lfmt!(
                "\r\nAuction ends (Unix ms): {ends}; your maximum bid: {}",
                row.maximum
                    .map_or_else(|| tr!("none").into(), |bid| bid.to_string()),
                ends = ends
            ));
        }
        text.push_str(&lfmt!(
            "\r\n{}: rent {}, price {}, ends {:?}, maximum {:?}, notice {:?}",
            row.house,
            row.rent,
            row.price,
            row.ends,
            row.maximum,
            row.notice
        ));
    }
    text.push_str(if panel.page.cursor.is_some() {
        tr!("\r\nNext page available")
    } else {
        tr!("\r\nEnd of catalogue")
    });
    text
}

#[path = "view/ownership.rs"]
mod ownership;
pub fn ownership_text(panel: &super::sale::OwnershipPanel) -> String {
    ownership::text(panel)
}

#[cfg(test)]
#[test]
fn house_and_auction_filters_exclude_shops_and_vacancies() {
    use oteryn_client_runtime::house::Row;
    let shop = Row {
        kind: 2,
        state: 2,
        ..Default::default()
    };
    let vacant = Row {
        kind: 1,
        state: 1,
        ..Default::default()
    };
    assert!(!matches_filters(&shop, Some(false), None));
    assert!(!matches_filters(&vacant, None, Some(false)));
    let mut panel = Panel::default();
    let house = Row {
        name: "House".into(),
        entrance: Some((10, 10, 7)),
        kind: 1,
        state: 2,
        ..Default::default()
    };
    panel.page.rows = vec![
        house,
        Row {
            entrance: Some((30, 30, 7)),
            ..shop
        },
        Row {
            entrance: Some((90, 90, 7)),
            ..vacant
        },
    ];
    let markers = super::filtered_map_houses(&panel, Some(false), Some(false));
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].label, "1: House");
}
