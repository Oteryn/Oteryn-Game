use super::DepotPanel;
pub fn panel_text(p: &DepotPanel) -> String {
    let location = p.reply.as_ref().map_or_else(
        || tr!("No box loaded").into(),
        |r| {
            lfmt!(
                "Town {}, box {}, page {}, {} items",
                r.town
                    .map_or_else(|| tr!("unavailable").into(), |town| town.to_string()),
                r.box_number,
                r.page + 1,
                r.total
            )
        },
    );
    lfmt!(
        "{location}\r\n{}\r\nSelect a depot cell to withdraw to the main backpack. Select a carried cell to deposit the whole stack.",
        p.feedback,
        location = location
    )
}
