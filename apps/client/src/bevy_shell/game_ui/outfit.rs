//! Outfit editor over the session-owned appearance model.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::appearance::{Intent, Panel};
use oteryn_client_ui_bevy as source;
mod animation;
const PARTS: [&str; 4] = ["Head", "Primary", "Secondary", "Detail"];
#[derive(Component)]
struct Window;
#[derive(Component, Clone)]
struct Action {
    generation: u64,
    kind: Kind,
}
#[derive(Clone)]
enum Kind {
    Preview(animation::Control),
    Outfit(String),
    Addon(u8),
    Colour(usize, usize),
    Mount(String),
    Ok,
    Close,
}
#[derive(Resource, Default)]
struct State {
    rendered: Option<(u64, Panel)>,
    playback: animation::Playback,
    artwork: Option<preview::Assets>,
    image: Option<Handle<Image>>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (project, animation::animate).chain());
}
fn outfits(panel: &Panel) -> Vec<&str> {
    panel.catalogue.as_ref().map_or_else(Vec::new, |c| {
        c.outfits.iter().map(|o| o.key.as_str()).collect()
    })
}
fn intent(kind: &Kind) -> Option<Intent> {
    matches!(kind, Kind::Ok).then_some(Intent::Apply)
}
fn edit(panel: &mut Panel, kind: &Kind) -> bool {
    if panel.pending || !panel.selected {
        return false;
    }
    match kind {
        Kind::Addon(bit) => panel.toggle_addon(*bit),
        Kind::Outfit(key) => {
            let Some(outfit) = panel
                .catalogue
                .as_ref()
                .and_then(|c| c.outfits.iter().find(|o| &o.key == key))
            else {
                return false;
            };
            panel.fields[0] = key.clone();
            panel.fields[1] =
                (panel.fields[1].parse::<u8>().unwrap_or(0) & outfit.permitted_addons).to_string();
            true
        }
        Kind::Mount(key) => {
            if !key.is_empty()
                && !panel
                    .catalogue
                    .as_ref()
                    .is_some_and(|c| c.mounts.contains(key))
            {
                return false;
            }
            panel.fields[2] = key.clone();
            true
        }
        Kind::Colour(part, colour) => {
            if *part >= 4
                || !panel
                    .catalogue
                    .as_ref()
                    .is_some_and(|c| *colour < c.palette.len())
            {
                return false;
            }
            let mut colours: Vec<_> = panel.fields[3].split(',').map(str::to_owned).collect();
            if colours.len() != 4 {
                return false;
            }
            colours[*part] = colour.to_string();
            panel.fields[3] = colours.join(",");
            true
        }
        Kind::Preview(_) | Kind::Ok | Kind::Close => false,
    }
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if action.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if matches!(action.kind, Kind::Close) {
        cancel(&play.link);
        return;
    }
    if let Kind::Preview(control) = &action.kind {
        state.playback.edit(control);
        return;
    }
    send(&play.link, &action.kind);
}
fn restore_draft(link: &crate::play::PlayLink) {
    let panel = link.appearance_panel();
    if panel.pending {
        return;
    }
    let selection = panel
        .confirmed
        .as_ref()
        .map(|confirmed| &confirmed.selection);
    let fields = if let Some(selection) = selection {
        let [head, body, legs, feet] = selection.colours;
        [
            selection.outfit_key.clone(),
            selection.addons.to_string(),
            selection.mount_key.clone().unwrap_or_default(),
            format!("{head},{body},{legs},{feet}"),
        ]
    } else {
        [
            panel
                .catalogue
                .as_ref()
                .and_then(|catalogue| catalogue.outfits.first())
                .map_or_else(String::new, |outfit| outfit.key.clone()),
            "0".into(),
            String::new(),
            "0,0,0,0".into(),
        ]
    };
    for (index, value) in fields.into_iter().enumerate() {
        link.appearance_edit(index, value);
    }
}
fn cancel(link: &crate::play::PlayLink) {
    restore_draft(link);
    link.appearance_close();
}
fn send(link: &crate::play::PlayLink, kind: &Kind) {
    let mut panel = link.appearance_panel();
    if panel.pending {
        return;
    }
    if let Some(intent) = intent(kind) {
        link.appearance_request(intent);
    } else if edit(&mut panel, kind) {
        for (index, value) in panel.fields.into_iter().enumerate() {
            link.appearance_edit(index, value);
        }
    }
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    kind: Kind,
    label: &str,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), 155.),
            Action { generation, kind },
        ))
        .insert(Node {
            width: px(155.),
            height: px(24.),
            flex_shrink: 0.,
            ..default()
        })
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
    entity
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    (keys, windows): (Res<ButtonInput<KeyCode>>, Query<Entity, With<Window>>),
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
    guard: ModalGuard,
) {
    let Some(play) = &owner.play else {
        for entity in &windows {
            commands.entity(entity).despawn();
        }
        state.rendered = None;
        return;
    };
    if keys.just_pressed(KeyCode::Escape)
        && !owner.modal_active()
        && windows.iter().any(|entity| guard.allows(entity))
    {
        cancel(&play.link);
    }
    if state.rendered.is_none() && play.link.appearance_panel().open {
        restore_draft(&play.link);
    }
    let panel = play.link.appearance_panel();
    if !panel.open {
        for entity in &windows {
            commands.entity(entity).despawn();
        }
        state.rendered = None;
        return;
    }
    let generation = owner.generation();
    if state.rendered.as_ref() == Some(&(generation, panel.clone())) {
        return;
    }
    for entity in &windows {
        commands.entity(entity).despawn();
    }
    let root = commands
        .spawn((
            Window,
            GameModal,
            GlobalZIndex(175),
            Node {
                position_type: PositionType::Absolute,
                left: percent(15.),
                top: percent(8.),
                width: percent(70.),
                max_height: percent(84.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(10.)),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
            BackgroundColor(Color::srgb(0.16, 0.16, 0.16)),
        ))
        .id();
    let title = commands
        .spawn(source::button_caption("Outfit", skin.font.clone()))
        .id();
    commands.entity(root).add_child(title);
    for key in outfits(&panel) {
        button(
            &mut commands,
            &skin,
            root,
            generation,
            Kind::Outfit(key.into()),
            key,
        );
    }
    if let Some(catalogue) = &panel.catalogue {
        if let Some(outfit) = catalogue.outfits.iter().find(|o| o.key == panel.fields[0]) {
            for bit in [1, 2] {
                if outfit.permitted_addons & bit != 0 {
                    button(
                        &mut commands,
                        &skin,
                        root,
                        generation,
                        Kind::Addon(bit),
                        &format!(
                            "Addon {bit}: {}",
                            panel.fields[1].parse::<u8>().unwrap_or(0) & bit != 0
                        ),
                    );
                }
            }
        }
        for (part, name) in PARTS.iter().enumerate() {
            let label = commands
                .spawn(source::button_caption(*name, skin.font.clone()))
                .id();
            commands.entity(root).add_child(label);
            let row = commands
                .spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .id();
            commands.entity(root).add_child(row);
            for (index, rgb) in catalogue.palette.iter().enumerate() {
                let cell = button(
                    &mut commands,
                    &skin,
                    row,
                    generation,
                    Kind::Colour(part, index),
                    &index.to_string(),
                );
                commands.entity(cell).remove::<ImageNode>().insert((
                    Node {
                        width: px(24.),
                        height: px(24.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(rgb[0], rgb[1], rgb[2])),
                ));
            }
        }
        button(
            &mut commands,
            &skin,
            root,
            generation,
            Kind::Mount(String::new()),
            "No mount",
        );
        for key in &catalogue.mounts {
            button(
                &mut commands,
                &skin,
                root,
                generation,
                Kind::Mount(key.clone()),
                key,
            );
        }
    }
    if state.artwork.is_none() {
        state.artwork = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .and_then(|root| preview::Assets::load(std::path::Path::new(&root)).ok());
    }
    if panel.preview().is_some() {
        let pixels = state
            .artwork
            .as_mut()
            .map(|assets| assets.draw_panel(&panel, 2, 0, false));
        match pixels {
            Some(Ok(pixels)) => {
                let image = animation::image(pixels);
                let handle = if let Some(handle) = &state.image {
                    if let Some(mut target) = images.get_mut(handle) {
                        *target = image;
                    }
                    handle.clone()
                } else {
                    let handle = images.add(image);
                    state.image = Some(handle.clone());
                    handle
                };
                let picture = commands
                    .spawn((
                        ImageNode::new(handle),
                        Node {
                            width: px(192.),
                            height: px(192.),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(root).add_child(picture);
            }
            Some(Err(message)) => {
                let label = commands
                    .spawn(source::button_caption(message, skin.font.clone()))
                    .id();
                commands.entity(root).add_child(label);
            }
            None => {
                let label = commands
                    .spawn(source::button_caption(
                        "Local outfit artwork unavailable",
                        skin.font.clone(),
                    ))
                    .id();
                commands.entity(root).add_child(label);
            }
        }
    }
    for (control, label) in [
        (animation::Control::Left, "Turn left"),
        (animation::Control::Right, "Turn right"),
        (animation::Control::Movement(true), "Movement on"),
        (animation::Control::Movement(false), "Movement off"),
    ] {
        button(
            &mut commands,
            &skin,
            root,
            generation,
            Kind::Preview(control),
            label,
        );
    }
    let preview = commands
        .spawn(source::button_caption(
            format!(
                "Preview: {} / mount: {}\n{}",
                panel.fields[0], panel.fields[2], panel.feedback
            ),
            skin.font.clone(),
        ))
        .id();
    commands.entity(root).add_child(preview);
    button(&mut commands, &skin, root, generation, Kind::Ok, "OK");
    button(
        &mut commands,
        &skin,
        root,
        generation,
        Kind::Close,
        "Cancel",
    );
    state.playback.invalidate();
    state.rendered = Some((generation, panel));
}
#[path = "../../features/0025-appearance/preview.rs"]
pub(super) mod preview;
#[cfg(test)]
mod tests;
