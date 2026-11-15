use super::{CharacterTab, CyclopediaCharacterPanel, Tab};
use oteryn_client_ui::cyclopedia_character as model;
pub(super) fn project(tab: Tab, panel: &CyclopediaCharacterPanel) -> Vec<String> {
    match tab {
        Tab::General | Tab::Character(CharacterTab::GeneralStats) => model::overview_cards(panel)
            .into_iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect(),
        Tab::Character(CharacterTab::Offence | CharacterTab::Defence | CharacterTab::Misc) => {
            let mut rows: Vec<_> = model::combat_rows(panel)
                .into_iter()
                .map(|r| format!("{}: {} ({})", r.label, r.value, r.source))
                .collect();
            rows.extend(
                model::misc_cards(panel)
                    .into_iter()
                    .map(|(name, value)| format!("{name}: {value}")),
            );
            rows
        }
        Tab::Character(CharacterTab::RecentDeaths | CharacterTab::RecentPvpKills) => {
            model::history_rows(panel)
                .into_iter()
                .map(|row| {
                    oteryn_client_ui::cyclopedia_death_details::history_label(row)
                        .replace('\t', " | ")
                })
                .collect()
        }
        Tab::Character(CharacterTab::Achievements) => {
            let Some(page) = &panel.achievements.page else {
                return Vec::new();
            };
            let mut rows = vec![format!(
                "{} points | {} earned",
                page.total_points, page.fact_count
            )];
            let confirmed = if panel.achievements.catalog.ready {
                &panel.achievements.catalog.rows
            } else {
                &page.rows
            };
            rows.extend(confirmed.iter().map(|r| {
                format!(
                    "{} | Grade {} | {} points | Earned: {} (server Unix ms)\n{}",
                    r.name, r.grade, r.points, r.earned_at, r.description
                )
            }));
            rows
        }
        Tab::Items => panel
            .items
            .rows()
            .into_iter()
            .map(|r| {
                format!(
                    "{} | Inventory: {} | Depot: {} | Stash: {} | Inbox: {}",
                    r.definition,
                    r.quantities[0],
                    r.quantities[1],
                    r.quantities[2],
                    r.quantities[3]
                )
            })
            .collect(),
        Tab::Character(CharacterTab::Appearances) | Tab::Familiars => gallery_entries(tab, panel)
            .into_iter()
            .map(|r| format!("{:?}: {}", r.kind, r.key))
            .collect(),
        Tab::Character(CharacterTab::StoreSummary) => {
            let mut rows: Vec<_> = model::store_cards(panel)
                .into_iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect();
            if panel.store.is_some() {
                rows.push(model::store_blessings(panel));
            }
            rows
        }
        Tab::Character(CharacterTab::Inspection) | Tab::Badges => {
            let mut rows = vec![model::profile_heading(panel)];
            rows.extend(
                model::profile_rows(panel)
                    .into_iter()
                    .map(|r| format!("{}\n{}", r.label, r.detail)),
            );
            if tab == Tab::Character(CharacterTab::Inspection)
                && let Some(profile) = &panel.profile
            {
                if profile.equipment.is_empty() {
                    rows.push("No equipment shared in this profile".into());
                }
                let mut visible = profile
                    .badges
                    .iter()
                    .filter(|badge| !panel.hidden_badges.contains(&badge.id()))
                    .peekable();
                if visible.peek().is_none() {
                    rows.push("No visible earned badges in this profile".into());
                }
                for badge in visible {
                    rows.push(format!("Earned badge: {}", badge.name()));
                }
            }
            rows
        }
    }
}
pub(super) fn gallery_entries(
    tab: Tab,
    panel: &CyclopediaCharacterPanel,
) -> Vec<&oteryn_client_runtime::cyclopedia_character::gallery::Entry> {
    if !matches!(
        tab,
        Tab::Familiars | Tab::Character(CharacterTab::Appearances)
    ) {
        return Vec::new();
    }
    panel
        .gallery
        .rows()
        .into_iter()
        .filter(|entry| {
            tab != Tab::Familiars
                || entry.kind
                    == oteryn_client_runtime::cyclopedia_character::gallery::Kind::Familiar
        })
        .collect()
}
