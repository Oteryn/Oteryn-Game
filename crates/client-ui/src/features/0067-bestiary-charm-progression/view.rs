use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    format!(
        "Bestiary and Charms\r\n{}\r\n{}",
        panel.summary,
        if panel.pending {
            "Waiting for the server"
        } else if !panel.selected && panel.feedback.is_empty() {
            "Unavailable in this session"
        } else {
            &panel.feedback
        }
    )
}

pub fn bestiary_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut out = format!("Bestiary — {:?}\r\n", panel.race_filter);
    if !panel.selected {
        return "Bestiary unavailable in this session".into();
    }
    if panel.reply.is_none() {
        return "Query to load Bestiary progress".into();
    }
    for race in panel.race_cards() {
        let _ = write!(
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
            let _ = write!(
                out,
                "  Stage {}: {}/{} {}\r\n",
                index + 1,
                race.kill_count.min(*goal),
                goal,
                if race.kill_count >= *goal {
                    "complete"
                } else {
                    "in progress"
                }
            );
        }
    }
    out.push_str("Creature names, illustrations, combat statistics, resistances and loot are not supplied by this progress snapshot.\r\n");
    out
}

pub fn charms_text(panel: &Panel) -> String {
    use oteryn_client_runtime::charm_progression::charm::CharmKind;
    use std::fmt::Write;
    if !panel.selected {
        return "Charms unavailable in this session".into();
    }
    let Some(reply) = &panel.reply else {
        return "Query to load Charms".into();
    };
    let mut out = format!(
        "Charms\r\n{} Charm Points | {} Minor Charm Echoes\r\n",
        reply.charms.charm_points_available, reply.charms.minor_charm_echoes_available
    );
    for charm in &reply.charms.charms {
        let (currency, stage) = match charm.kind {
            CharmKind::Major => ("Charm Points", 3),
            CharmKind::Minor => ("Minor Charm Echoes", 1),
        };
        let _ = write!(
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
                .map_or_else(|| "none".into(), |r| r.to_string()),
            if charm.effect_active {
                "Effect active"
            } else {
                "Effect not yet active"
            }
        );
    }
    let _ = write!(
        out,
        "Selected assignment race: {}\r\n{}\r\n",
        panel.fields[1], panel.feedback
    );
    out
}
