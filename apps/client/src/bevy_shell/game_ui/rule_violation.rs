//! Rule-violation presentation over the existing capability-76 report intent.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    input_focus::{InputFocus, tab_navigation::TabGroup},
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_runtime::player_report as wire;
use oteryn_client_ui::player_report::Panel;
use oteryn_client_ui_bevy as source;
use oteryn_renderer::TileCoord;
use oteryn_session::EntityRef;

pub(in crate::bevy_shell) struct Target {
    generation: u64,
    tile: TileCoord,
    entity: EntityRef,
    name: String,
}
pub(in crate::bevy_shell) fn context(owner: &HostOwner, tile: TileCoord) -> Option<Target> {
    let play = owner.play.as_ref()?;
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !play.link.player_report_panel().selected
    {
        return None;
    }
    let (entity, name) = play.view.party_context_target(tile)?;
    Some(Target {
        generation: owner.generation(),
        tile,
        entity,
        name: name?,
    })
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner, target: &Target) -> bool {
    let Some(play) = &owner.play else {
        return false;
    };
    if owner.phase != Phase::Playing
        || owner.modal_active()
        || owner.generation() != target.generation
        || play.view.gameplay_input_blocked(&play.link)
        || play.view.party_context_target(target.tile)
            != Some((target.entity, Some(target.name.clone())))
    {
        return false;
    }
    play.link.rule_violation_open(&target.name);
    play.link.player_report_panel().open
}
fn reasons() -> impl ExactSizeIterator<Item = (u8, String)> {
    (0..=wire::MAX_REPORT_REASON).map(|id| {
        let mut panel = Panel::default();
        panel.fields[0] = id.to_string();
        (id, panel.reason_help())
    })
}
fn actions() -> [(u8, &'static str); 3] {
    [(0, "Name"), (1, "Statement"), (2, "Bot")]
}
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Status;
#[derive(Clone, Copy)]
enum Kind {
    Reason(u8),
    Action(u8),
    Send,
    Close,
}
#[derive(Component)]
struct Control {
    generation: u64,
    kind: Kind,
}
#[derive(Resource, Default)]
struct State {
    root: Option<Entity>,
    generation: u64,
    send: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project)
        .add_systems(PostUpdate, sync.after(bevy::text::EditableTextSystems));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<Entity, With<Field>>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.player_report_panel());
    let visible = owner.phase == Phase::Playing && panel.as_ref().is_some_and(|panel| panel.open);
    if (!visible || state.generation != owner.generation())
        && let Some(root) = state.root.take()
    {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
        state.send = false;
    }
    if !visible || state.root.is_some() {
        return;
    }
    let Some(panel) = panel else {
        return;
    };
    state.generation = owner.generation();
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(170),
            TabGroup {
                order: 5,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
        ))
        .id();
    let window = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(560.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollArea,
        ))
        .id();
    commands.entity(root).add_child(window);
    source::dialog_caption(
        &mut commands,
        window,
        "Report rule violation",
        skins.font.clone(),
    );
    label(&mut commands, &skins, window, "Reported character");
    editor(&mut commands, &skins, window, 2, &panel.fields[2]);
    label(&mut commands, &skins, window, "Choose a reason");
    let list = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(220.),
                min_height: px(90.),
                flex_shrink: 1.,
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(window).add_child(list);
    for (id, help) in reasons() {
        let (name, description) = help.split_once("\r\n").unwrap_or((&help, ""));
        button(
            &mut commands,
            &skins,
            list,
            state.generation,
            Kind::Reason(id),
            name,
        );
        label(&mut commands, &skins, list, description);
    }
    label(
        &mut commands,
        &skins,
        window,
        "Choose an action (report type)",
    );
    for (id, name) in actions() {
        button(
            &mut commands,
            &skins,
            window,
            state.generation,
            Kind::Action(id),
            name,
        );
    }
    label(
        &mut commands,
        &skins,
        window,
        "Comment (up to 1,024 UTF-8 bytes)",
    );
    editor(&mut commands, &skins, window, 4, &panel.fields[4]);
    let status = label(&mut commands, &skins, window, "");
    commands.entity(status).insert(Status);
    button(
        &mut commands,
        &skins,
        window,
        state.generation,
        Kind::Send,
        "Send",
    );
    button(
        &mut commands,
        &skins,
        window,
        state.generation,
        Kind::Close,
        "Cancel",
    );
    state.root = Some(root);
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) -> Entity {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn editor(commands: &mut Commands, skins: &Skins, parent: Entity, index: usize, value: &str) {
    let mut text = EditableText::new(value);
    text.allow_newlines = index == 4;
    text.visible_lines = Some(if index == 4 { 3. } else { 1. });
    text.max_characters = Some(if index == 4 {
        wire::MAX_COMMENT_BYTES
    } else {
        wire::MAX_TARGET_BYTES
    });
    let entity = commands
        .spawn((
            Field(index),
            TextInput,
            text,
            TextCursorStyle::default(),
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                min_height: px(28.),
                flex_shrink: 0.,
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    kind: Kind,
    name: &str,
) {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 510.),
            Control { generation, kind },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(name, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() || state.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    match control.kind {
        Kind::Reason(id) => play.link.player_report_edit(0, id.to_string()),
        Kind::Action(id) => play.link.player_report_edit(1, id.to_string()),
        Kind::Close => play.link.rule_violation_close(),
        Kind::Send => state.send = true,
    }
}
fn sync(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(Entity, &Field, &EditableText)>,
    controls: Query<(Entity, &Control)>,
    mut status: Query<&mut Text, With<Status>>,
) {
    if state.root.is_none() || state.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.player_report_panel();
    if !panel.open {
        state.send = false;
        return;
    }
    let mut editing = false;
    for (entity, field, text) in &fields {
        editing |= text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive);
        play.link
            .player_report_edit(field.0, text.value().to_string());
        if panel.pending {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    if state.send && !editing {
        state.send = false;
        play.link.rule_violation_send();
    }
    let panel = play.link.player_report_panel();
    for (entity, control) in &controls {
        let disabled = match control.kind {
            Kind::Send => editing || !play.link.rule_violation_can_send(),
            Kind::Close => false,
            Kind::Reason(_) | Kind::Action(_) => panel.pending,
        };
        if disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
    for mut text in &mut status {
        let action = actions()
            .into_iter()
            .find(|(id, _)| panel.fields[1] == id.to_string())
            .map_or("Choose an action", |(_, label)| label);
        text.0 = format!(
            "{}\nAction: {}\n{}\n{}",
            panel.reason_help(),
            action,
            panel.feedback,
            panel.summary
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::player_report as wire;
    #[test]
    fn rule_violation_reasons_from_protocol() {
        let reasons = reasons();
        assert_eq!(reasons.len(), usize::from(wire::MAX_REPORT_REASON) + 1);
        for (id, text) in reasons {
            assert!(text.contains(&format!("Reason {id}:")));
            assert!(text.contains("Evidence:"));
            assert!(!text.contains("Choose a report reason"));
        }
        assert_eq!(actions().len(), usize::from(wire::MAX_REPORT_ACTION) + 1);
    }
    #[test]
    fn rule_violation_window_widgets_send_existing_intent() -> Result<(), Box<dyn std::error::Error>>
    {
        use crate::{
            combat_input::PlayCommand,
            play::{PlayView, play_channel},
        };
        let (link, mut receiver, outbox) = play_channel();
        link.use_shared_frontend();
        outbox.bevy_report_selected(true);
        link.rule_violation_open("Other Player");
        let mut owner = HostOwner::new(None)?;
        owner.phase = Phase::Playing;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        });
        let mut app = App::new();
        app.insert_non_send(owner)
            .init_resource::<InputFocus>()
            .insert_resource(Skins {
                buttons: source::ButtonImages {
                    up: Handle::default(),
                    down: Handle::default(),
                },
                checked: Handle::default(),
                unchecked: Handle::default(),
                frame: Handle::default(),
                inset: Handle::default(),
                font: Handle::default(),
            });
        install(&mut app);
        app.update();
        let control = |app: &mut App, predicate: fn(Kind) -> bool| {
            app.world_mut()
                .query::<(Entity, &Control)>()
                .iter(app.world())
                .find(|(_, control)| predicate(control.kind))
                .map(|(entity, _)| entity)
        };
        let send = control(&mut app, |kind| matches!(kind, Kind::Send)).ok_or("send")?;
        assert!(app.world().get::<InteractionDisabled>(send).is_some());
        for predicate in [
            (|kind| matches!(kind, Kind::Reason(6))) as fn(Kind) -> bool,
            |kind| matches!(kind, Kind::Action(1)),
        ] {
            let entity = control(&mut app, predicate).ok_or("choice")?;
            app.world_mut().trigger(Activate { entity });
        }
        for (field, mut text) in app
            .world_mut()
            .query::<(&Field, &mut EditableText)>()
            .iter_mut(app.world_mut())
        {
            if field.0 == 4 {
                *text = EditableText::new("Observed repeated messages");
            }
        }
        app.update();
        assert!(app.world().get::<InteractionDisabled>(send).is_none());
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        let PlayCommand::Feature(command) = receiver.try_recv()? else {
            return Err("report".into());
        };
        let report =
            wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!((report.reason, report.action), (6, 1));
        assert_eq!(report.target, "Other Player");
        assert_eq!(report.comment, "Observed repeated messages");
        assert!(app.world().get::<InteractionDisabled>(send).is_some());
        Ok(())
    }
}
