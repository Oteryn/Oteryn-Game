use super::{BossCategory, BossEncounterPanel};
use oteryn_client_runtime::boss_encounter::catalogue::Entry;
/// Canonical names, real classes and confirmed Character progress; no numeric boss input.
pub fn catalogue_cards(panel: &BossEncounterPanel) -> Vec<&Entry> {
    let search = panel.browser_search.trim().to_lowercase();
    panel
        .catalogue
        .iter()
        .flat_map(|p| &p.entries)
        .filter(|e| {
            (panel.browser_category.server_filter() == 0
                || panel.browser_category.server_filter() == e.category)
                && e.name.to_lowercase().contains(&search)
                && match panel.browser_category {
                    BossCategory::Progress => e.kills > 0,
                    BossCategory::Cooldown => panel
                        .snapshot
                        .as_ref()
                        .is_some_and(|p| p.cooldowns.iter().any(|(key, _)| key == &e.key)),
                    _ => true,
                }
        })
        .collect()
}
pub(super) fn records(panel: &BossEncounterPanel) -> Vec<&str> {
    catalogue_cards(panel)
        .into_iter()
        .map(|e| e.key.as_str())
        .collect()
}
pub(super) fn text(panel: &BossEncounterPanel) -> String {
    use std::fmt::Write;
    let Some(page) = &panel.catalogue else {
        return "Browse to load the boss gallery".into();
    };
    let mut text = format!(
        "\nBosstiary gallery — {:?}\nTotal boss points: {}\n",
        panel.browser_category, page.points
    );
    for entry in catalogue_cards(panel) {
        let _ = writeln!(
            text,
            "{} {} — {} | {} kills | Stage {}/3 | {} earned points",
            if panel.fields[0] == entry.key {
                ">"
            } else {
                " "
            },
            entry.name,
            entry.category_name(),
            entry.kills,
            entry.stage(),
            entry.earned_points()
        );
        if panel.fields[0] == entry.key {
            for (i, name) in ["Prowess", "Expertise", "Mastery"].into_iter().enumerate() {
                let goal = entry.thresholds[i];
                let _ = writeln!(
                    text,
                    "  {name}: {}/{} | {} total points | {}",
                    entry.kills.min(goal),
                    goal,
                    entry.points[i],
                    if entry.kills >= goal {
                        "complete"
                    } else {
                        "in progress"
                    }
                );
            }
        }
    }
    if catalogue_cards(panel).is_empty() {
        text.push_str("No matching bosses on this page\n");
    }
    text.push_str(if page.next.is_some() {
        "More canonical bosses: Next gallery page\n"
    } else {
        "End of boss catalogue\n"
    });
    text
}
