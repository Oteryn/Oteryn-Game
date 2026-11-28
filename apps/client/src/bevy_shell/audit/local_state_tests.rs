use super::{AuditOutput, HostOwner, Phase, write};
use crate::{
    bevy_shell::{Options, health_circle, owner::ActivePlay},
    play::{MinimapNavigation, PlayView, play_channel},
};
use bevy::{prelude::*, ui::Checked, window::PrimaryWindow};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorVitals,
    spell_catalog::{SpellCatalogEntry, encode_catalog},
};
use serde_json::{Value, json};
use std::{num::NonZeroU32, path::Path};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn audit_app(root: &Path, enabled: bool) -> Result<App, Box<dyn std::error::Error>> {
    let mut args = vec![
        "--smoke".into(),
        "--ui-assets".into(),
        root.join("assets").into(),
    ];
    if enabled {
        args.extend(["--ui-audit".into(), root.join("audit.json").into()]);
    }
    args.extend(["--preferences".into(), root.join("preferences.json").into()]);
    let options = Options::parse(args)?;
    let mut app = App::new();
    app.insert_non_send(HostOwner::new(options.preferences)?)
        .insert_resource(AuditOutput::new(options.ui_audit))
        .add_systems(PostUpdate, write);
    health_circle::install(&mut app);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    Ok(app)
}

fn enter(app: &mut App, root: &Path, cast_index: u32) -> TestResult {
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.enter_character(Some([1; 16]));
    let (link, _commands, outbox) = play_channel();
    outbox.bevy_hud_fixture(
        &[],
        Some(ActorVitals {
            health: 25,
            max_health: 100,
            mana: 10,
            max_mana: 20,
            ..default()
        }),
    );
    let catalog = encode_catalog(&[SpellCatalogEntry {
        cast_index,
        name: "Heal".into(),
        words: "exura".into(),
        level: 1,
        mana: 0,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![],
        rune_item: None,
    }])
    .map_err(|error| format!("{error:?}"))?;
    crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
        &outbox, catalog,
    )?;
    let mut view = PlayView::new(TileCoord::new(100, 200), 7, &[])?;
    view.bind_test_minimap(root.join("minimap"));
    view.navigate_minimap(MinimapNavigation::Centre)?;
    owner.settings.sync_bar_actions(&link);
    owner.settings.sync_archive_bindings(&link);
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    Ok(())
}

fn read_local_state(app: &mut App, root: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    app.update();
    let snapshot: Value = serde_json::from_slice(&std::fs::read(root.join("audit.json"))?)?;
    assert!(snapshot["local_state"].is_object(), "missing local_state");
    Ok(snapshot["local_state"].clone())
}

