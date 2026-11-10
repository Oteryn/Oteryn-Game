pub(super) fn project(
    panel: u32,
    state: &oteryn_client_ui::quick_loot::Discovery,
    store: &oteryn_client_ui::store::StorePanel,
) -> (String, Vec<String>, bool) {
    match panel {
        98 => (
            oteryn_client_ui::quick_loot::panel_text(&state.loot),
            if state.loot.managed {
                state
                    .loot
                    .destinations
                    .iter()
                    .map(|action| format!("{action:?}"))
                    .collect()
            } else {
                state
                    .loot
                    .skipped
                    .iter()
                    .map(|id| format!("Skip item {id}"))
                    .chain(
                        state
                            .loot
                            .accepted
                            .iter()
                            .map(|id| format!("Accept item {id}")),
                    )
                    .collect()
            },
            state.loot.selected && state.loot.pending.is_none(),
        ),
        132 => (
            oteryn_client_ui::inspection::panel_text(&state.inspection),
            state
                .inspection
                .reply
                .as_ref()
                .map(|r| {
                    r.requests
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("Permission request {}", i + 1))
                        .collect()
                })
                .unwrap_or_default(),
            state.inspection.selected && !state.inspection.pending,
        ),
        232 => {
            let names = state.teams.visible_names();
            (
                format!(
                    "{}\r\n{}",
                    oteryn_client_ui::team_finder::panel_text(&state.teams),
                    if state.teams.applicants {
                        oteryn_client_ui::team_finder_leader::view::panel_text(&state.leader)
                    } else {
                        String::new()
                    }
                ),
                names,
                state.teams.selected && state.teams.pending.is_none(),
            )
        }
        _ => (
            if store.page == oteryn_client_ui::store::StorePage::CoinHistory {
                let mut text = oteryn_client_ui::store::coin_history_text(&store.coin_history);
                if let Some(feedback) = &store.feedback {
                    text.push_str(&format!("\r\n{feedback}"));
                }
                text
            } else if store.page == oteryn_client_ui::store::StorePage::Catalogue {
                oteryn_client_ui::store::panel_text(store)
            } else {
                oteryn_client_ui::purchase_history::panel_text(&state.history)
            },
            Vec::new(),
            state.history.selected && !state.history.pending,
        ),
    }
}
