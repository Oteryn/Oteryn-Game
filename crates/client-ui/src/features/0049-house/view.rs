use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    let mut text = format!("Houses\r\n{}", panel.feedback);
    for row in &panel.page.rows {
        text.push_str(&format!(
            "\r\n{}: rent {}, price {}, ends {:?}, maximum {:?}, notice {:?}",
            row.house, row.rent, row.price, row.ends, row.maximum, row.notice
        ));
    }
    text
}
