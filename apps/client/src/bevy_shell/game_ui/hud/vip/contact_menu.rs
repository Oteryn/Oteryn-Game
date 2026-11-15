use super::*;

#[derive(Component)]
pub(super) struct ContactMenu;
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    state: Res<State>,
    lists: Query<Entity, With<List>>,
    menus: Query<Entity, With<ContactMenu>>,
    mut shown: Local<Option<(u64, u64, u64)>>,
) {
    let key = state.contact_menu.and_then(|entry| {
        owner
            .play
            .as_ref()
            .map(|play| (owner.generation(), play.link.vip_panel().revision, entry))
    });
    if *shown == key {
        return;
    }
    *shown = key;
    for entity in &menus {
        commands.entity(entity).despawn();
    }
    let Some(entry) = state.contact_menu else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.vip_panel();
    if !Snapshot::read(&panel)
        .is_ok_and(|snapshot| snapshot.entries.iter().any(|row| row.entry == entry))
    {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    let menu = commands
        .spawn((
            ContactMenu,
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_child(menu);
    for (label, command, name) in [
        ("Message", Action::MessageContact(entry), "vip.message"),
        ("Edit", Action::Edit(entry), "vip.edit"),
    ] {
        let action = button(
            &mut commands,
            &skins,
            Control {
                generation: owner.generation(),
                revision: panel.revision,
                command,
            },
            label,
            100.,
            name,
        );
        commands.entity(menu).add_child(action);
    }
}
