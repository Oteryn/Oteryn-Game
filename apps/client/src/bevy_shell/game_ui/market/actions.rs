use super::*;
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Close,
    Coins,
    Funds,
    Hands(u8),
    MyLevel,
    MyVocation,
    Potions,
    Search,
    Category(usize),
    Item(usize),
    Catalogue(i32),
    Book(i32),
    Statistics,
    Intent(Intent),
    Browse,
}

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
    state.character_filter = None;
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
        Kind::Hands(hand) => return play.link.market_toggle_hands(hand),
        Kind::Funds => return play.link.market_request_funds(),
        Kind::Potions => {
            return play.link.market_open_potions();
        }
        Kind::MyLevel | Kind::MyVocation => {
            let mut filter = panel.details.filter.clone();
            if matches!(action.kind, Kind::MyLevel) {
                let Some(level) = play.link.spellbook_actor_level() else {
                    let skills = play.link.skill_progress_panel();
                    if skills.complete_selected {
                        if !skills.pending {
                            play.link.skill_progress_request();
                        }
                        state.character_filter = Some(action);
                    }
                    return false;
                };
                let maximum = u16::try_from(level).unwrap_or(u16::MAX);
                filter.maximum_level = (filter.maximum_level != Some(maximum)).then_some(maximum);
                filter.minimum_level = None;
            } else {
                // General Stats includes promotions. Market filters use the base vocation.
                let character = play.link.cyclopedia_character_panel();
                let (hud_vocation, hud_pending) = play.link.own_hud_vocation();
                let vocation = character
                    .vocation_id
                    .filter(|_| character.target.is_none())
                    .or(hud_vocation);
                let family = match vocation {
                    Some(1) => 6,
                    Some(2 | 3) => 1,
                    Some(4 | 5) => 5,
                    Some(6 | 7) => 2,
                    Some(8 | 9) => 4,
                    Some(10 | 11) => 3,
                    None if character.selected && character.extensions.vocation => {
                        use oteryn_client_runtime::cyclopedia_character::CharacterTab;
                        use oteryn_client_ui::cyclopedia_character::Intent;
                        let independent = character.pending
                            || character.target.is_some()
                            || character.tab != CharacterTab::GeneralStats
                            || hud_pending;
                        let requested = if independent {
                            hud_pending || play.link.request_hud_stamina()
                        } else {
                            play.link.cyclopedia_character_request(Intent::SelectTab(
                                CharacterTab::GeneralStats,
                            ))
                        };
                        if requested {
                            state.character_filter = Some(action);
                        }
                        return false;
                    }
                    _ => return false,
                };
                filter.vocation = (filter.vocation != Some(family)).then_some(family);
            }
            MarketQuery::Catalogue { filter, page: 0 }
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
            let Some(ware) = panel
                .details
                .items
                .get(index)
                .filter(|w| panel.matches_hand_filter(w.definition))
            else {
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

pub(super) fn finish_character_filter(owner: &HostOwner, state: &mut State) {
    let Some(action) = state.character_filter else {
        return;
    };
    let Some(play) = owner.play.as_ref() else {
        return;
    };
    if action.key.generation != owner.generation()
        || state.key != Some(action.key)
        || owner.modal_active()
    {
        state.character_filter = None;
        return;
    }
    let (waiting, available) = if matches!(action.kind, Kind::MyLevel) {
        let skills = play.link.skill_progress_panel();
        (skills.pending, skills.complete.is_some())
    } else {
        let stats = play.link.cyclopedia_character_panel();
        let (hud_vocation, hud_pending) = play.link.own_hud_vocation();
        let available =
            hud_vocation.is_some() || (stats.target.is_none() && stats.vocation_id.is_some());
        (hud_pending || (stats.pending && !available), available)
    };
    if !waiting {
        state.character_filter = None;
        if available {
            perform(owner, state, action, "");
        }
    }
}
