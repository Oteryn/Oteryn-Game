//! Consent belongs to the displayed death and the admitted session.
use super::*;
#[derive(Component)]
pub(super) struct Panel;
#[derive(Component)]
pub(super) struct Control {
    generation: u64,
    death: u64,
    leave: bool,
}
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    panels: Query<Entity, With<Panel>>,
    controls: Query<Entity, With<Control>>,
    mut previous: Local<Option<(u64, u64)>>,
) {
    let key = owner
        .play
        .as_ref()
        .filter(|p| p.view.death_modal_notice().is_some())
        .map(|p| (owner.generation(), p.link.screenshot_facts().deaths));
    if *previous == key {
        return;
    }
    for entity in &controls {
        commands.entity(entity).despawn();
    }
    *previous = key;
    let (Some((generation, death)), Ok(parent)) = (key, panels.single()) else {
        return;
    };
    for (leave, caption) in [(false, "Continue"), (true, "Leave")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 100.),
                Control {
                    generation,
                    death,
                    leave,
                },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(parent).add_child(button);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() || owner.modal_active() {
        return;
    }
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.link.server_modal_active()
        || play.view.death_modal_notice().is_none()
        || control.death != play.link.screenshot_facts().deaths
    {
        return;
    }
    if control.leave {
        play.view.logout();
    } else {
        play.view.acknowledge_death(&play.link);
    }
}
