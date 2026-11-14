use super::*;
fn editing(text: &EditableText) -> bool {
    text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
}
pub(super) fn perform(owner: &HostOwner, state: &State, control: &Control, query: &str) -> bool {
    let Some(snapshot) = current(owner, state, control) else {
        return false;
    };
    let Some(play) = &owner.play else {
        return false;
    };
    let generation = owner.generation();
    match (snapshot, control.kind) {
        (Snapshot::Depot(bound), kind) => {
            let intent = match kind {
                Kind::Close => None,
                Kind::Browse { box_number, page } => {
                    Some(depot::Intent::Browse { box_number, page })
                }
                Kind::Deposit(handle) => Some(depot::Intent::Deposit(handle)),
                Kind::Retrieve(row) => Some(depot::Intent::Withdraw(row)),
                _ => return false,
            };
            play.link.depot_dispatch_bound(bound, generation, intent)
        }
        (Snapshot::Stash(bound), kind) => {
            let control = match kind {
                Kind::Close => StashControl::Close,
                Kind::Filter => StashControl::Filter(query.into()),
                Kind::Refresh => StashControl::Request(stash::Intent::Refresh),
                Kind::Previous => StashControl::Request(stash::Intent::Previous),
                Kind::Next => StashControl::Request(stash::Intent::Next),
                _ => return false,
            };
            play.link.stash_dispatch_bound(bound, generation, control)
        }
        (Snapshot::Search(bound), Kind::Close) => play
            .link
            .depot_search_dispatch_bound(bound, generation, None),
        (Snapshot::Search(bound), Kind::Search) => query.parse::<u32>().ok().is_some_and(|query| {
            play.link
                .depot_search_dispatch_bound(bound, generation, Some(query))
        }),
        _ => false,
    }
}
pub(super) fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(Entity, &Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if matches!(control.kind, Kind::Filter | Kind::Search) {
        let Some((entity, _, text)) = fields
            .iter()
            .find(|(_, field, _)| field.0 == control.key.window)
        else {
            return;
        };
        if editing(text) {
            state.pending = Some((control.clone(), event.entity, entity));
            return;
        }
        perform(&owner, &state, control, &text.value().to_string());
    } else {
        perform(&owner, &state, control, "");
    }
}
pub(super) fn finish(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<&EditableText>,
    guard: ModalGuard,
) {
    let Some((control, source, field)) = state.pending.take() else {
        return;
    };
    if !guard.allows(source) || !guard.allows(field) {
        return;
    }
    let Ok(text) = fields.get(field) else {
        return;
    };
    if editing(text) {
        state.pending = Some((control, source, field));
        return;
    }
    perform(&owner, &state, &control, &text.value().to_string());
}
pub(super) fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if !events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) {
        return;
    }
    for window in state.windows.iter().flatten() {
        if guard.allows(window.root) {
            perform(
                &owner,
                &state,
                &Control {
                    key: window.key,
                    kind: Kind::Close,
                },
                "",
            );
            break;
        }
    }
}
