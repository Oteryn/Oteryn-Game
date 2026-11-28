use super::input::send_page;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::account_achievements::{
    ACCOUNT_ACHIEVEMENTS_PAGE_ROWS, AchievementCompletion,
};
use oteryn_client_ui::account_achievements::AchievementsPanel;
impl PlayLink {
    #[must_use]
    pub fn achievements_panel(&self) -> AchievementsPanel {
        lock(&self.mailbox).achievements.clone()
    }
}

fn shown_rows(
    panel: &AchievementsPanel,
) -> Vec<&oteryn_client_runtime::account_achievements::AccountAchievementRow> {
    if panel.catalog.ready {
        oteryn_client_ui::account_achievements::catalog_rows(&panel.catalog)
    } else {
        panel
            .page
            .as_ref()
            .map_or_else(Vec::new, |page| page.rows.iter().collect())
    }
}

fn achievements_text(panel: &AchievementsPanel) -> String {
    let rows = shown_rows(panel);
    let matching = rows.len();
    let ready = panel.catalog.ready;
    let status = if !panel.selected {
        tr!("Feature unavailable in this session")
    } else if panel.catalog.collecting {
        tr!("Loading the complete account list")
    } else if panel.pending.is_some() {
        tr!("Loading…")
    } else {
        panel.error.unwrap_or("")
    };
    let (points, earned, page) = if ready {
        (
            panel.catalog.points,
            panel.catalog.earned_count,
            (panel.catalog.page as u64).saturating_add(1),
        )
    } else {
        panel.page.as_ref().map_or((0, 0, 1), |page| {
            (page.total_points, page.fact_count, u64::from(page.page) + 1)
        })
    };
    let scope = if ready {
        "matching"
    } else {
        tr!("here; load all for filters")
    };
    let mut text =
        format!("{points} points · {earned} earned · {matching} {scope} · Page {page}\n{status}\n");
    if let Some(mode) = panel.completion {
        let label = match mode {
            AchievementCompletion::Completed => tr!("Show completed"),
            AchievementCompletion::Incomplete => tr!("Hide completed"),
        };
        text.push_str(label);
        text.push('\n');
    }
    let skip = if ready {
        panel
            .catalog
            .page
            .saturating_mul(ACCOUNT_ACHIEVEMENTS_PAGE_ROWS)
    } else {
        0
    };
    for row in rows
        .into_iter()
        .skip(skip)
        .take(ACCOUNT_ACHIEVEMENTS_PAGE_ROWS)
    {
        let secret = if row.secret { tr!(" — Secret") } else { "" };
        text.push_str(&format!(
            "{} — Grade {} — {} points{secret}\n  {}\n",
            row.name, row.grade, row.points, row.description
        ));
    }
    text
}

/// Account achievements for the shared frontend; search, filter and sort apply once loaded.
pub(in crate::play) fn form(link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use oteryn_client_ui::account_achievements::{AchievementFilter, AchievementSort, Intent};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Account achievements — Ctrl+Shift+A"),
        labels: &[
            "Search",
            "Filter: 0 all, 1 regular, 2 secret",
            "Sort: 0 name, 1 points, 2 grade, 3 earned",
        ],
        buttons: if lock(&link.mailbox).achievements.completion.is_some() {
            &[
                "Load all",
                "Previous",
                "Next",
                "Apply search",
                "Show completed",
                "Hide completed",
            ]
        } else {
            &["Load all", "Previous", "Next", "Apply search"]
        },
        snapshot: |mailbox| {
            let panel = &mailbox.achievements;
            let filter = match panel.catalog.filter {
                AchievementFilter::All => "0",
                AchievementFilter::Regular => "1",
                AchievementFilter::Secret => "2",
            };
            let sort = match panel.catalog.sort {
                AchievementSort::Name => "0",
                AchievementSort::Points => "1",
                AchievementSort::Grade => "2",
                AchievementSort::Earned => "3",
            };
            (
                panel.open,
                panel.pending.is_some(),
                vec![
                    panel.catalog.search.clone(),
                    filter.to_owned(),
                    sort.to_owned(),
                ],
                achievements_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.achievements.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            let panel = &mut mailbox.achievements;
            if panel.catalog.ready {
                let filter = match field(&values, 1) {
                    "1" => AchievementFilter::Regular,
                    "2" => AchievementFilter::Secret,
                    _ => AchievementFilter::All,
                };
                let sort = match field(&values, 2) {
                    "1" => AchievementSort::Points,
                    "2" => AchievementSort::Grade,
                    "3" => AchievementSort::Earned,
                    _ => AchievementSort::Name,
                };
                panel.catalog.set_options(field(&values, 0), filter, sort);
            }
            let matching = shown_rows(panel).len();
            let intent = match action {
                0 => {
                    panel.catalog.enabled = true;
                    Some(Intent::Refresh)
                }
                4 => Some(Intent::Completion(AchievementCompletion::Completed)),
                5 => Some(Intent::Completion(AchievementCompletion::Incomplete)),
                1 if !panel.catalog.enabled => Some(Intent::Previous),
                2 if !panel.catalog.enabled => Some(Intent::Next),
                1 => {
                    panel.catalog.page = panel.catalog.page.saturating_sub(1);
                    None
                }
                2 if panel
                    .catalog
                    .page
                    .saturating_add(1)
                    .saturating_mul(ACCOUNT_ACHIEVEMENTS_PAGE_ROWS)
                    < matching =>
                {
                    panel.catalog.page += 1;
                    None
                }
                _ => None,
            };
            if let Some(intent) = intent
                && let Some(page) = oteryn_client_ui::account_achievements::request(panel, intent)
            {
                send_page(panel, commands, page);
            }
        },
    }
}

/// While the complete list loads, each frame asks for its next page, as the native window did.
pub(in crate::play) fn tick(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    if !mailbox.achievements.open {
        return;
    }
    let panel = &mut mailbox.achievements;
    if let Some(page) = oteryn_client_ui::account_achievements::catalog_next_request(panel) {
        send_page(panel, commands, page);
    }
}
