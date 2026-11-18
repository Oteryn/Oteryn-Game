//! Shell control buttons: construction, activation and toggles.
use super::*;

pub(super) fn action_button(
    commands: &mut Commands,
    skins: &Skins,
    control: Control,
    caption: &str,
    width: f32,
) -> Entity {
    let button = commands
        .spawn((source::button(skins.buttons.clone(), width), control))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    button
}

pub(super) fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::super::game_ui::modal::ModalGuard,
    windows: window_layout::OpenWindows,
    mut manager: ResMut<toolbar::Manager>,
) {
    // Ordinary windows stay open together: the toolbar opens another over them.
    let beside = windows.only_windows()
        && controls.get(event.entity).is_ok_and(|control| {
            !matches!(control, Control::Options)
                && TOOLBAR.iter().any(|entry| entry.1.id() == control.id())
        });
    if !beside && !modal.allows(event.entity) {
        return;
    }
    if let Ok(control) = controls.get(event.entity) {
        if owner.modal_active()
            && matches!(
                control,
                Control::Options
                    | Control::ChangeChannel
                    | Control::Market
                    | Control::Party
                    | Control::Quests
                    | Control::Spells
                    | Control::Login
                    | Control::CancelLogin
            )
        {
            return;
        }
        match control {
            Control::ManageButtons if !owner.modal_active() && owner.play.is_some() => {
                manager.open = true;
            }
            control if TOOLBAR.iter().any(|entry| entry.1.id() == control.id()) => {
                let _ = open_toolbar(&mut owner, *control);
            }
            Control::Login => owner.request_login(),
            Control::CancelLogin => owner.cancel_login(),
            Control::Apply => owner.apply(false),
            Control::Ok => owner.apply(true),
            Control::Cancel => owner.cancel(),
            _ => {}
        }
    }
}

pub(super) fn toggle(
    event: On<ValueChange<bool>>,
    controls: Query<&Control>,
    mut owner: NonSendMut<HostOwner>,
    modal: super::super::game_ui::modal::ModalGuard,
) {
    if !modal.allows(event.source) {
        return;
    }
    if let Ok(control) = controls.get(event.source) {
        owner.edit(|draft| match control {
            Control::Fullscreen => draft.fullscreen = event.value,
            Control::Vsync => draft.vsync = event.value,
            Control::IntegerScaling => draft.integer_scaling = event.value,
            _ => {}
        });
    }
}
