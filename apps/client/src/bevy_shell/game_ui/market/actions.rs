use super::*;
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn submit(
    owner: &HostOwner,
    state: &mut State,
    action: Action,
    source: Entity,
    fields: &Query<(Entity, &EditableText), With<Search>>,
) {
    state.pending = None;
    if matches!(action.kind, Kind::Search) {
        let Ok((entity, text)) = fields.single() else {
            return;
        };
        if editing(text) {
            state.pending = Some((action, source, entity));
            return;
        }
        perform(owner, state, action, &text.value().to_string());
    } else {
        perform(owner, state, action, "");
    }
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(Entity, &EditableText), With<Search>>,
    guard: ModalGuard,
) {
    let Some((action, source, field)) = state.pending.take() else {
        return;
    };
    if !guard.allows(source) || !guard.allows(field) {
        return;
    }
    let Ok((_, text)) = fields.get(field) else {
        return;
    };
    if editing(text) {
        state.pending = Some((action, source, field));
        return;
    }
    perform(&owner, &mut state, action, &text.value().to_string());
}
pub(super) fn perform(owner: &HostOwner, state: &mut State, action: Action, search: &str) -> bool {
    if state.key != Some(action.key)
        || owner.generation() != action.key.generation
        || owner.modal_active()
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let panel = play.link.market_panel();
    if !panel.open || state.seen.as_ref() != Some(&panel) {
        return false;
    }
    if matches!(action.kind, Kind::Close) {
        play.link.market_set_open(false);
        return true;
    }
    if !panel.selected || panel.details.pending.is_some() || panel.pending.is_some() {
        return false;
    }
    let query = match action.kind {
        Kind::Intent(intent) => {
            play.link.market_request(intent);
            return true;
        }
        Kind::Browse => {
            if let Some(ware) = &state.selected {
                MarketQuery::Book {
                    definition: ware.definition,
                    page: 0,
                }
            } else if panel.view != MarketView::Browse {
                play.link.market_request(Intent::Wares);
                return true;
            } else {
                MarketQuery::Catalogue {
                    filter: panel.details.filter.clone(),
                    page: 0,
                }
            }
        }
        Kind::Search => {
            let mut filter = panel.details.filter.clone();
            filter.search = search.into();
            MarketQuery::Catalogue { filter, page: 0 }
        }
        Kind::Category(index) => {
            let mut filter = panel.details.filter.clone();
            filter.category = if index == 0 {
                None
            } else {
                let Some(category) = panel.details.categories.get(index - 1) else {
                    return false;
                };
                Some(category.clone())
            };
            MarketQuery::Catalogue { filter, page: 0 }
        }
        Kind::Item(index) => {
            let Some(ware) = panel.details.items.get(index) else {
                return false;
            };
            if !play.link.market_query(MarketQuery::Book {
                definition: ware.definition,
                page: 0,
            }) {
                return false;
            }
            state.selected = Some(ware.clone());
            state.dirty = true;
            return true;
        }
        Kind::Catalogue(delta) => {
            if delta > 0 && !panel.details.has_next {
                return false;
            }
            let page = panel.details.page.saturating_add_signed(delta);
            MarketQuery::Catalogue {
                filter: panel.details.filter.clone(),
                page,
            }
        }
        Kind::Book(delta) => {
            let Some(ware) = state
                .selected
                .as_ref()
                .filter(|ware| ware.definition == panel.definition)
            else {
                return false;
            };
            if delta > 0 && !panel.details.book_has_next {
                return false;
            }
            MarketQuery::Book {
                definition: ware.definition,
                page: panel.page.saturating_add_signed(delta),
            }
        }
        Kind::Statistics => {
            let Some(ware) = state
                .selected
                .as_ref()
                .filter(|ware| ware.definition == panel.definition)
            else {
                return false;
            };
            MarketQuery::Statistics {
                definition: ware.definition,
            }
        }
        Kind::Close => return false,
        Kind::Coins => {
            return owner.phase == crate::bevy_shell::owner::Phase::Playing
                && !play.view.gameplay_input_blocked(&play.link)
                && play.link.coin_market_open_bound();
        }
    };
    play.link.market_query(query)
}
