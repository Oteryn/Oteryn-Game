/// Readable item identity; the numeric handle stays an internal wire detail.
fn item_label(p: &super::ItemForgePanel, handle: u64) -> String {
    match p.preview.item(handle) {
        Some(item) => format!(
            "{} — class {}, tier {} / {}",
            item.name, item.classification, item.tier, item.maximum_tier
        ),
        None if handle != 0 && p.preview.live.iter().any(|e| e.handle.get() == handle) => {
            "Carried item — name unavailable on this server".into()
        }
        None => "No current item selected".into(),
    }
}

pub fn panel_text(p: &super::ItemForgePanel) -> String {
    let mut s = lfmt!(
        "Item Forge\nAction: {}\nItem: {} / {}",
        if p.query_current_dust {
            tr!("Current dust").to_owned()
        } else {
            format!("{:?}", p.operation)
        },
        item_label(p, p.first_item),
        item_label(p, p.second_item)
    );
    if let Some(reply) = &p.decoded {
        match reply {
            super::reply::Reply::CurrentDust { dust, limit } => {
                s.push_str(&lfmt!(
                    "\nCurrent dust: {dust} / {limit}",
                    dust = dust,
                    limit = limit
                ));
            }
            super::reply::Reply::Result(receipt) => {
                s.push('\n');
                s.push_str(&super::reply::render(receipt));
            }
            super::reply::Reply::History(rows) => {
                if rows.is_empty() {
                    s.push_str(tr!("\nNo forge operations yet"));
                }
                for (ordinal, receipt) in rows {
                    s.push_str(&lfmt!(
                        "\nHistory #{ordinal}\n{}",
                        super::reply::render(receipt),
                        ordinal = ordinal
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

pub fn selected_pair_text(p: &super::ItemForgePanel) -> String {
    let item = |handle| item_label(p, handle);
    let mut text = format!(
        "Receiver: {}\nSource / tier donor: {}\n",
        item(p.first_item),
        item(p.second_item)
    );
    if let Some(quote) = &p.preview.quote {
        text.push_str(&format!("Server-verified pair: target tier {}\nBase cost before outcome bonuses: {} gold, {} dust, {} cores\nSuccess: {}%; tier-loss protection: {}%\nAvailable: {} carried gold; {} bank gold{}; {} / {} dust; {} cores; {} slivers\nThe original server writer independently rechecks the current state.",quote.target_tier,quote.cost.gold,quote.cost.dust,quote.cost.cores,quote.success_percent,quote.protection_percent,quote.resources.carried_gold,quote.resources.bank_gold,if quote.resources.bank_spendable{""}else{" (not spendable by this character)"},quote.resources.dust,quote.resources.dust_limit,quote.resources.cores,quote.resources.slivers));
    } else if !p.preview.selected {
        text.push_str("This server has no Forge preview capability. Legacy submission is available; the server evaluates its cost and eligibility.\n");
    } else {
        text.push_str("Choose two current carried items and request a server quote.\n");
    }
    if p.retry.is_some() {
        text.push_str("\nAn attempt needs confirmation. Retry sends its exact original occurrence and operands; new paid attempts are disabled.");
    }
    if let Some(message) = &p.preview.feedback {
        text.push('\n');
        text.push_str(message);
    }
    text
}
