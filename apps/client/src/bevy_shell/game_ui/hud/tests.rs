use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    combat_input::{CombatCommand, PlayCommand},
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, ui_widgets::Activate};
use oteryn_session::{
    ActorPosition, ActorVitals, EntityDetail, EntityKind, EntityRef, StepDirection,
    WorldSpatialEntity,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn row(generation: u64) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [2; 16],
            generation,
        },
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 42,
        },
    }
}
type AppResult = Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
>;
pub(super) fn app() -> AppResult {
    app_with_vitals(Some(ActorVitals {
        health: 42,
        max_health: 100,
        mana: 10,
        max_mana: 20,
        ..default()
    }))
}
fn app_with_vitals(vitals: Option<ActorVitals>) -> AppResult {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>();
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    outbox.bevy_hud_fixture(&[row(7)], vitals);
    host.phase = Phase::Playing;
    host.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    app.insert_non_send(host);
    let mut entities = Vec::new();
    for _ in 0..11 {
        entities.push(app.world_mut().spawn(Node::default()).id());
    }
    app.insert_resource(super::super::GameRegions {
        vitals: entities[0],
        minimap: entities[1],
        equipment: entities[2],
        containers: entities[3],
        battle: entities[4],
        skills: entities[5],
        vip: entities[6],
        analyzers: entities[7],
        chat: entities[8],
        bars: entities[9],
        world: entities[10],
    });
    app.add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
#[test]
fn battle_activate_reaches_real_command_owner_and_uses_server_vitals() -> TestResult {
    let (mut app, mut commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("attack row")?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        matches!(commands.try_recv(),Ok(PlayCommand::Battle(CombatCommand::Attack(target))) if target==row(7).entity)
    );
    app.update();
    assert!(
        app.world().get::<HudAction>(button).is_some(),
        "pending updates retain the live target control"
    );
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|value| value.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("42/100"));
    assert!(text.contains("10/20"));
    assert!(!text.contains("7907"));
    Ok(())
}
#[test]
fn stale_or_modal_battle_actions_do_not_reach_current_session() -> TestResult {
    let (mut app, mut commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| matches!(action.command, Command::Attack(_)).then_some(entity))
        .ok_or("attack row")?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.world_mut()
        .get_mut::<HudAction>(button)
        .ok_or("action")?
        .command = Command::Attack(row(6).entity);
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    {
        let mut action = app
            .world_mut()
            .get_mut::<HudAction>(button)
            .ok_or("action")?;
        action.command = Command::Attack(row(7).entity);
        action.generation = 999;
    }
    app.world_mut().trigger(Activate { entity: button });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn absent_vitals_show_unknown_instead_of_claiming_full_health() -> TestResult {
    let (mut app, _commands, _outbox) = app_with_vitals(None)?;
    app.update();
    let labels = app
        .world_mut()
        .query::<(&VitalLabel, &Text)>()
        .iter(app.world())
        .map(|(_, text)| text.0.clone())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["—", "—"]);
    for (_, node) in app
        .world_mut()
        .query::<(&VitalFill, &Node)>()
        .iter(app.world())
    {
        assert_eq!(node.width, percent(0.));
    }
    Ok(())
}

#[test]
fn analyser_selection_uses_retained_session_counters() -> TestResult {
    let (mut app, _commands, _outbox) = app()?;
    let button = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(entity, action)| {
            matches!(
                action.command,
                Command::Analyser(oteryn_client_ui::analyser::Tool::Damage)
            )
            .then_some(entity)
        })
        .ok_or("damage analyser selector")?;
    app.world_mut().trigger(Activate { entity: button });
    let host = app.world().non_send::<HostOwner>();
    let panel = host.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(panel.tool, oteryn_client_ui::analyser::Tool::Damage);
    assert_eq!(panel.kills, 23);
    Ok(())
}
