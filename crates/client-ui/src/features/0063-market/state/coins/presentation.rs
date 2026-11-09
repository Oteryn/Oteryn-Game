use serde_json::Value;
fn kind(v: &str) -> &str {
    match v {
        "reserve" => "Coin reservation",
        "reserve_take" => "Sell into buy offer",
        "settle" | "settlement" => "Trade settlement",
        "release" => "Reservation returned",
        "buy" => "Buy offer",
        "cancel_buy" => "Buy offer cancelled",
        "transfer" => "Coin gift",
        _ => v,
    }
}
fn number(v: &Value) -> String {
    v.as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string())
}
pub(super) fn targets(value: &Value) -> Vec<String> {
    let mut rows = Vec::new();
    for (array, key) in [
        ("offers", "offer"),
        ("recent_operations", "operation"),
        ("history", "operation"),
    ] {
        for row in value[array].as_array().into_iter().flatten() {
            if let Some(v) = row[key].as_str() {
                rows.push(v.into());
            }
        }
    }
    rows
}
pub(super) fn render(v: &Value) -> String {
    let mut text = String::new();
    let mut index = 0;
    if let Some(rows) = v["offers"].as_array() {
        text.push_str("Offers (enter a row number to accept or cancel):\r\n");
        for row in rows {
            index += 1;
            let state = match row["state"].as_i64() {
                Some(0) => "pending reservation",
                Some(1) => "open",
                Some(2) => "closing",
                Some(3) => "cancelled / expired",
                Some(4) => "filled",
                _ => "unknown",
            };
            text.push_str(&format!(
                "{index}. {} {} coins at {} gold each — {} — {}\r\n",
                row["side"].as_str().unwrap_or(""),
                number(&row["remaining"]),
                number(&row["price"]),
                row["name"].as_str().unwrap_or(""),
                state
            ));
        }
        if rows.is_empty() {
            text.push_str("No offers on this page.\r\n");
        }
    }
    if let Some(rows) = v["recent_operations"].as_array() {
        if !rows.is_empty() {
            text.push_str("Recent operations (select a row for details):\r\n");
        }
        for row in rows {
            index += 1;
            text.push_str(&format!(
                "{index}. {} — {} coins — {} {}\r\n",
                kind(row["kind"].as_str().unwrap_or("")),
                number(&row["amount"]),
                if row["pending"] == true {
                    "pending"
                } else {
                    "complete"
                },
                row["error"].as_str().unwrap_or("")
            ));
        }
    }
    if let Some(rows) = v["history"].as_array() {
        text.push_str("Coin transactions (select a row for details):\r\n");
        for row in rows {
            index += 1;
            text.push_str(&format!(
                "{index}. {} — {} — {} — available {} / reserved {}\r\n",
                row["occurred_at"].as_str().unwrap_or(""),
                kind(row["kind"].as_str().unwrap_or("")),
                row["coin_class"].as_str().unwrap_or(""),
                number(&row["available_delta"]),
                number(&row["reserved_delta"])
            ));
        }
        if rows.is_empty() {
            text.push_str("No older transactions.\r\n");
        }
    }
    if v.get("transferable").is_some() {
        text.push_str(&format!(
            "Transferable: {}\r\nNon-transferable: {}\r\nReserved: {}\r\n",
            number(&v["transferable"]),
            number(&v["non_transferable"]),
            number(&v["reserved"])
        ));
    }
    if v.get("operation").is_some() {
        text.push_str(&format!(
            "{} — {} coins, {} gold each\r\nStatus: {}\r\n",
            kind(v["kind"].as_str().unwrap_or("operation")),
            number(&v["amount"]),
            number(&v["price"]),
            if v["pending"] == true {
                "pending reconciliation"
            } else {
                "complete"
            }
        ));
    }
    let receipt = if v["receipt"]["receipt"].is_object() {
        &v["receipt"]["receipt"]
    } else {
        &v["receipt"]
    };
    if receipt.is_object() {
        text.push_str(&format!(
            "Receipt: {}\r\n{} — {} coins — {}\r\n",
            receipt["operation_id"].as_str().unwrap_or(""),
            kind(receipt["kind"].as_str().unwrap_or("")),
            number(&receipt["amount"]),
            receipt["occurred_at"].as_str().unwrap_or("")
        ));
        if receipt.get("transferable_after").is_some() {
            text.push_str(&format!(
                "Transferable after: {}\r\nNon-transferable after: {}\r\nReserved after: {}\r\n",
                number(&receipt["transferable_after"]),
                number(&receipt["non_transferable_after"]),
                number(&receipt["reserved_after"])
            ));
        }
    }
    text
}
