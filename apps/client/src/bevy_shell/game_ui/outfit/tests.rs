use super::*;
use oteryn_client_runtime::appearance::{Catalogue, Context, OutfitOption};
fn panel() -> Panel {
    Panel {
        selected: true,
        fields: [
            "knight".into(),
            "0".into(),
            String::new(),
            "0,0,0,0".into(),
            "revision".into(),
        ],
        catalogue: Some(Catalogue {
            context: Context {
                content_generation: [1; 32],
                cosmetics_digest: [2; 32],
            },
            root_revision: 1,
            selected: None,
            palette: vec![[128, 64, 32]; 133],
            outfits: vec![
                OutfitOption {
                    key: "knight".into(),
                    look_type: 128,
                    permitted_addons: 1,
                },
                OutfitOption {
                    key: "mage".into(),
                    look_type: 130,
                    permitted_addons: 3,
                },
            ],
            mounts: vec!["horse".into()],
        }),
        ..Default::default()
    }
}
#[test]
fn outfit_list_and_addons() {
    let mut p = panel();
    assert_eq!(outfits(&p), vec!["knight", "mage"]);
    assert!(!edit(&mut p, &Kind::Addon(2)));
    assert!(edit(&mut p, &Kind::Addon(1)));
    assert_eq!(p.fields[1], "1");
    assert!(edit(&mut p, &Kind::Outfit("mage".into())));
    assert!(edit(&mut p, &Kind::Addon(2)));
    assert_eq!(p.fields[1], "3");
}
#[test]
fn outfit_colour_parts() {
    assert_eq!(PARTS, ["Head", "Primary", "Secondary", "Detail"]);
    let mut p = panel();
    for part in 0..4 {
        assert!(edit(&mut p, &Kind::Colour(part, 132)));
    }
    assert_eq!(p.fields[3], "132,132,132,132");
    assert!(!edit(&mut p, &Kind::Colour(0, 133)));
}
#[test]
fn outfit_preview_uses_selection() {
    let mut p = panel();
    assert!(edit(&mut p, &Kind::Outfit("mage".into())));
    assert_eq!(p.preview().map(|v| v.look_type), Some(130));
    assert!(edit(&mut p, &Kind::Mount("horse".into())));
    assert_eq!(p.fields[2], "horse");
}
#[test]
fn outfit_ok_maps_apply() {
    assert_eq!(
        intent(&Kind::Ok),
        Some(oteryn_client_ui::appearance::Intent::Apply)
    );
    assert_eq!(intent(&Kind::Close), None);
}
#[test]
fn outfit_ok_sends_intent() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = crate::play::play_channel();
    outbox.bevy_outfit_test_panel(panel());
    send(&link, &Kind::Ok);
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("appearance command missing".into());
    };
    assert_eq!(
        command.kind,
        oteryn_client_runtime::appearance::COMMAND_TYPE_APPEARANCE_CHANGE
    );
    let change = oteryn_client_runtime::appearance::Change::decode(&command.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(change.selection.outfit_key, "knight");
    assert_eq!(change.selection.colours, [0; 4]);
    send(&link, &Kind::Ok);
    assert!(receiver.try_recv().is_err());
    Ok(())
}
#[test]
fn outfit_shortcut_follows_the_rebound_chord_and_the_input_fence()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::{
        bevy_shell::owner::{ActivePlay, HostOwner},
        play::{PlayView, play_channel},
    };
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let key = |code: u16,
               modifiers: Modifiers|
     -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        })
    };
    let mut owner = HostOwner::new(None)?;
    let (link, _commands, _outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner
        .settings
        .preferences
        .rebind("0025-appearance-open", "CTRL+ALT+SHIFT+L")?;
    let open = |owner: &HostOwner| {
        owner
            .play
            .as_ref()
            .is_some_and(|p| p.link.appearance_panel().open)
    };
    let default_chord = key(4, Modifiers::one(Modifier::Control).with(Modifier::Alt))?;
    let rebound = key(
        15,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    owner.route_input(std::slice::from_ref(&default_chord), false);
    assert!(!open(&owner), "the old default chord was rebound away");
    owner.route_input(std::slice::from_ref(&rebound), true);
    assert!(
        !open(&owner),
        "blocked input (focused editor or modal window) must not open it"
    );
    owner.route_input(&[rebound], false);
    assert!(open(&owner), "the rebound chord opens the outfit window");
    Ok(())
}

#[test]
fn outfit_escape_respects_foreground_modal() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .insert_non_send(HostOwner::new(None)?)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    let (link, _, outbox) = crate::play::play_channel();
    let mut p = panel();
    p.open = true;
    outbox.bevy_outfit_test_panel(p);
    app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    app.init_resource::<ButtonInput<KeyCode>>();
    install(&mut app);
    app.update();
    let top = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .open
    );
    app.world_mut().entity_mut(top).despawn();
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .open
    );
    Ok(())
}

#[test]
fn outfit_cancel_and_escape_restore_confirmed_selection() -> Result<(), Box<dyn std::error::Error>>
{
    for escape in [false, true] {
        let mut p = panel();
        p.open = true;
        p.confirmed = Some(oteryn_client_runtime::appearance::Selected {
            selection: oteryn_client_runtime::appearance::Selection {
                outfit_key: "knight".into(),
                colours: [0; 4],
                addons: 0,
                mount_key: None,
            },
            revision: 1,
            provenance: "confirmed".into(),
        });
        let expected = p.fields.clone();
        let (link, _, outbox) = crate::play::play_channel();
        outbox.bevy_outfit_test_panel(p);
        let mut owner = HostOwner::new(None)?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_non_send(owner)
            .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        app.update();
        {
            let owner = app.world().non_send::<HostOwner>();
            let link = &owner.play.as_ref().ok_or("play")?.link;
            for (field, value) in ["mage", "3", "horse", "1,2,3,4"].iter().enumerate() {
                link.appearance_edit(field, (*value).into());
            }
        }
        if escape {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::Escape);
            app.update();
        } else {
            let button = app
                .world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(entity, action)| matches!(action.kind, Kind::Close).then_some(entity))
                .ok_or("cancel")?;
            app.world_mut().trigger(Activate { entity: button });
        }
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(!link.appearance_panel().open);
        assert_eq!(link.appearance_panel().fields, expected);
        link.appearance_open();
        assert_eq!(link.appearance_panel().fields, expected);
    }
    Ok(())
}
