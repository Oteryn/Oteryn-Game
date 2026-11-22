use super::*;

/// The slot popup: the slot's assignments, then the bar's lock and visibility, which moved
/// here from the removed configuration row. Assignments are refused on a locked bar.
pub(super) fn menu_items(locked: bool) -> Vec<(popup::Item, Option<MenuChoice>)> {
    vec![
        (
            popup::Item::entry("Assign Spell"),
            Some(MenuChoice::AssignSpell),
        ),
        (
            popup::Item::entry("Assign Object"),
            Some(MenuChoice::AssignObject),
        ),
        (popup::Item::entry("Set Text"), Some(MenuChoice::SetText)),
        (
            popup::Item::entry("Edit Hotkey"),
            Some(MenuChoice::EditHotkey),
        ),
        (popup::Item::entry("Clear"), Some(MenuChoice::Clear)),
        (popup::Item::Separator, None),
        (
            popup::Item::check("Lock Action Bar", locked),
            Some(MenuChoice::Lock),
        ),
        (
            popup::Item::entry("Hide Action Bar"),
            Some(MenuChoice::Hide),
        ),
        (popup::Item::Separator, None),
        (
            popup::Item::entry("Reset Zoom"),
            Some(MenuChoice::ResetZoom),
        ),
    ]
}

// The menu owns its own widgets and the shared editor; the queries do not alias.
#[allow(clippy::too_many_arguments)]
pub(super) fn edit_click(
    event: On<PointerClick>,
    guard: ModalGuard,
    controls: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    editor: Res<editor::EditorState>,
    skins: Res<Skins>,
    scale: Option<Res<UiScale>>,
    mut menu: ResMut<SlotMenu>,
    mut commands: Commands,
) {
    if event.button != bevy::picking::pointer::PointerButton::Secondary
        || !guard.allows(event.entity)
        || owner.modal_active()
        || editor.active()
    {
        return;
    }
    let Ok((ActionControl::Slot(slot), generation)) = controls.get(event.entity) else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if generation.0 != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let Some(row) = play.link.bar_layout().rows.get(*slot / ROW_SLOTS).cloned() else {
        return;
    };
    if !row.enabled {
        return;
    }
    if let Some(old) = menu.0.take() {
        commands.entity(old).despawn();
    }
    let rows = menu_items(row.locked);
    let items: Vec<_> = rows.iter().map(|(item, _)| item.clone()).collect();
    let look = popup::Look {
        font: skins.font.clone(),
        frame: skins.frame.clone(),
        checked: Some(skins.checked.clone()),
        unchecked: Some(skins.unchecked.clone()),
    };
    let (opened, layer) = super::super::hud_panes::context_menu::spawn(
        &mut commands,
        &look,
        event.pointer.location().position,
        scale.as_deref(),
        &items,
    );
    commands.entity(opened.scrim).observe(dismiss_menu);
    for (entity, choice) in opened
        .entries
        .iter()
        .zip(rows.iter().filter_map(|(_, choice)| *choice))
    {
        commands
            .entity(*entity)
            .insert(MenuEntry {
                slot: *slot,
                generation: owner.generation(),
                choice,
            })
            .observe(menu_action);
        if row.locked
            && !matches!(
                choice,
                MenuChoice::Lock | MenuChoice::Hide | MenuChoice::ResetZoom
            )
        {
            commands
                .entity(*entity)
                .insert(bevy::ui::InteractionDisabled);
        }
    }
    menu.0 = Some(layer);
}

fn dismiss_menu(_event: On<PointerClick>, mut menu: ResMut<SlotMenu>, mut commands: Commands) {
    if let Some(layer) = menu.0.take() {
        commands.entity(layer).despawn();
    }
}

/// Escape closes the slot menu; a new session generation drops it.
pub(super) fn close_menu(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    owner: NonSend<HostOwner>,
    entries: Query<&MenuEntry>,
    mut menu: ResMut<SlotMenu>,
    mut commands: Commands,
) {
    let Some(layer) = menu.0 else {
        return;
    };
    let escape = keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape));
    let stale = owner.play.is_none()
        || entries
            .iter()
            .any(|entry| entry.generation != owner.generation());
    if escape || stale {
        commands.entity(layer).despawn();
        menu.0 = None;
    }
}

fn menu_action(
    event: On<Activate>,
    guard: ModalGuard,
    entries: Query<&MenuEntry>,
    owner: NonSend<HostOwner>,
    mut editor: ResMut<editor::EditorState>,
    (mut menu, mut scale): (ResMut<SlotMenu>, Option<ResMut<ModuleScale>>),
    mut commands: Commands,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(entry) = entries.get(event.entity) else {
        return;
    };
    if let Some(layer) = menu.0.take() {
        commands.entity(layer).despawn();
    }
    let Some(play) = &owner.play else {
        return;
    };
    if entry.generation != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let bar = entry.slot / ROW_SLOTS;
    let Some(row) = play.link.bar_layout().rows.get(bar).cloned() else {
        return;
    };
    match entry.choice {
        MenuChoice::Lock => configure(&play.link, bar, false, true),
        MenuChoice::Hide => configure(&play.link, bar, true, false),
        MenuChoice::ResetZoom => {
            if let Some(scale) = scale.as_mut() {
                scale.set(Module::ActionBars, 1.);
            }
        }
        _ if !row.enabled || row.locked => {}
        // The shared editor holds the spell catalogue, object, text and hotkey fields.
        MenuChoice::AssignSpell
        | MenuChoice::AssignObject
        | MenuChoice::SetText
        | MenuChoice::EditHotkey => editor.open(entry.slot, owner.generation()),
        MenuChoice::Clear => {
            play.link.clear_bar_action(entry.slot);
            play.link.assign_spell_slot(entry.slot, None);
        }
    }
}
