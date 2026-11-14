//! Bug reports over the retained report owner and its reviewed-draft flow.
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins, window_drag::DraggableDialog};
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::player_report::Panel;
use oteryn_client_ui_bevy as source;

#[derive(Component)]
struct Field(usize);
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Category(usize),
    Review,
    Send,
    Close,
}
struct Widgets {
    root: Entity,
    fields: [Entity; 3],
    categories: [Entity; 4],
    review: Entity,
    send: Entity,
    preview: Entity,
    feedback: Entity,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    widgets: Option<Widgets>,
    pending: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.player_report_panel());
    let open = panel.as_ref().is_some_and(|panel| panel.bug.open);
    if (!open || state.generation != owner.generation())
        && let Some(widgets) = state.widgets.take()
    {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == widgets.root {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(widgets.root).despawn();
        state.pending = None;
    }
    if open
        && state.widgets.is_none()
        && let Some(panel) = panel
    {
        state.generation = owner.generation();
        let widgets = build(&mut commands, &skins, state.generation, &panel);
        focus.set(widgets.fields[1], FocusCause::Navigated);
        state.widgets = Some(widgets);
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    mut state: ResMut<State>,
    owner: NonSend<HostOwner>,
    actions: Query<&Action>,
) {
    if guard.allows(event.entity)
        && let Ok(action) = actions.get(event.entity)
        && action.generation == owner.generation()
    {
        if action.kind == Kind::Close {
            if let Some(play) = &owner.play {
                play.link.bug_report_close();
            }
            state.pending = None;
        } else if state.pending.is_none() {
            state.pending = Some(event.entity);
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    actions: Query<(Entity, &Action)>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && actions.iter().any(|(entity, action)| {
        action.kind == Kind::Close
            && action.generation == owner.generation()
            && guard.allows(entity)
    }) && let Some(play) = &owner.play
    {
        play.link.bug_report_close();
    }
}
fn finish(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    fields: Query<(&Field, &EditableText)>,
    actions: Query<&Action>,
    mut texts: Query<&mut Text>,
) {
    let Some(widgets) = &state.widgets else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if state.generation != owner.generation() || !play.link.player_report_panel().bug.open {
        state.pending = None;
        return;
    }
    let action = state.pending.and_then(|entity| {
        actions
            .get(entity)
            .ok()
            .filter(|action| action.generation == owner.generation() && guard.allows(entity))
            .map(|action| action.kind)
    });
    let committed = widgets.fields.iter().all(|entity| {
        fields.get(*entity).is_ok_and(|(_, editor)| {
            editor.pending_paste.is_none()
                && !editor
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
        })
    });
    let mut valid = committed;
    if committed {
        for (index, entity) in widgets.fields.iter().enumerate() {
            if let Ok((field, editor)) = fields.get(*entity) {
                valid &= field.0 == index
                    && play.link.bug_report_edit(index, editor.value().to_string());
            }
        }
    }
    let panel = play.link.player_report_panel();
    let ready = valid && panel.selected && !panel.pending && !panel.bug.fields[1].trim().is_empty();
    if committed && !owner.modal_active() {
        let requested = match action {
            Some(Kind::Category(category)) if panel.selected && !panel.pending => Some(category),
            Some(Kind::Review) if ready => Some(4),
            Some(Kind::Send) if ready && panel.bug.reviewed.is_some() => Some(5),
            _ => None,
        };
        if let Some(requested) = requested {
            play.link.bug_report_action(requested);
        }
    }
    let panel = play.link.player_report_panel();
    for (category, entity) in widgets.categories.iter().enumerate() {
        commands.entity(*entity).remove::<Checked>();
        if usize::from(panel.bug.category) == category {
            commands.entity(*entity).insert(Checked);
        }
        disabled(&mut commands, *entity, panel.pending || !panel.selected);
    }
    for entity in widgets.fields {
        disabled(&mut commands, entity, panel.pending);
    }
    disabled(&mut commands, widgets.review, !ready);
    disabled(
        &mut commands,
        widgets.send,
        !ready || panel.bug.reviewed.is_none(),
    );
    if let Ok(mut text) = texts.get_mut(widgets.preview) {
        text.0 = panel.bug.preview;
    }
    if let Ok(mut text) = texts.get_mut(widgets.feedback) {
        text.0 = if !panel.selected {
            tr!("Reports are unavailable in this session").into()
        } else if panel.pending {
            tr!("Waiting for the server").into()
        } else if panel.feedback == tr!("Report accepted") && !panel.summary.is_empty() {
            "Bug report sent.".into()
        } else {
            panel.feedback
        };
    }
    if committed {
        state.pending = None;
    }
}
fn disabled(commands: &mut Commands, entity: Entity, value: bool) {
    if value {
        commands.entity(entity).insert(InteractionDisabled);
    } else {
        commands.entity(entity).remove::<InteractionDisabled>();
    }
}
fn build(commands: &mut Commands, skins: &Skins, generation: u64, model: &Panel) -> Widgets {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            DraggableDialog,
            TabGroup {
                order: 4,
                modal: true,
            },
            ScrollArea,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(500.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(25.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, tr!("Bug report"), skins.font.clone());
    label(
        commands,
        skins,
        panel,
        "Please use this dialog to only report bugs. Do not report rule violations here!",
    );
    let categories = [
        "Map location",
        "Text or spelling",
        "Client behavior",
        "Other issue",
    ]
    .map(oteryn_client_ui::localization::tr);
    let categories = std::array::from_fn(|category| {
        button(
            commands,
            skins,
            panel,
            generation,
            Kind::Category(category),
            categories[category],
        )
    });
    let fields = std::array::from_fn(|index| {
        label(
            commands,
            skins,
            panel,
            [
                tr!("Map location: x,y,floor (map reports only)"),
                tr!("Steps, observed behavior and expected behavior"),
                tr!("Optional build details: enter YES to include build, OS and architecture"),
            ][index],
        );
        let mut editable = EditableText::new(&model.bug.fields[index]);
        editable.allow_newlines = index == 1;
        editable.max_characters = Some(1024);
        editable.visible_lines = Some(if index == 1 { 4. } else { 1. });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    min_height: px(if index == 1 { 80. } else { 26. }),
                    flex_shrink: 0.,
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(field);
        field
    });
    let preview = label(commands, skins, panel, &model.bug.preview);
    let feedback = label(commands, skins, panel, &model.feedback);
    let review = button(
        commands,
        skins,
        panel,
        generation,
        Kind::Review,
        tr!("Review report"),
    );
    let send = button(commands, skins, panel, generation, Kind::Send, tr!("Send"));
    button(
        commands,
        skins,
        panel,
        generation,
        Kind::Close,
        tr!("Cancel"),
    );
    Widgets {
        root,
        fields,
        categories,
        review,
        send,
        preview,
        feedback,
    }
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) -> Entity {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
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
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    kind: Kind,
    text: &str,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 220.),
            Action { generation, kind },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(text, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
    entity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    use oteryn_client_runtime::{
        player_report as wire,
        raw_feature::{CommandStatus, GatedCommandResult},
    };
    use oteryn_renderer::TileCoord;
    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    fn fixture() -> std::result::Result<
        (
            App,
            tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
            crate::play::Outbox,
        ),
        Box<dyn std::error::Error>,
    > {
        let mut owner = HostOwner::new(None)?;
        let (link, commands, outbox) = play_channel();
        link.use_shared_frontend();
        outbox.bevy_bug_report_selected();
        link.bug_report_open();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            source::GlobalUiPlugin,
        ))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
        super::super::modal::install(&mut app);
        install(&mut app);
        app.update();
        Ok((app, commands, outbox))
    }
    fn action(
        app: &mut App,
        kind: Kind,
    ) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(e, a)| (a.kind == kind).then_some(e))
            .ok_or_else(|| "missing action".into())
    }
    fn edit(app: &mut App, field: usize, value: &str) -> Result {
        let entity = app
            .world_mut()
            .query::<(Entity, &Field)>()
            .iter(app.world())
            .find_map(|(e, f)| (f.0 == field).then_some(e))
            .ok_or("missing field")?;
        let mut editor = app
            .world_mut()
            .get_mut::<EditableText>(entity)
            .ok_or("editor")?;
        *editor = EditableText::new(value);
        app.update();
        Ok(())
    }
    #[test]
    fn bug_report_send_calls_flow() -> Result {
        let (mut app, mut commands, outbox) = fixture()?;
        let category = action(&mut app, Kind::Category(2))?;
        app.world_mut().trigger(Activate { entity: category });
        app.update();
        edit(&mut app, 1, "Opening the map closes the client")?;
        let review = action(&mut app, Kind::Review)?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        assert!(commands.try_recv().is_err());
        let send = action(&mut app, Kind::Send)?;
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(send)
                .is_none(),
            "{:?}",
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .player_report_panel()
        );
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("missing report".into());
        };
        let request =
            wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(request.category, 2);
        assert_eq!(request.comment, "Opening the map closes the client");
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .all(|t| t.0 != "Bug report sent.")
        );
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(commands.try_recv().is_err());
        outbox.bevy_bug_report_result(
            &command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire::encode_player_report_reply(&wire::PlayerReportReply {
                    code: wire::PlayerReportCode::Accepted,
                    report: 42,
                    wait_seconds: 0,
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|t| t.0 == "Bug report sent.")
        );
        Ok(())
    }
    #[test]
    fn bug_report_empty_text_disabled() -> Result {
        let (mut app, mut commands, _) = fixture()?;
        for text in ["", " \n "] {
            edit(&mut app, 1, text)?;
            let send = action(&mut app, Kind::Send)?;
            assert!(
                app.world()
                    .get::<bevy::ui::InteractionDisabled>(send)
                    .is_some()
            );
            app.world_mut().trigger(Activate { entity: send });
            app.update();
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }
    #[test]
    fn bug_report_edits_require_new_review_and_close_clears_focus() -> Result {
        let (mut app, mut commands, _) = fixture()?;
        let category = action(&mut app, Kind::Category(3))?;
        app.world_mut().trigger(Activate { entity: category });
        app.update();
        edit(&mut app, 1, "The depot window closes")?;
        let review = action(&mut app, Kind::Review)?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        edit(&mut app, 1, "The depot window closes after walking")?;
        let send = action(&mut app, Kind::Send)?;
        assert!(
            app.world()
                .get::<bevy::ui::InteractionDisabled>(send)
                .is_some()
        );
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(commands.try_recv().is_err());
        let field = app
            .world_mut()
            .query_filtered::<Entity, With<Field>>()
            .iter(app.world())
            .next()
            .ok_or("field")?;
        app.world_mut()
            .resource_mut::<bevy::input_focus::InputFocus>()
            .set(field, bevy::input_focus::FocusCause::Navigated);
        let close = action(&mut app, Kind::Close)?;
        app.world_mut().trigger(Activate { entity: close });
        app.update();
        assert!(
            app.world()
                .resource::<bevy::input_focus::InputFocus>()
                .get()
                .is_none()
        );
        assert!(
            !app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .player_report_panel()
                .bug
                .open
        );
        Ok(())
    }
    #[test]
    fn bug_report_map_coordinates_and_refusal_reach_window() -> Result {
        let (mut app, mut commands, outbox) = fixture()?;
        edit(&mut app, 0, "100,200,7")?;
        edit(&mut app, 1, "This tile cannot be reached")?;
        let review = action(&mut app, Kind::Review)?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        let send = action(&mut app, Kind::Send)?;
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("missing map report".into());
        };
        let request =
            wire::decode_player_report_request(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (
                request.category,
                request.map_x,
                request.map_y,
                request.map_floor
            ),
            (0, 100, 200, 7)
        );
        outbox.bevy_bug_report_result(
            &command,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire::encode_player_report_reply(&wire::PlayerReportReply {
                    code: wire::PlayerReportCode::RateLimited,
                    report: 0,
                    wait_seconds: 30,
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
        app.update();
        let mut texts = app.world_mut().query::<&Text>();
        assert!(
            texts
                .iter(app.world())
                .any(|t| t.0.contains("Wait 30 seconds"))
        );
        assert!(texts.iter(app.world()).all(|t| t.0 != "Bug report sent."));
        Ok(())
    }
    #[test]
    fn bug_report_deferred_edits_cannot_send_reviewed_old_text() -> Result {
        let (mut app, mut commands, _) = fixture()?;
        let category = action(&mut app, Kind::Category(3))?;
        app.world_mut().trigger(Activate { entity: category });
        app.update();
        edit(&mut app, 1, "The map closes")?;
        let review = action(&mut app, Kind::Review)?;
        app.world_mut().trigger(Activate { entity: review });
        app.update();
        let field = app
            .world_mut()
            .query::<(Entity, &Field)>()
            .iter(app.world())
            .find_map(|(e, f)| (f.0 == 1).then_some(e))
            .ok_or("field")?;
        app.world_mut()
            .get_mut::<EditableText>(field)
            .ok_or("editor")?
            .pending_edits
            .push(bevy::text::TextEdit::Insert(" unexpectedly".into()));
        let send = action(&mut app, Kind::Send)?;
        app.world_mut().trigger(Activate { entity: send });
        app.update();
        assert!(commands.try_recv().is_err());
        edit(&mut app, 1, "The map closes unexpectedly")?;
        assert!(commands.try_recv().is_err());
        Ok(())
    }
    #[test]
    fn bug_report_ctrl_z_respects_input_fences() -> Result {
        use oteryn_input_actions::{
            ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
        };
        let (mut app, _, _) = fixture()?;
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play.as_ref().ok_or("play")?.link.bug_report_close();
        let key = NormalizedInputEvent::Key {
            code: KeyCode::new(29)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        };
        owner.route_input(std::slice::from_ref(&key), true);
        assert!(
            !owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .player_report_panel()
                .bug
                .open
        );
        owner.open_graphics();
        owner.route_input(std::slice::from_ref(&key), false);
        assert!(
            !owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .player_report_panel()
                .bug
                .open
        );
        owner.cancel();
        owner.route_input(&[key], false);
        assert!(
            owner
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .player_report_panel()
                .bug
                .open
        );
        Ok(())
    }
}
