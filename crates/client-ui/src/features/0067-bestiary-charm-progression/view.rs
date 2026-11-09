use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    lfmt!(
        "Bestiary and Charms\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            tr!("Waiting for the server")
        } else if !panel.selected && panel.feedback.is_empty() {
            tr!("Unavailable in this session")
        } else {
            &panel.feedback
        }
    )
}

pub fn bestiary_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut out = super::catalogue::text(panel);
    out.push_str(&format!("Bestiary progress — {:?}\r\n", panel.race_filter));
    if !panel.selected {
        return tr!("Bestiary unavailable in this session").into();
    }
    if panel.reply.is_none() {
        out.push_str("Query to load Bestiary progress\r\n");
        return out;
    }
    for race in panel.race_cards() {
        let _ = lwrite!(
            out,
            "{}Race {}: {} kills\r\n",
            if panel.fields[1] == race.race.to_string() {
                "> "
            } else {
                ""
            },
            race.race,
            race.kill_count
        );
        for (index, goal) in race.kill_thresholds.iter().enumerate() {
            let _ = lwrite!(
                out,
                "  Stage {}: {}/{} {}\r\n",
                index + 1,
                race.kill_count.min(*goal),
                goal,
                if race.kill_count >= *goal {
                    "complete"
                } else {
                    tr!("in progress")
                }
            );
        }
    }
    out.push_str(&bestiary_detail_text(panel));

    out
}

pub fn charms_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Charms unavailable in this session".into();
    }
    if !panel.charm_browser.cards.is_empty() {
        return super::charm_browser::text(panel);
    }
    format!(
        "{}\r\nLoad charm names to select by name and read effect previews",
        legacy_charms_text(panel)
    )
}
fn legacy_charms_text(panel: &Panel) -> String {
    use oteryn_client_runtime::charm_progression::charm::CharmKind;
    use std::fmt::Write;
    if !panel.selected {
        return tr!("Charms unavailable in this session").into();
    }
    let Some(reply) = &panel.reply else {
        return tr!("Query to load Charms").into();
    };
    let mut out = lfmt!(
        "Charms\r\n{} Charm Points | {} Minor Charm Echoes\r\n",
        reply.charms.charm_points_available,
        reply.charms.minor_charm_echoes_available
    );
    for charm in &reply.charms.charms {
        let (currency, stage) = match charm.kind {
            CharmKind::Major => (tr!("Charm Points"), 3),
            CharmKind::Minor => (tr!("Minor Charm Echoes"), 1),
        };
        let _ = lwrite!(
            out,
            "{}Charm {} — {:?}, stage {}\r\nNext stage: {} {currency} | Assign to Bestiary stage {stage} or higher\r\nAssigned race: {} | {}\r\n",
            if panel.fields[0] == charm.charm.to_string() {
                "> "
            } else {
                ""
            },
            charm.charm,
            charm.kind,
            charm.unlocked_stage,
            charm.next_stage_cost,
            charm
                .assigned_race
                .map_or_else(|| tr!("none").into(), |r| r.to_string()),
            if charm.effect_active {
                tr!("Effect active")
            } else {
                tr!("Effect not yet active")
            },
            currency = currency,
            stage = stage
        );
    }
    let _ = lwrite!(
        out,
        "Selected assignment race: {}\r\n{}\r\n",
        panel.fields[1],
        panel.feedback
    );
    out
}

fn bestiary_detail_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let selected = panel.fields[1].parse::<u32>().ok();
    let Some(reply) = &panel.reply else {
        return String::new();
    };
    let Some(race) = reply
        .bestiary
        .iter()
        .find(|race| Some(race.race.get()) == selected)
    else {
        return tr!("Select a confirmed creature to inspect its discovery card\r\n").into();
    };
    let mut text = lfmt!(
        "Selected creature — race {}\r\nCurrent stage: {}/3\r\n",
        race.race,
        race.unlocked_stage()
    );
    if let Some((index, goal)) = race
        .kill_thresholds
        .iter()
        .enumerate()
        .find(|(_, goal)| race.kill_count < **goal)
    {
        let _ = lwrite!(
            text,
            "Next discovery: stage {}, {} kills remaining\r\n",
            index + 1,
            goal.saturating_sub(race.kill_count)
        );
    } else {
        text.push_str(tr!("All discovery stages complete\r\n"));
    }
    for charm in reply
        .charms
        .charms
        .iter()
        .filter(|charm| charm.assigned_race == Some(race.race))
    {
        let _ = lwrite!(
            text,
            "Assigned charm {}: {:?}, stage {}\r\n",
            charm.charm,
            charm.kind,
            charm.unlocked_stage
        );
    }
    text
}
