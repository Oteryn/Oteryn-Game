use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn character_info_panel(&self) -> oteryn_client_ui::character_info::Panel {
        lock(&self.mailbox).properties.character_info.clone()
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::Form;
    Form {
        title: "Character information",
        labels: &[
            "Page offset",
            "Page size (1-50)",
            "Visible character UUID",
            "Title ID (0 clears selection)",
        ],
        buttons: &["Self", "Visible", "Look", "Unlock", "Select"],
        snapshot: |mailbox| {
            let panel = &mailbox.properties.character_info;
            (
                panel.open,
                panel.pending,
                panel.fields.to_vec(),
                oteryn_client_ui::character_info::panel_text(panel),
            )
        },
        set_open: |mailbox, value| mailbox.properties.character_info.open = value,
        submit: |mailbox, commands, action, values| {
            let panel = &mut mailbox.properties.character_info;
            if panel.pending {
                return;
            }
            for (index, value) in values.into_iter().enumerate() {
                super::input::edit(panel, index, value);
            }
            let intent = match action {
                0 => oteryn_client_ui::character_info::Intent::Query,
                1 => oteryn_client_ui::character_info::Intent::Visible,
                2 => oteryn_client_ui::character_info::Intent::Look,
                3 => oteryn_client_ui::character_info::Intent::Unlock,
                4 => oteryn_client_ui::character_info::Intent::Select,
                _ => return,
            };
            super::input::submit(mailbox, commands, intent);
        },
    }
}


/// The character titles window: search, filter and row fields over the server's own titles.
pub(in crate::play) fn titles_form(
    _link: &PlayLink,
) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::play::feature_0115_mail::view::form::{Form, field};
    use oteryn_client_ui::character_info::{Intent, TitleFilter, title_rows};
    const FILTERS: [TitleFilter; 5] = [
        TitleFilter::All,
        TitleFilter::Earned,
        TitleFilter::Locked,
        TitleFilter::Permanent,
        TitleFilter::Temporary,
    ];
    Form {
        title: tr!("Character titles — Ctrl+Alt+Shift+Y"),
        labels: &[
            "Search",
            "Filter: 1 all, 2 earned, 3 locked, 4 permanent, 5 temporary",
            "Title row",
        ],
        buttons: &[
            "Apply search",
            "Refresh",
            "Unlock selected",
            "Use selected",
            "Clear selection",
        ],
        snapshot: |mailbox| {
            let p = &mailbox.properties.character_info;
            let filter = FILTERS
                .iter()
                .position(|filter| *filter == p.title_filter)
                .map_or(1, |at| at + 1);
            let current = p
                .own_titles
                .as_ref()
                .and_then(|own| own.selected)
                .map_or(tr!("None"), |title| title.name());
            let message = if !p.selected {
                tr!("Feature unavailable in this session")
            } else if p.own_titles.is_none() {
                tr!("Refresh to load your earned titles")
            } else {
                &p.feedback
            };
            let mut text = format!("{current}\n{message}");
            for (row, choice) in title_rows(p).iter().enumerate() {
                text.push_str(&format!(
                    "\n{}. {} — {}{}",
                    row + 1,
                    choice.title.name(),
                    if choice.earned {
                        tr!("Earned")
                    } else {
                        tr!("Locked")
                    },
                    if choice.selected {
                        tr!(" — Selected")
                    } else {
                        ""
                    }
                ));
            }
            (
                p.titles_open,
                p.pending,
                vec![p.title_search.clone(), filter.to_string(), String::new()],
                text,
            )
        },
        set_open: |mailbox, value| mailbox.properties.character_info.titles_open = value,
        submit: |mailbox, commands, action, values| {
            let p = &mut mailbox.properties.character_info;
            p.title_search = field(&values, 0).to_owned();
            if let Some(filter) = field(&values, 1)
                .parse::<usize>()
                .ok()
                .and_then(|at| at.checked_sub(1))
                .and_then(|at| FILTERS.get(at))
            {
                p.title_filter = *filter;
            }
            let chosen = field(&values, 2)
                .parse::<usize>()
                .ok()
                .and_then(|row| row.checked_sub(1))
                .and_then(|row| title_rows(p).get(row).cloned());
            let intent = match action {
                1 => {
                    p.fields[0] = "0".into();
                    p.fields[1] = "1".into();
                    p.fields[2].clear();
                    Intent::Query
                }
                2 | 3 => {
                    let Some(row) = chosen else {
                        p.feedback = tr!("Select a title to unlock or use it").into();
                        return;
                    };
                    if (action == 2) == row.earned {
                        return;
                    }
                    p.fields[3] = row.title.id().to_string();
                    if action == 2 {
                        Intent::Unlock
                    } else {
                        Intent::Select
                    }
                }
                4 if p.own_titles.is_some() => {
                    p.fields[3] = "0".into();
                    Intent::Select
                }
                _ => return,
            };
            super::input::submit(mailbox, commands, intent);
        },
    }
}
