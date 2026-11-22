use super::*;
use oteryn_session::{ActorPosition, EntityDetail, EntityKind, StepDirection};

#[test]
fn review_battle_icon_actor_health_draws_and_refreshes_the_same_bar()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, outbox) = super::super::tests::app()?;
    let mut retained = None;
    let mut widths = Vec::new();
    let mut colors = Vec::new();
    for hp in [75, 20] {
        outbox.bevy_hud_fixture(
            &[WorldSpatialEntity {
                motion: None,
                display_name: None,
                kind: EntityKind::Creature,
                entity: EntityRef {
                    identity: [2; 16],
                    generation: 8,
                },
                position: ActorPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
                detail: EntityDetail::ActorWithIcons {
                    direction: StepDirection::South,
                    appearance_ref: 1,
                    health_percent: hp,
                    icons: Default::default(),
                },
            }],
            None,
        );
        app.update();
        let mut fills = app
            .world_mut()
            .query_filtered::<(Entity, &Node, &BackgroundColor), With<HealthFill>>();
        let (bar, node, color) = fills.single(app.world())?;
        widths.push(node.width);
        colors.push(color.0);
        assert_eq!(*retained.get_or_insert(bar), bar);
    }
    assert_eq!(widths, [percent(75.), percent(20.)]);
    assert_eq!(
        colors,
        [75, 20].map(|hp| crate::bevy_shell::health_circle::health_colour(hp, 100))
    );
    Ok(())
}

#[test]
fn battle_menu_cycles_targets_and_collapses_filters() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, _) = super::super::tests::app()?;
    let menu = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(e, a)| matches!(a.command, Command::BattleMenu).then_some(e))
        .ok_or("battle menu")?;
    app.world_mut().trigger(Activate { entity: menu });
    app.update();
    let entry = |app: &mut App, caption: &str| -> Option<Entity> {
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
    };
    let toggle = entry(&mut app, "Hide filters").ok_or("hide filters")?;
    app.world_mut().trigger(Activate { entity: toggle });
    app.update();
    assert!(
        !app.world_mut()
            .query::<&HudAction>()
            .iter(app.world())
            .any(|a| matches!(a.command, Command::BattleFilter(_)))
    );
    let first = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .battle_panel()
        .rows[0];
    let mut last = first;
    last.entity.identity = [7; 16];
    for (caption, current, expected) in [
        ("Next target", None, first.entity),
        ("Next target", Some(first.entity), last.entity),
        ("Next target", Some(last.entity), first.entity),
        ("Previous target", Some(first.entity), last.entity),
        ("Previous target", Some(last.entity), first.entity),
    ] {
        let (mut app, mut commands, outbox) = super::super::tests::app()?;
        let menu = app
            .world_mut()
            .query::<(Entity, &HudAction)>()
            .iter(app.world())
            .find_map(|(id, a)| matches!(a.command, Command::BattleMenu).then_some(id))
            .ok_or("menu")?;
        app.world_mut().trigger(Activate { entity: menu });
        outbox.bevy_hud_fixture(&[first, last], None);
        if let Some(current) = current {
            outbox.bevy_hud_target(current);
        }
        app.update();
        let button = entry(&mut app, caption).ok_or("target action")?;
        app.world_mut().trigger(Activate { entity: button });
        assert!(
            matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::Battle(crate::combat_input::CombatCommand::Attack(target))) if target == expected)
        );
    }
    Ok(())
}

#[test]
fn primary_and_secondary_battle_lists_show_their_live_rows_together()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, _) = super::super::tests::app()?;
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let mut lists = host.settings.preferences.battle_lists.clone();
        let mut secondary = lists[0].clone();
        secondary.name = "Hunt".to_owned();
        lists.push(secondary);
        host.settings.preferences.battle_lists.clone_from(&lists);
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .sync_battle_lists(&mut lists, 0);
    }
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&HealthFill>()
            .iter(app.world())
            .count(),
        2
    );
    Ok(())
}

#[test]
fn battle_rows_show_skull_and_emblem_and_hover_marks_the_actor()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _, outbox) = super::super::tests::app()?;
    let row = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(e, a)| {
            if let Command::Attack(target) = a.command {
                Some((e, target))
            } else {
                None
            }
        })
        .ok_or("attack row")?;
    outbox.battle_marks_fixture(row.1);
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|_| "tick")?;
    }
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("White skull") && t.0.contains("Enemy"))
    );
    let entry = app
        .world_mut()
        .query::<(Entity, &HudAction)>()
        .iter(app.world())
        .find_map(|(e, a)| matches!(a.command, Command::Attack(_)).then_some(e))
        .ok_or("marked row")?;
    app.world_mut()
        .entity_mut(entry)
        .insert(bevy::picking::hover::Hovered(true));
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|n| n.as_str() == "Battle hover border")
            .count(),
        4
    );
    Ok(())
}

#[test]
fn battle_portrait_reads_real_outfit_cells_from_the_scene_atlas()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::{game_ui::viewport::WorldViewport, scene};
    let (mut app, _, outbox) = super::super::tests::app()?;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        let actor = play.link.battle_panel().rows[0];
        play.view
            .prime_icon_assets(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        play.view
            .set_presentations(vec![oteryn_session::VisibleAppearance {
                entity: actor,
                look: Some(oteryn_session::Look::Outfit {
                    look_type: 128,
                    colours: [0; 4],
                    addons: 0,
                }),
                light: None,
                forge_stacks: None,
            }])?;
        outbox.bevy_hud_fixture(&[actor], None);
    }
    app.init_resource::<scene::SceneCache>()
        .init_resource::<WorldViewport>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .add_systems(Update, scene::project);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    app.update();
    let atlas = app
        .world()
        .resource::<scene::SceneCache>()
        .atlas_image()
        .ok_or("world atlas")?
        .clone();
    assert!(
        app.world_mut()
            .query::<(&Name, &ImageNode)>()
            .iter(app.world())
            .any(|(n, i)| n.as_str() == "Battle portrait cell"
                && i.image == atlas
                && i.rect.is_some())
    );
    Ok(())
}
