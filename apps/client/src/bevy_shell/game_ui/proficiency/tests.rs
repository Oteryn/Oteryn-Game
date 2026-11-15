use super::*;
use crate::bevy_shell::owner::HostOwner;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_runtime::{
    raw_feature::{FeatureCommand, GatedCommandResult},
    weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
        SelectPerkRequest,
    },
};
use oteryn_client_ui::weapon_proficiency::Intent;
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn catalogue(experience: u32) -> ProficiencyCataloguePage {
    ProficiencyCataloguePage {
        offset: 0,
        total: 2,
        entries: vec![
            ProficiencyCatalogueEntry {
                definition: 258,
                weapon: "sword".into(),
                name: "Sword".into(),
                experience,
                thresholds: [100, 200, 300, 400, 500, 600, 700, 800, 900],
                levels: vec![
                    ProficiencyCatalogueLevel {
                        selected: Some(0),
                        perks: vec!["Critical hit +1%".into(), "Damage +2%".into()],
                    },
                    ProficiencyCatalogueLevel {
                        selected: None,
                        perks: vec!["Life leech +1%".into()],
                    },
                ],
            },
            ProficiencyCatalogueEntry {
                definition: 259,
                weapon: "axe".into(),
                name: "Axe".into(),
                experience: 0,
                thresholds: [100, 200, 300, 400, 500, 600, 700, 800, 900],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: None,
                    perks: vec!["Damage +3%".into()],
                }],
            },
        ],
    }
}
fn publish(outbox: &crate::play::Outbox, page: &ProficiencyCataloguePage) -> Result {
    outbox.bevy_proficiency_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: page.encode().ok_or("catalogue encoding")?,
    });
    Ok(())
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    Ok(command)
}
fn app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_proficiency_initialize();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .weapon_proficiency_input(&[oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(19)?,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::one(
                oteryn_input_actions::Modifier::Control,
            )
            .with(oteryn_input_actions::Modifier::Shift),
            repeat: false,
        }]);
    assert_eq!(command(&mut commands)?.kind, 660);
    publish(&outbox, &catalogue(150))?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
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
fn click(app: &mut App, kind: impl Fn(Kind) -> bool) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| kind(control.kind))
        .map(|(entity, _)| entity)
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}
#[test]
fn proficiency_weapon_list() -> Result {
    let (mut app, mut commands, _) = app()?;
    let labels = texts(&mut app);
    assert!(labels.iter().any(|label| label.contains("Sword")
        && label.contains("Level 1")
        && label.contains("150 / 400 XP")));
    assert!(labels.iter().any(|label| label.contains("Axe")
        && label.contains("Level 0")
        && label.contains("0 / 300 XP")));
    let bars: Vec<_> = app
        .world_mut()
        .query_filtered::<&Node, With<Progress>>()
        .iter(app.world())
        .map(|node| node.width)
        .collect();
    assert!(bars.contains(&Val::Percent(37.)));
    assert!(bars.contains(&Val::Percent(0.)));
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|label| label.contains("Critical hit +1%"))
    );
    assert!(
        commands.try_recv().is_err(),
        "selection must not invent a shaping query without capability 448"
    );
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn proficiency_perk_select_guard() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Perk(1, 0)))?;
    assert!(
        commands.try_recv().is_err(),
        "locked level must never send an intent"
    );
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Apply))?;
    let sent = command(&mut commands)?;
    assert_eq!((sent.capability, sent.kind), (195, 378));
    let selection = SelectPerkRequest::decode(&sent.payload).ok_or("perk request")?;
    assert_eq!(selection.definition.get(), 258);
    assert_eq!((selection.level, selection.perk), (0, Some(1)));
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(
        commands.try_recv().is_err(),
        "pending/stale widget must be fenced"
    );
    // A reply can invalidate the displayed catalogue before the next Bevy projection.
    outbox.bevy_proficiency_result(&GatedCommandResult {
        command_id: 2,
        status: CommandStatus::Rejected,
        server_sequence: 2,
        payload: vec![],
    });
    let owner = app.world().non_send::<HostOwner>();
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .weapon_proficiency_request(Intent::Catalogue);
    assert_eq!(command(&mut commands)?.kind, 660);
    publish(&outbox, &catalogue(0))?;
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(
        commands.try_recv().is_err(),
        "a stale unlocked widget must not select after XP changes"
    );
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn proficiency_modal_and_session_fences() -> Result {
    let (mut app, mut commands, _) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    let blocker = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(blocker);
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn proficiency_reopen_restarts_catalogue_after_all_pages() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    assert_eq!(command(&mut commands)?.payload, 0u32.to_be_bytes());
    let mut first = catalogue(150);
    first.total = 3;
    publish(&outbox, &first)?;
    app.update();
    let next = command(&mut commands)?;
    assert_eq!(next.kind, 660);
    assert_eq!(next.payload, 2u32.to_be_bytes());
    let mut last = catalogue(150);
    last.offset = 2;
    last.total = 3;
    last.entries.truncate(1);
    last.entries[0].definition = 260;
    last.entries[0].weapon = "club".into();
    last.entries[0].name = "Club".into();
    publish(&outbox, &last)?;
    app.update();
    assert!(texts(&mut app).iter().any(|text| text.contains("Club")));
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    let key = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(19)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    let reopened = command(&mut commands)?;
    assert_eq!(reopened.kind, 660);
    assert_eq!(
        reopened.payload,
        0u32.to_be_bytes(),
        "reopening must refresh from the first catalogue page"
    );
    Ok(())
}

