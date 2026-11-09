use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = lfmt!("Houses\r\n{}", panel.feedback);
    if panel.page.rows.is_empty() {
        text.push_str(tr!(
            "\r\nNo houses on this page. Choose a town and Query to refresh."
        ));
    }
    for (index, row) in panel.page.rows.iter().enumerate() {
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
