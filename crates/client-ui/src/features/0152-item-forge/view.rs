pub fn panel_text(p: &super::ItemForgePanel) -> String {
    let mut s = format!(
        "Item Forge\nAction: {}\nItem: {} / {}",
        if p.query_current_dust {
            "Current dust".to_owned()
        } else {
            format!("{:?}", p.operation)
        },
        p.first_item,
        p.second_item
    );
    if let Some(reply) = &p.decoded {
        match reply {
            super::reply::Reply::CurrentDust { dust, limit } => {
                s.push_str(&format!("\nCurrent dust: {dust} / {limit}"));
            }
            super::reply::Reply::Result(receipt) => {
                s.push('\n');
                s.push_str(&super::reply::render(receipt));
            }
            super::reply::Reply::History(rows) => {
                if rows.is_empty() {
                    s.push_str("\nNo forge operations yet");
                }
                for (ordinal, receipt) in rows {
                    s.push_str(&format!(
                        "\nHistory #{ordinal}\n{}",
                        super::reply::render(receipt)
                    ));
                }
            }
        }
    }
    if let Some(f) = &p.feedback {
        s.push('\n');
        s.push_str(f)
    }
    s
}