#[test]
fn proficiency_poll_preserves_panes_scroll_and_focused_control() -> Result {
    use bevy::{
        input_focus::{FocusCause, InputFocus},
        ui::ScrollPosition,
        ui_widgets::ScrollArea,
    };
    let (mut app, _commands, outbox) = app()?;
    app.init_resource::<InputFocus>();
    let panes: Vec<_> = app
        .world_mut()
        .query_filtered::<Entity, With<ScrollArea>>()
        .iter(app.world())
        .collect();
    let weapon = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(id, c)| matches!(c.kind, Kind::Weapon(0)).then_some(id))
        .ok_or("weapon")?;
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(weapon, FocusCause::Navigated);
    for pane in &panes {
        app.world_mut()
            .entity_mut(*pane)
            .insert(ScrollPosition(Vec2::new(0., 50.)));
    }
    publish(&outbox, &catalogue(175))?;
    app.update();
    for pane in panes {
        assert_eq!(
            app.world()
                .get::<ScrollPosition>(pane)
                .ok_or("pane replaced")?
                .0
                .y,
            50.
        );
    }
    assert!(
        app.world().get::<Control>(weapon).is_some(),
        "focused control replaced"
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(weapon));
    assert!(
        texts(&mut app)
            .iter()
            .any(|text| text.contains("175 / 400 XP"))
    );
    Ok(())
}

#[test]
fn proficiency_preview_does_not_commit_and_close_offers_discard() -> Result {
    let (mut app, mut commands, _) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    let label = app
        .world_mut()
        .query::<(Entity, &Text)>()
        .iter(app.world())
        .find_map(|(e, t)| (t.0 == "Discard draft and close").then_some(e))
        .ok_or("discard action missing")?;
    let entity = app.world().get::<ChildOf>(label).ok_or("parent")?.parent();
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .weapon_proficiency_is_open()
    );
    Ok(())
}

#[test]
fn proficiency_applies_a_reviewed_build_only_after_each_server_confirmation() -> Result {
    use oteryn_client_runtime::weapon_proficiency::{SelectPerkReply, SelectPerkResult};
    let (mut app, mut commands, outbox) = app()?;
    publish(&outbox, &catalogue(300))?;
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Weapon(0)))?;
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Perk(0, 1)))?;
    app.update();
    click(&mut app, |kind| matches!(kind, Kind::Perk(1, 0)))?;
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, |kind| matches!(kind, Kind::Apply))?;
    let first = command(&mut commands)?;
    assert_eq!(
        SelectPerkRequest::decode(&first.payload)
            .ok_or("first perk")?
            .level,
        0
    );
    assert!(commands.try_recv().is_err());
    for (id, selections) in [(2, vec![Some(1), None]), (3, vec![Some(1), Some(0)])] {
        outbox.bevy_proficiency_result(&GatedCommandResult {
            command_id: id,
            server_sequence: id,
            status: CommandStatus::Accepted,
            payload: SelectPerkReply {
                result: SelectPerkResult::Accepted,
                selections,
            }
            .encode()
            .ok_or("perk reply")?,
        });
        app.update();
        if id == 2 {
            let next = command(&mut commands)?;
            assert_eq!(
                SelectPerkRequest::decode(&next.payload)
                    .ok_or("next perk")?
                    .level,
                1
            );
        }
        assert!(commands.try_recv().is_err());
    }
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Draft confirmed by the server.")
    );
    Ok(())
}

#[test]
fn restored_proficiency_preview_allows_switch_and_refresh_then_apply() -> Result {
    for refresh in [false, true] {
        let (mut app, mut commands, outbox) = app()?;
        click(&mut app, |k| matches!(k, Kind::Weapon(0)))?;
        app.update();
        click(&mut app, |k| matches!(k, Kind::Perk(0, 1)))?;
        app.update();
        click(&mut app, |k| matches!(k, Kind::Perk(0, 0)))?;
        app.update();
        if refresh {
            click(&mut app, |k| matches!(k, Kind::Refresh))?;
            assert_eq!(command(&mut commands)?.kind, 660);
        } else {
            click(&mut app, |k| matches!(k, Kind::Weapon(1)))?;
        }
        let mut page = catalogue(250);
        page.entries[1].experience = 250;
        publish(&outbox, &page)?;
        app.update();
        click(&mut app, |k| {
            if refresh {
                matches!(k, Kind::Perk(0, 1))
            } else {
                matches!(k, Kind::Perk(0, 0))
            }
        })?;
        app.update();
        click(&mut app, |k| matches!(k, Kind::Apply))?;
        let sent = command(&mut commands)?;
        let selection = SelectPerkRequest::decode(&sent.payload).ok_or("perk request")?;
        assert_eq!(selection.definition.get(), if refresh { 258 } else { 259 });
    }
    Ok(())
}
