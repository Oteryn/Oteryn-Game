use super::*;
pub(super) fn request_action(panel: &HighscoresPanel, kind: Kind) -> Option<Request> {
    if !panel.selected || panel.pending {
        return None;
    }
    Some(match kind {
        Kind::Category => {
            let categories =
                oteryn_client_ui::highscores_loyalty::categories(panel.loyalty_selected);
            let current = categories
                .iter()
                .position(|(id, _)| *id == panel.category)?;
            Request::Filter {
                category: categories[(current + 1) % categories.len()].0,
                vocation: panel.vocation,
            }
        }
        Kind::Vocation => Request::Filter {
            category: panel.category,
            vocation: (panel.vocation + 1) % 6,
        },
        Kind::World(world) => Request::World(world),
        Kind::Refresh => Request::Refresh,
        Kind::First | Kind::Previous => {
            if panel.reply.as_ref()?.page <= 1 {
                return None;
            }
            Request::Previous
        }
        Kind::Last | Kind::Next => {
            let reply = panel.reply.as_ref()?;
            let max_page = oteryn_client_runtime::highscores::wire::MAX_HIGHSCORES_LIST_ROWS
                / oteryn_client_runtime::highscores::wire::MAX_HIGHSCORES_PAGE_ROWS as u32;
            if !reply.has_more || reply.page >= max_page {
                return None;
            }
            Request::Next
        }
        Kind::Own => {
            panel.reply.as_ref()?.own_page?;
            Request::ShowMe
        }
        Kind::Close | Kind::WorldInput => return None,
    })
}
pub(super) struct Row {
    pub cells: [String; 6],
    pub color: Color,
    pub background: Color,
}
pub(super) fn table_rows(panel: &HighscoresPanel) -> impl Iterator<Item = Row> + '_ {
    panel.reply.iter().flat_map(|reply| {
        reply.rows.iter().enumerate().map(|(index, row)| Row {
            cells: [
                row.rank.to_string(),
                row.name.clone(),
                row.vocation.clone(),
                reply
                    .world
                    .map(oteryn_client_ui::highscores::world_text)
                    .unwrap_or_else(|| "current".into()),
                row.level.to_string(),
                row.value.to_string(),
            ],
            color: if reply
                .own_row
                .as_ref()
                .is_some_and(|own| own.name == row.name && own.rank == row.rank)
            {
                Color::srgb_u8(0x60, 0xf8, 0x60)
            } else {
                source::TEXT_COLOR
            },
            background: if index % 2 == 0 {
                Color::srgb_u8(48, 48, 48)
            } else {
                Color::srgb_u8(60, 60, 60)
            },
        })
    })
}
pub(super) fn updated_text(panel: &HighscoresPanel, now: i64) -> String {
    panel
        .reply
        .as_ref()
        .map(|reply| {
            format!(
                "Last updated {} minutes ago",
                now.saturating_sub(reply.computed_at).max(0) / 60000
            )
        })
        .unwrap_or_default()
}
