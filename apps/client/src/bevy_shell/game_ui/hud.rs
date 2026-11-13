//! Source-skinned HUD over the retained admitted session owners.
use super::super::{owner::HostOwner, ui::Skins};
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use oteryn_client_ui::{battle::Action, vip::Intent};
use oteryn_client_ui_bevy as source;
use oteryn_session::{ChaseMode, EntityRef, FightMode};

#[path = "hud/battle.rs"]
mod battle;
#[path = "hud/skills.rs"]
mod skills;
#[derive(Clone, Copy, Component)]
struct HudAction {
    generation: u64,
    command: Command,
}
#[derive(Clone, Copy)]
enum Command {
    Attack(EntityRef),
    Stop,
    Fight(FightMode),
    Chase(ChaseMode),
    Skills,
    Vip,
    BattleList(usize),
    Analyser(oteryn_client_ui::analyser::Tool),
}
#[derive(Component)]
struct VitalLabel(bool);
#[derive(Component)]
struct VitalFill(bool);
#[derive(Clone, Copy, Component)]
enum InfoText {
    Vip,
    Analyser,
}

pub(super) fn install(app: &mut App) {
    skills::install(app);
    app.add_systems(
        Startup,
        setup.after(super::setup).after(super::super::ui::setup),
    )
    .add_systems(Update, (project, battle::project).chain())
    .add_observer(activate);
}
fn setup(mut commands: Commands, regions: Res<super::GameRegions>, skins: Res<Skins>) {
    for (mana, color) in [
        (false, Color::srgb_u8(0, 160, 0)),
        (true, Color::srgb_u8(0, 70, 180)),
    ] {
        let track = commands
            .spawn((
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    flex_grow: 1.,
                    height: px(14.),
                    margin: UiRect::all(px(2.)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(30, 30, 30)),
            ))
            .id();
        let fill = commands
            .spawn((
                VitalFill(mana),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(1.),
                    top: px(1.),
                    bottom: px(1.),
                    width: percent(0.),
                    ..default()
                },
                BackgroundColor(color),
                Pickable::IGNORE,
            ))
            .id();
        let label = commands
            .spawn((
                VitalLabel(mana),
                source::text("—", source::font(skins.font.clone(), false)),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.),
                    top: px(0.),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .insert((
                TextLayout::justify(Justify::Center),
                TextColor(Color::WHITE),
            ))
            .id();
        commands.entity(track).add_children(&[fill, label]);
        commands.entity(regions.vitals).add_child(track);
    }
    for (region, title, command) in [
        (regions.skills, "Skills", Command::Skills),
        (regions.vip, "VIP", Command::Vip),
    ] {
        let header = commands
            .spawn(Node {
                width: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .id();
        let caption = commands
            .spawn(source::text(title, source::font(skins.font.clone(), false)))
            .id();
        let refresh = button(&mut commands, &skins, 0, command, "Refresh", 48.);
        commands.entity(header).add_children(&[caption, refresh]);
        commands.entity(region).add_child(header);
        if matches!(command, Command::Vip) {
            let body = commands
                .spawn((
                    InfoText::Vip,
                    source::text("", source::font(skins.font.clone(), false)),
                    Node {
                        width: percent(100.),
                        padding: UiRect::all(px(3.)),
                        ..default()
                    },
                ))
                .insert(TextLayout::default())
                .id();
            commands.entity(region).add_child(body);
        }
    }
    let title = commands
        .spawn(source::text(
            "Analysers",
            source::font(skins.font.clone(), false),
        ))
        .id();
    let tools = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    for tool in oteryn_client_ui::analyser::Tool::ALL {
        let selector = button(
            &mut commands,
            &skins,
            0,
            Command::Analyser(tool),
            tool.title(),
            86.,
        );
        commands.entity(tools).add_child(selector);
    }
    let text = commands
        .spawn((
            InfoText::Analyser,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                padding: UiRect::all(px(3.)),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(regions.analyzers)
        .add_children(&[title, tools, text]);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    command: Command,
    label: &str,
    width: f32,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            HudAction {
                generation,
                command,
            },
        ))
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    entity
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    actions: Query<&HudAction>,
    owner: NonSend<HostOwner>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if owner.modal_active() || action.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else { return };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match action.command {
        Command::Attack(target) => play.link.battle_request(Action::Attack(target)),
        Command::Stop => play.link.battle_request(Action::Stop),
        Command::Fight(mode) => play.link.battle_request(Action::Fight(mode)),
        Command::Chase(mode) => play.link.battle_request(Action::Chase(mode)),
        Command::Skills => play.link.skills_widget_request(),
        Command::Vip => play.link.vip_request(Intent::Query(0)),
        Command::BattleList(index) => {
            let _ = play.link.battle_select_list(index);
        }
        Command::Analyser(tool) => {
            if play.link.analyser_panel().selected {
                let _ = play.link.analyser_select(tool.key());
            }
        }
    }
}
// Bevy's independent queries separate projected vitals, public text and action state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut labels: Query<(&VitalLabel, &mut Text), Without<InfoText>>,
    mut info: Query<(&InfoText, &mut Text), Without<VitalLabel>>,
    mut fills: Query<(&VitalFill, &mut Node)>,
    mut actions: Query<(Entity, &mut HudAction, Has<InteractionDisabled>)>,
    mut entered: Local<Option<u64>>,
) {
    let Some(play) = &owner.play else {
        *entered = None;
        return;
    };
    let generation = owner.generation();
    if *entered != Some(generation) {
        play.link.skills_widget_request();
        play.link.vip_request(Intent::Query(0));
        *entered = Some(generation);
    }
    let vitals = play.link.actor_vitals();
    for (label, mut text) in &mut labels {
        let value = vitals.map(|v| {
            if label.0 {
                (v.mana, v.max_mana)
            } else {
                (v.health, v.max_health)
            }
        });
        let next = value.map_or_else(|| "—".into(), |(current, max)| format!("{current}/{max}"));
        if text.0 != next {
            text.0 = next;
        }
    }
    for (fill, mut node) in &mut fills {
        let value = vitals.map(|v| {
            if fill.0 {
                (v.mana, v.max_mana)
            } else {
                (v.health, v.max_health)
            }
        });
        let width = value
            .filter(|(_, max)| *max > 0)
            .map_or(0., |(current, max)| {
                current.min(max) as f32 / max as f32 * 100.
            });
        let width = percent(width);
        if node.width != width {
            node.width = width;
        }
    }
    for (kind, mut text) in &mut info {
        let next = match kind {
            InfoText::Analyser => play.link.analyser_text().replace("\r\n", "\n"),
            InfoText::Vip => vip_text(&play.link.vip_panel(), &play.link.vip_presence_panel()),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    let blocked = owner.modal_active() || play.view.gameplay_input_blocked(&play.link);
    let skills = play.link.skill_progress_panel();
    let loyalty = play.link.loyalty_skills_panel();
    let skills_blocked =
        (!skills.selected && !loyalty.selected) || skills.pending || loyalty.pending;
    let vip = play.link.vip_panel();
    let vip_blocked = !vip.selected || vip.pending;
    let analyser_blocked = !play.link.analyser_panel().selected;
    for (entity, mut action, disabled) in &mut actions {
        if matches!(
            action.command,
            Command::Skills | Command::Vip | Command::Analyser(_)
        ) {
            action.generation = generation;
            let blocked = blocked
                || match action.command {
                    Command::Skills => skills_blocked,
                    Command::Vip => vip_blocked,
                    Command::Analyser(_) => analyser_blocked,
                    _ => false,
                };
            if blocked && !disabled {
                commands.entity(entity).insert(InteractionDisabled);
            }
            if !blocked && disabled {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
        }
    }
}
fn vip_text(
    panel: &oteryn_client_ui::vip::VipPanel,
    presence: &oteryn_client_ui::vip_presence::VipPresencePanel,
) -> String {
    if !panel.selected {
        return "VIP unavailable in this session".into();
    }
    let mut lines = Vec::new();
    if let Some(view) = panel
        .view
        .as_ref()
        .and_then(|view| serde_json::from_str::<serde_json::Value>(view).ok())
    {
        for entry in view["entries"].as_array().into_iter().flatten() {
            if let Some(name) = entry["name"].as_str() {
                let status = presence
                    .presence
                    .entries
                    .iter()
                    .find(|row| Some(row.entry) == entry["entry"].as_u64())
                    .map(|row| format!("{:?}", row.status))
                    .unwrap_or_else(|| "Unconfirmed".into());
                lines.push(format!("{name} — {status}"));
            }
        }
    }
    if let Some(feedback) = &panel.feedback {
        lines.push(feedback.clone());
    }
    if panel.pending {
        lines.push("Updating…".into());
    }
    lines.join("\n")
}
#[cfg(test)]
#[path = "hud/tests.rs"]
mod tests;