#[test]
fn audit_local_state_survives_reopening_the_client_config() -> TestResult {
    use crate::play::BarAction;

    let config = tempfile::tempdir()?;
    let root = config.path();
    let mut app = audit_app(root, true)?;
    enter(&mut app, root, 7)?;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.open_graphics();
        let mut draft = owner.preferences_draft().ok_or("options draft")?.clone();
        draft.rebind("local-spell-action-bar-spell-5", "SUPER+ALT+F12")?;
        owner.edit_preferences(|prefs| {
            *prefs = draft;
            prefs.graphics.vsync = false;
            prefs.chat_drag_locked = true;
            prefs.confirm_store_purchase = false;
            prefs.arcs.health = true;
            prefs.arcs.mana = true;
            prefs.health_circle = true;
            prefs.player_hud_enabled = true;
            prefs.action_bar.visible = true;
            prefs.screenshots.directory = root
                .with_file_name("private-audit-path")
                .to_string_lossy()
                .into_owned();
            prefs.bar_layout.rows[3].enabled = true;
        });
        owner.apply(true);
        let ActivePlay { mut view, link } = owner.play.take().ok_or("play")?;
        owner.settings.sync_bar_actions(&link);
        assert!(link.assign_bar_action(
            0,
            BarAction::ToggleContainer {
                definition: NonZeroU32::MIN
            }
        ));
        assert!(link.assign_bar_action(
            1,
            BarAction::Text {
                text: "PRIVATE_ACTION_TEXT".into(),
                send_automatically: true,
            }
        ));
        assert!(link.assign_spell_slot(4, Some(7)));
        owner.settings.sync_bar_actions(&link);
        owner.settings.sync_archive_bindings(&link);
        view.edit_minimap_marker((100, 200, 7), "PRIVATE_MAP_LABEL".into(), 1)?;
        owner.play = Some(ActivePlay { view, link });
        owner.open_graphics();
        owner.edit_preferences(|prefs| prefs.graphics.vsync = true);
    }

    app.insert_resource(crate::bevy_shell::game_ui::options::Installed);
    let dialog = app
        .world_mut()
        .spawn((
            Name::new("options.dialog"),
            Node::default(),
            InheritedVisibility::VISIBLE,
        ))
        .id();
    app.world_mut().spawn((
        Name::new("options.page.Graphics"),
        Node::default(),
        InheritedVisibility::VISIBLE,
        Checked,
        ChildOf(dialog),
    ));
    // Capture the first file, then reopen before asserting the new contract.
    app.update();
    let before: Value = serde_json::from_slice(&std::fs::read(root.join("audit.json"))?)?;
    drop(app);
    std::fs::remove_file(root.join("audit.json"))?;

    let mut reopened = audit_app(root, true)?;
    enter(&mut reopened, root, 11)?;
    let after = read_local_state(&mut reopened, root)?;
    let before = &before["local_state"];
    assert_eq!(before["options_page"], "Graphics");
    assert!(after["options_page"].is_null());
    assert_eq!(after["settings"]["graphics"]["vsync"], false);
    assert_eq!(
        after["settings"]["hotkeys"]["action_keys"]["4"],
        "SUPER+ALT+F12"
    );
    assert_eq!(after["settings"]["interface"]["chat_drag_locked"], true);
    assert_eq!(after["settings"]["confirm_store_purchase"], false);
    assert_eq!(
        after["arcs"],
        json!({
            "health": { "shown": true, "fill": 0.25 },
            "mana": { "shown": true, "fill": 0.5 },
        })
    );
    let world = reopened.world_mut();
    let segment_count = world.query::<&Sprite>().iter(world).count();
    assert_eq!(segment_count, 16 + 32);
    assert_eq!(
        after["minimap"],
        json!({"center": [100, 200, 7], "zoom": 1, "following": true,
               "remembered_tiles": 1, "marked_tiles": 1})
    );
    assert_eq!(after["action_bar"]["visible_bars"], json!([0, 3]));
    assert_eq!(
        after["action_bar"]["slots"]["0"],
        json!({
            "kind": "ToggleContainer", "definition": 1,
        })
    );
    assert_eq!(
        after["action_bar"]["slots"]["1"],
        json!({ "kind": "Text", "send_automatically": true })
    );
    assert_eq!(
        after["action_bar"]["slots"]["4"],
        json!({"kind": "ArchiveSpell", "rune_item": null,
               "id": "f8d61e672cb85a56a06ad4532131e7501ab15787e77b2fdbc5883a416db73e51"})
    );
    for field in ["settings", "arcs", "minimap", "action_bar"] {
        assert_eq!(after[field], before[field], "{field}");
    }
    let local_json = after.to_string();
    for private in [
        "private-audit-path",
        "PRIVATE_ACTION_TEXT",
        "PRIVATE_MAP_LABEL",
    ] {
        assert!(!local_json.contains(private));
    }
    Ok(())
}

#[test]
fn audit_minimap_counts_imported_settings_memory() -> TestResult {
    use crate::play::{MinimapMark, PreferenceStore};
    use oteryn_session::{
        ActorPosition,
        world_map::{MapViewHeader, WorldMapViewSnapshot},
    };

    let config = tempfile::tempdir()?;
    let root = config.path();
    let mut app = audit_app(root, true)?;
    enter(&mut app, root, 7)?;
    let import = root.join("import.json");
    let before_quads = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        play.view.set_live_map(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: ActorPosition {
                    x: 100,
                    y: 200,
                    floor: 7,
                },
                first_visible_floor: 7,
            },
            tiles: vec![],
        })?;
        play.view
            .edit_minimap_marker((500, 500, 7), "Cached mark".into(), 1)?;
        let mut incoming = play.view.preferences().clone();
        // The cached tile overlaps, (113, 200) is visible, and the live window hides (101, 200).
        incoming.minimap.tiles = vec![(500, 500, 7, 55), (113, 200, 7, 55), (101, 200, 7, 55)];
        for (x, y) in [(500, 500), (113, 200)] {
            incoming.minimap.add_mark(MinimapMark {
                x,
                y,
                floor: 7,
                label: "Imported mark".into(),
            })?;
        }
        PreferenceStore::export_to(&incoming, &import)?;
        play.view.scene().minimap_quad_range().len()
    };

    let before = read_local_state(&mut app, root)?;
    assert_eq!(before["minimap"]["remembered_tiles"], 1);
    assert_eq!(before["minimap"]["marked_tiles"], 1);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.import_from(&import)?;
        owner.poll([800, 600]);
        let play = owner.play.as_ref().ok_or("play")?;
        // The scene gains one terrain quad and one mark quad.
        assert_eq!(
            play.view.scene().minimap_quad_range().len(),
            before_quads + 2
        );
    }

    let after = read_local_state(&mut app, root)?;
    assert_eq!(
        (
            after["minimap"]["remembered_tiles"].as_u64(),
            after["minimap"]["marked_tiles"].as_u64()
        ),
        (Some(2), Some(2))
    );
    Ok(())
}

#[test]
fn audit_without_ui_audit_writes_nothing() -> TestResult {
    let config = tempfile::tempdir()?;
    let root = config.path();
    let mut app = audit_app(root, false)?;
    assert!(app.world().resource::<AuditOutput>().path.is_none());
    app.update();
    assert_eq!(app.world().resource::<AuditOutput>().update, 0);
    assert_eq!(std::fs::read_dir(root)?.count(), 0);
    Ok(())
}
