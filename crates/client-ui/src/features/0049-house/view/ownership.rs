pub fn text(model: &super::super::sale::OwnershipPanel) -> String {
    let mut out = String::from("Choose an owned house or incoming transfer offer.\r\n");
    if let Some(row) = model.row() {
        use std::fmt::Write;
        let _ = write!(
            out,
            "\r\n{}\r\nOwner: {}\r\nRecipient: {}\r\nPrice: {} gold; rent: {} gold; total: {} gold\r\n",
            row.name,
            row.owner,
            if row.recipient.is_empty() {
                "Not offered"
            } else {
                &row.recipient
            },
            row.price,
            row.rent,
            row.price + row.rent
        );
        if row.due_at_unix_ms > 0 {
            let _ = write!(
                out,
                "Effective date: {}\r\n",
                super::super::ownership_date::format(row.due_at_unix_ms)
            );
        }
        out.push_str(match row.transfer_state {
            1 => "Transfer offered; no gold reserved yet.\r\n",
            2 => "Transfer accepted; price and rent are reserved.\r\n",
            _ => "",
        });
        if row.move_out_state != 0 {
            out.push_str("A move-out is scheduled or being processed.\r\n");
        }
    } else {
        out.push_str("\r\nNo house selected. Refresh to load your houses and offers.\r\n");
    }
    out.push_str("\r\nThe recipient needs a vocation and Premium eligibility. Active house bids or another house on this account and world prevent acceptance.\r\nMovable contents go to depot mail inboxes; bound items go to their owners. The buyer receives the emptied house.\r\n");
    out.push_str(&model.feedback);
    out
}
