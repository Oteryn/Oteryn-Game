use super::*;
use oteryn_client_ui::imbue::{ImbuePanel, imbuement};
use std::num::NonZeroU64;

fn panel() -> ImbuePanel {
    ImbuePanel {
        selected: true,
        open: true,
        tracker_open: true,
        item: Some(NonZeroU64::MIN),
        success_selected: true,
        success: vec![73],
        window: Some(imbuement::ImbuementWindow {
            slot_count: 2,
            clear_gold: 19,
            offers: vec![imbuement::ImbuementOffer {
                family: 1,
                tier: 3,
                gold: 1234,
                requires_premium: false,
            }],
            applied: vec![imbuement::AppliedImbuement {
                slot: 1,
                family: 12,
                tier: 2,
                remaining_ms: 4321000,
            }],
        }),
        ..Default::default()
    }
}
#[test]
fn imbue_window_shows_server_offers_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = panel();
    p.details = Some(oteryn_client_runtime::imbuement_details::Reply {
        item: NonZeroU64::MIN,
        quotes: vec![oteryn_client_runtime::imbuement_details::Quote {
            family: 1,
            tier: 3,
            gold: 1234,
            requires_premium: false,
            recipe_available: true,
            success_percent: 100,
            materials: vec![oteryn_client_runtime::imbuement_details::Material {
                definition_key: "oteryn:item.rope".into(),
                quantity: 17,
            }],
        }],
    });
    let text = offer_text(&p, 0);
    assert!(text.contains("17 × oteryn:item.rope"));
    assert!(text.contains("tier 3"));
    assert!(text.contains("1234"));
    assert!(text.contains("73%"));
    assert_rendered(
        p.clone(),
        &[
            "tier 3",
            "1234",
            "73%",
            "17 × oteryn:item.rope",
            "Slot 1: empty",
        ],
    )?;
    assert!(offer_text(&p, 1).is_empty());
    p.success.clear();
    p.details = None;
    assert!(!offer_text(&p, 0).contains("100%"));
    assert!(offer_text(&p, 0).contains("unavailable"));
    Ok(())
}
#[test]
fn imbue_tracker_shows_server_durations() -> Result<(), Box<dyn std::error::Error>> {
    let p = panel();
    let text = tracker_text(&p);
    assert!(text.contains("4321 seconds"));
    assert!(text.contains("Void"));
    assert!(!tracker_text(&ImbuePanel::default()).contains("4321"));
    assert_rendered(p, &["4321 seconds", "Void"])?;
    Ok(())
}

#[test]
fn imbue_modal_confirmation_and_generation_fence() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    let p = panel();
    outbox.bevy_imbue_fixture(p.clone());
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let find = |app: &mut App, confirm: bool| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(e, c)| {
                (if confirm {
                    matches!(c.action, Action::Owner(Kind::Apply))
                } else {
                    matches!(c.action, Action::Confirm(true))
                })
                .then_some(e)
            })
            .ok_or("button".into())
    };
    let apply = find(&mut app, false)?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: apply });
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    let confirmed = find(&mut app, true)?;
    assert!(commands.try_recv().is_err());
    app.world_mut().trigger(Activate { entity: confirmed });
    assert!(matches!(
        commands.try_recv(),
        Ok(crate::combat_input::PlayCommand::Imbue(_))
    ));
    app.world_mut().trigger(Activate { entity: confirmed });
    assert!(commands.try_recv().is_err());
    outbox.bevy_imbue_fixture(p.clone());
    outbox.bevy_imbue_end();
    app.world_mut().non_send_mut::<HostOwner>().poll([960, 640]);
    let (link, mut replacement, replacement_outbox) = play_channel();
    replacement_outbox.bevy_imbue_fixture(p);
    app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    app.world_mut().trigger(Activate { entity: confirmed });
    assert!(replacement.try_recv().is_err());
    Ok(())
}

fn assert_rendered(panel: ImbuePanel, expected: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    let (link, _commands, outbox) = play_channel();
    outbox.bevy_imbue_fixture(panel);
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in expected {
        assert!(text.contains(needle), "missing {needle}: {text}");
    }
    Ok(())
}

fn review_fixture(
    p: ImbuePanel,
) -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    use crate::play::{PlayView, play_channel};
    use bevy::{asset::AssetPlugin, input_focus::InputFocus};
    let (link, commands, outbox) = play_channel();
    outbox.bevy_imbue_fixture(p);
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands))
}

#[test]
fn imbue_review_missing_cost_keeps_cancel_only() -> Result<(), Box<dyn std::error::Error>> {
    for missing_window in [false, true] {
        let mut p = panel();
        if missing_window {
            p.window = None;
        } else {
            p.offer = 1;
        }
        let (mut app, mut commands) = review_fixture(p)?;
        let owner = app.world().non_send::<HostOwner>();
        let generation = owner.generation();
        let snapshot = owner.play.as_ref().ok_or("play")?.link.imbue_snapshot();
        app.world_mut().resource_mut::<State>().confirmation = Some((generation, snapshot, true));
        app.world_mut().resource_mut::<State>().shown = None;
        app.update();
        let controls: Vec<_> = app
            .world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .map(|(e, c)| (e, c.action.clone()))
            .collect();
        assert_eq!(controls.len(), 1, "missing cost must leave exactly Cancel");
        let cancel = controls
            .iter()
            .find_map(|(e, a)| matches!(a, Action::Cancel).then_some(*e))
            .ok_or("Cancel missing")?;
        app.world_mut().trigger(Activate { entity: cancel });
        assert!(app.world().resource::<State>().confirmation.is_none());
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn imbue_review_escape_closes_only_top_window() -> Result<(), Box<dyn std::error::Error>> {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, mut commands) = review_fixture(panel())?;
    let escape = |app: &mut App, repeat| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: ButtonState::Pressed,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    };
    let get_panel = |app: &App| -> Result<ImbuePanel, Box<dyn std::error::Error>> {
        Ok(app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .imbue_panel())
    };
    escape(&mut app, true);
    assert!(get_panel(&app)?.open);
    let apply = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control.action, Action::Confirm(true)).then_some(entity)
        })
        .ok_or("Apply missing")?;
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_some());
    escape(&mut app, false);
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(get_panel(&app)?.open);
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    escape(&mut app, false);
    assert!(get_panel(&app)?.open);
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    escape(&mut app, false);
    assert!(!get_panel(&app)?.open);
    assert!(get_panel(&app)?.tracker_open);
    escape(&mut app, false);
    assert!(!get_panel(&app)?.tracker_open);
    assert!(commands.try_recv().is_err());
    Ok(())
}
