fn entries(panel: &TaskBoardPanel, tab: Tab, confirm: bool) -> Vec<Entry> {
    if matches!(tab, Tab::Preferences | Tab::Soulseals) { return extras::entries(panel, tab); }
    let mut rows = Vec::new();
    let Some(r) = &panel.weekly else {
        entry(&mut rows, "Refresh to load the Task Board", None);
        return rows;
    };
    match tab {
        Tab::Bounty => {
            let b = &r.bounty;
            entry(
                &mut rows,
                format!("Reroll Tokens: {} / {}", b.rerolls, model::REROLL_TOKEN_CAP),
                None,
            );
            entry(
                &mut rows,
                "Daily tokens: claim at the Daily Reward Wall",
                None,
            );
            if unopened(b) {
                entry(&mut rows, "Open bounty", Some(Kind::OpenBounty));
            }
            for (index, offer) in b.offers.iter().enumerate() {
                let active = usize::from(b.selected) == index + 1;
                let progress = if active { b.current } else { 0 };
                let state = if active && completed(panel) {
                    "Completed"
                } else if active {
                    "Active"
                } else {
                    "Available"
                };
                entry(
                    &mut rows,
                    format!(
                        "{}: {}/{} | {} | {}",
                        model::creature_name(&offer.race),
                        progress,
                        offer.required,
                        model::rarity_name(offer.grade),
                        state
                    ),
                    if b.selected == 0 {
                        u8::try_from(index).ok().map(Kind::Select)
                    } else {
                        None
                    },
                );
            }
            if let Some(active) = &b.active
                && b.offers.get(usize::from(b.selected.saturating_sub(1))) != Some(active)
            {
                entry(
                    &mut rows,
                    format!(
                        "Active: {}: {}/{} | {} | {}",
                        model::creature_name(&active.race),
                        b.current,
                        active.required,
                        model::rarity_name(active.grade),
                        if completed(panel) {
                            "Completed"
                        } else {
                            "Active"
                        }
                    ),
                    None,
                );
            }
            entry(
                &mut rows,
                "Claim reward",
                completed(panel).then_some(Kind::Claim),
            );
            entry(
                &mut rows,
                format!("Task Difficulty: {}", model::difficulty_name(b.difficulty)),
                None,
            );
            for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                entry(&mut rows, *name, Some(Kind::Difficulty(i as u8)));
            }
            if confirm {
                entry(&mut rows, "Reroll these tasks for 1 token?", None);
                entry(
                    &mut rows,
                    "Confirm reroll (1 token)",
                    Some(Kind::ConfirmReroll),
                );
                entry(&mut rows, "Cancel", Some(Kind::Cancel));
            } else {
                entry(
                    &mut rows,
                    "Reroll (1 token)",
                    (b.rerolls > 0).then_some(Kind::Reroll),
                );
            }
        }
        Tab::Weekly => {
            let w = &r.weekly;
            entry(
                &mut rows,
                format!(
                    "Week {} | Difficulty {} | Multiplier {}",
                    r.period, w.difficulty, w.multiplier
                ),
                None,
            );
            entry(
                &mut rows,
                format!(
                    "Kill XP: {} | Delivery XP: {} | Pending points: {} | Soulseals: {}",
                    w.kill_experience, w.delivery_experience, w.pending_points, w.pending_soulseals
                ),
                None,
            );
            for task in &w.kills {
                entry(
                    &mut rows,
                    format!(
                        "{}: {}/{}",
                        if task.race.is_empty() {
                            "Any creature".into()
                        } else {
                            model::creature_name(&task.race)
                        },
                        task.current,
                        task.required
                    ),
                    None,
                );
            }
            for (i, task) in w.deliveries.iter().enumerate() {
                entry(
                    &mut rows,
                    format!(
                        "Item {}: {}/{}{}",
                        task.item,
                        task.current,
                        task.required,
                        if task.delivered { " (delivered)" } else { "" }
                    ),
                    (!task.delivered && task.current >= task.required)
                        .then_some(Kind::Deliver(i as u8)),
                );
            }
            if !w.started {
                for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                    entry(
                        &mut rows,
                        format!("Start weekly: {name}"),
                        Some(Kind::Start(i as u8)),
                    );
                }
            }
            if let Some(results) = &r.weekly_results {
                entry(
                    &mut rows,
                    format!(
                        "Week {} rewards already paid: {} points, {} soulseals",
                        results.period, results.points, results.soulseals
                    ),
                    None,
                );
            }
        }
        Tab::Preferences | Tab::Soulseals => {}
        Tab::Talisman => {
            entry(
                &mut rows,
                format!("Bounty points: {}", r.bounty_points),
                None,
            );
            entry(
                &mut rows,
                format!("Buy Bounty Talisman ({TALISMAN_PURCHASE_GOLD} gold)"),
                Some(Kind::BuyTalisman),
            );
            for (path, (value, name)) in r
                .talisman
                .iter()
                .zip(["Damage", "Life leech", "Loot", "Bestiary"])
                .enumerate()
            {
                entry(
                    &mut rows,
                    format!(
                        "Talisman {} ({name}): level {}, bonus {}.{:02}%, next cost {}",
                        path + 1,
                        value.level,
                        // The reply carries hundredths of one percent (300 is 3.00%).
                        value.bonus / 100,
                        value.bonus % 100,
                        value.next_cost
                    ),
                    None,
                );
                if value.level < model::talisman::MAX_LEVEL {
                    entry(
                        &mut rows,
                        format!("Upgrade {name} ({} bounty points)", value.next_cost),
                        u8::try_from(path).ok().map(Kind::UpgradeTalisman),
                    );
                }
            }
        }
        Tab::Shop => {
            entry(
                &mut rows,
                format!(
                    "Points: {} | Bounty points: {} | Soulseals: {}",
                    r.points, r.bounty_points, r.soulseals
                ),
                None,
            );
            for offer in &r.shop {
                entry(
                    &mut rows,
                    model::shop_offer_text(offer, r.points),
                    (!offer.bought).then_some(Kind::Buy(offer.index)),
                );
            }
        }
    }
    rows
}
