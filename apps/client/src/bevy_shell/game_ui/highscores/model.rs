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
        Kind::WorldType(kind) => Request::WorldType(kind),
        Kind::Refresh if panel.catalogue_selected => Request::Catalogue,
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
pub(super) fn world_label(
    world: Option<oteryn_client_runtime::highscores::wire::WorldId>,
    worlds: &[(oteryn_client_runtime::highscores::wire::WorldId, String)],
) -> String {
    world.map_or_else(
        || "Current world".into(),
        |id| {
            worlds
                .iter()
                .find(|(known, _)| *known == id)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| oteryn_client_ui::highscores::world_text(id))
        },
    )
}
pub(super) fn table_rows<'a>(
    panel: &'a HighscoresPanel,
    worlds: &'a [(oteryn_client_runtime::highscores::wire::WorldId, String)],
) -> impl Iterator<Item = Row> + 'a {
    panel.reply.iter().flat_map(move |reply| {
        reply.rows.iter().enumerate().map(move |(index, row)| Row {
            cells: [
                row.rank.to_string(),
                row.name.clone(),
                row.vocation.clone(),
                world_label(reply.world, worlds),
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
