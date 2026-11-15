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
            if tab == Tab::Character(CharacterTab::Offence) {
                rows.push(model::augment_heading(panel));
                rows.extend(
                    model::augment_rows(panel)
                        .into_iter()
                        .map(|row| format!("{}\n{}", row.label, row.detail)),
                );
            }
            rows
        }
        Tab::Character(CharacterTab::RecentDeaths | CharacterTab::RecentPvpKills) => {
            model::history_rows(panel)
                .into_iter()
                .map(|row| {
                    let label = oteryn_client_ui::cyclopedia_death_details::history_label(row);
                    let detail = label.split_once('\t').map_or("", |(_, detail)| detail);
                    format!(
                        "{} | {}",
                        oteryn_client_ui::house::format_ownership_date(row.at),
                        detail.replace('\t', " | ")
                    )
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
                    "{} | Grade {} | {} points | Earned: {}\n{}",
                    r.name,
                    r.grade,
                    r.points,
                    oteryn_client_ui::house::format_ownership_date(r.earned_at),
                    r.description
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
        Tab::Character(CharacterTab::Appearances) => panel
            .gallery
            .rows()
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
            rows
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cyclopedia_histories_and_achievements_use_calendar_dates() {
        let mut panel = CyclopediaCharacterPanel::default();
        for tab in [CharacterTab::RecentDeaths, CharacterTab::RecentPvpKills] {
            panel.tab = tab;
            panel.history_tab = Some(tab);
            panel.history = vec![model::HistoryRow {
                at: 1700000000123,
                status: None,
                description: "History".into(),
                death: None,
            }];
            let rows = project(Tab::Character(tab), &panel);
            assert!(rows[0].contains("2023-11-14 22:13:20 UTC"));
        }
        panel.achievements.page = Some(
            oteryn_client_runtime::account_achievements::AccountAchievementsResult {
                total_points: 1,
                fact_count: 1,
                page: 0,
                has_more: false,
                rows: vec![
                    oteryn_client_runtime::account_achievements::AccountAchievementRow {
                        key: "oteryn:achievement/test".into(),
                        name: "Earned".into(),
                        description: "Description".into(),
                        grade: 1,
                        points: 1,
                        earned_at: 1700000000123,
                        secret: false,
                    },
                ],
            },
        );
        assert!(
            project(Tab::Character(CharacterTab::Achievements), &panel)[1]
                .contains("2023-11-14 22:13:20 UTC")
        );
    }
}
