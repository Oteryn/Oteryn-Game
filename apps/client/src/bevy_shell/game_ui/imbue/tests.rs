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
