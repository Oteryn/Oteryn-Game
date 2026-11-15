use super::*;
fn editing(fields: &Query<(Entity, &Field, &EditableText)>) -> bool {
    fields.iter().any(|(_, _, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    })
}
pub(super) fn submit(
    owner: &HostOwner,
    state: &mut State,
    action: Action,
    entity: Entity,
    fields: &Query<(Entity, &Field, &EditableText)>,
) {
    if editing(fields) {
        state.pending = Some((action, entity));
        return;
    }
    let mut values = state.fields.clone();
    for (_, field, text) in fields {
        values[field.0] = text.value().to_string();
    }
    perform(owner, state, action, values);
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(Entity, &Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some((action, entity)) = state.pending.take() else {
        return;
    };
    if guard.allows(entity) {
        submit(&owner, &mut state, action, entity, &fields);
    }
}
pub(super) fn perform(
    owner: &HostOwner,
    state: &mut State,
    action: Action,
    values: [String; 5],
) -> bool {
    if owner.modal_active()
        || state.key != Some(action.key)
        || owner.generation() != action.key.generation
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let current = Snapshot::read(&play.link);
    if !current.store.open || state.seen.as_ref() != Some(&current) {
        return false;
    }
    let link = &play.link;
    if action.kind == Kind::Close {
        link.store_set_open(false);
        return true;
    }
    if action.kind == Kind::Cancel {
        state.confirmation = None;
        state.dirty = true;
        return true;
    }
    if state.confirmation.is_some() && action.kind != Kind::Confirm {
        return false;
    }
    let panel = &current.store;
    if !panel.selected || panel.pending || panel.gift.pending.is_some() {
        return false;
    }
    state.fields = values;
    let success = match action.kind {
        Kind::Buy => {
            if !buy_enabled(panel) {
                return false;
            }
            let Some(quote) = store::confirmation_quote(panel) else {
                return false;
            };
            state.confirmation = Some(Confirmation {
                quote,
                fields: std::array::from_fn(|i| state.fields[i].clone()),
            });
            true
        }
        Kind::Confirm => {
            let Some(confirmation) = state.confirmation.take() else {
                return false;
            };
            if !buy_enabled(panel) || panel.catalogue.as_ref() != Some(&confirmation.quote) {
                return false;
            }
            let [name, sex, identity] = &confirmation.fields;
            link.store_confirm_purchase(&confirmation.quote, name, sex, identity)
        }
        Kind::Category(index) => {
            let Some(category) = panel
                .browser
                .page
                .as_ref()
                .and_then(|p| p.categories.get(index))
            else {
                return false;
            };
            let Ok(sort) = sort_value(&state.fields[4]) else {
                return false;
            };
            if sort > 4 {
                return false;
            }
            link.store_browse(&category.key, &state.fields[3], sort, 0);
            true
        }
        Kind::Product(index) => link.store_select_product(index),
        Kind::Products => {
            state.page = Page::Products;
            state.fields[..3].fill(String::new());
            link.store_browse(
                &panel.browser.category,
                &panel.browser.search,
                panel.browser.sort,
                0,
            );
            true
        }
        Kind::Sort => {
            state.fields[4] = ((sort_value(&state.fields[4]).unwrap_or(0) + 1) % 5).to_string();
            true
        }
        Kind::Browse => {
            let Ok(sort) = sort_value(&state.fields[4]) else {
                return false;
            };
            link.store_browse(&panel.browser.category, &state.fields[3], sort, 0);
            true
        }
        Kind::Page(step) => {
            link.store_browse(
                &panel.browser.category,
                &panel.browser.search,
                panel.browser.sort,
                step,
            );
            true
        }
        Kind::Gift | Kind::Transfer => {
            state.page = if action.kind == Kind::Gift {
                Page::Gift
            } else {
                Page::Transfer
            };
            state.fields[..3].clone_from_slice(&[
                panel.gift_fields[0].clone(),
                panel.gift_fields[1].clone(),
                String::new(),
            ]);
            link.store_gift_open();
            true
        }
        Kind::GiftControl(control) => {
            link.coin_gift_control(control, [state.fields[0].clone(), state.fields[1].clone()])
        }
        Kind::History(next) => {
            state.page = Page::History;
            link.purchase_history(next)
        }
        Kind::Ledger(next) => {
            state.page = Page::Ledger;
            link.coin_history(next)
        }
        Kind::TokenPayment => link.store_payment(
            !panel
                .catalogue
                .as_ref()
                .and_then(|p| p.offers.first())
                .is_some_and(|o| o.currency == "COSMETIC_TOKEN"),
        ),
        Kind::Close | Kind::Cancel => return false,
    };
    state.dirty = true;
    success
}

fn sort_value(value: &str) -> Result<u8, ()> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    value
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|sort| *sort <= 4)
        .ok_or(())
}
