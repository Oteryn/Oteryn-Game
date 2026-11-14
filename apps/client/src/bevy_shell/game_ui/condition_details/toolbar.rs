//! Detail toolbar registrations and their input-focus projection.
use super::*;
use crate::bevy_shell::ui::Control as ToolbarControl;
use oteryn_client_ui_bevy as source;

pub(super) fn install(app: &mut App) {
    app.init_resource::<OpeningWindow>()
        .add_systems(Update, window_focus.before(super::escape));
}
#[derive(Resource, Default)]
struct OpeningWindow {
    focused: bool,
}
fn window_focus(
    mut state: ResMut<OpeningWindow>,
    windows: Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
) {
    state.focused = windows.iter().all(|window| window.focused);
}
#[derive(bevy::ecs::system::SystemParam)]
struct OpeningInput<'w, 's> {
    focus: Res<'w, InputFocus>,
    editors: Query<'w, 's, (), With<bevy::text::EditableText>>,
    // Shared toolbar observers must not register Window in headless fixtures.
    window: Option<Res<'w, OpeningWindow>>,
}
impl OpeningInput<'_, '_> {
    fn blocked(&self) -> bool {
        self.focus
            .get()
            .is_some_and(|entity| self.editors.contains(entity))
            || self.window.as_ref().is_some_and(|window| !window.focused)
    }
}
pub(in crate::bevy_shell) fn setup_toolbar(commands: &mut Commands, skin: &Skins, parent: Entity) {
    for (target, width) in [
        (Target::BattleInstinct, 100.),
        (Target::Conditions, 80.),
        (Target::Rest, 100.),
    ] {
        let button = commands
            .spawn((
                ToolbarControl::Details(target),
                source::button(skin.buttons.clone(), width),
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(target.title(), skin.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        // Insert new entries among playing windows without moving existing children.
        // Resolve anchors after the other lanes' registrations have been materialized.
        commands.queue(move |world: &mut World| {
            let index = world.get::<Children>(parent).map_or(0, |children| {
                children
                    .iter()
                    .position(|child| {
                        world.get::<ToolbarControl>(child).is_some_and(|control| {
                            !matches!(
                                control,
                                ToolbarControl::Login
                                    | ToolbarControl::CancelLogin
                                    | ToolbarControl::Options
                            ) && control.id().starts_with("shell.")
                                && control.id() > target.id()
                        })
                    })
                    .unwrap_or(children.len())
            });
            world.entity_mut(parent).insert_children(index, &[button]);
        });
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&ToolbarControl>,
    owner: NonSend<HostOwner>,
    input: OpeningInput,
) {
    if guard.allows(event.entity)
        && let Ok(ToolbarControl::Details(target)) = controls.get(event.entity)
    {
        super::toggle(&owner, *target, input.blocked());
    }
}
