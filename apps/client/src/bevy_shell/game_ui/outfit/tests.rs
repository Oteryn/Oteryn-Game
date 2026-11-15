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
fn outfit_preview_rotates_verified_artwork() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut assets = preview::Assets::load(&root)?;
    let selection = panel();
    let south = assets.draw_panel(&selection, 2, 0, false)?;
    let east = assets.draw_panel(&selection, 1, 0, false)?;
    assert!(south != east, "rotation must change the displayed sprite");
    Ok(())
}

#[test]
fn outfit_preview_buttons_animate_the_retained_image_without_sending_a_change()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::owner::ActivePlay;
    use std::time::Duration;
    let mut owner = HostOwner::new(None)?;
    let generation = owner.generation();
    let (link, mut receiver, _) = crate::play::play_channel();
    owner.play = Some(ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let artwork = preview::Assets::load(&root)?;
    let mut images = Assets::<Image>::default();
    let handle = images.add(Image::default());
    let mut app = App::new();
    app.insert_non_send(owner)
        .insert_resource(images)
        .insert_resource(Time::<Real>::default())
        .insert_resource(State {
            rendered: Some((generation, panel())),
            artwork: Some(artwork),
            image: Some(handle.clone()),
            ..default()
        })
        .add_systems(Update, animation::animate);
    let mut control = |kind| {
        app.world_mut()
            .spawn((
                Node::default(),
                Action {
                    generation,
                    kind: Kind::Preview(kind),
                },
            ))
            .observe(activate)
            .id()
    };
    let right = control(animation::Control::Right);
    let left = control(animation::Control::Left);
    let stop = control(animation::Control::Movement(false));
    let walk = control(animation::Control::Movement(true));
    let pixels = |app: &App| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        app.world()
            .resource::<Assets<Image>>()
            .get(&handle)
            .and_then(|image| image.data.clone())
            .ok_or_else(|| "image".into())
    };
    app.update();
    let south = pixels(&app)?;
    app.world_mut().trigger(Activate { entity: right });
    app.update();
    assert!(pixels(&app)? != south);
    app.world_mut().trigger(Activate { entity: left });
    app.update();
    assert_eq!(pixels(&app)?, south);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_millis(300));
    app.update();
    assert!(
        pixels(&app)? != south,
        "walking must advance the rendered frame"
    );
    app.world_mut().trigger(Activate { entity: stop });
    app.update();
    let standing = pixels(&app)?;
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(1));
    app.update();
    assert_eq!(pixels(&app)?, standing);
    app.world_mut().trigger(Activate { entity: walk });
    assert!(app.world().resource::<State>().playback.moving);
    assert_eq!(
        app.world().resource::<State>().image.as_ref(),
        Some(&handle)
    );
    assert_eq!(
        app.world().resource::<State>().rendered,
        Some((generation, panel()))
    );
    assert!(
        receiver.try_recv().is_err(),
        "preview controls must stay local"
    );
    app.world_mut().resource_mut::<State>().rendered = None;
    app.update();
    assert_eq!(app.world().resource::<State>().playback.direction, 2);
    Ok(())
}

#[test]
fn outfit_mount_composition_requires_owned_key_and_matching_metadata_digest()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut assets = preview::Assets::load(&root)?;
    let mut document: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/cosmetics/outfits/authoring.json"),
    )?)?;
    document["document"]["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
    let digest = Sha256::digest(serde_json::to_vec(&document["document"])?).into();
    let mut p = panel();
    let catalogue = p.catalogue.as_mut().ok_or("catalogue")?;
    catalogue.context.cosmetics_digest = digest;
    catalogue.mounts.push("oteryn:content.mount.haze".into());
    let plain = assets.draw_panel(&p, 2, 0, true)?;
    p.fields[2] = "oteryn:content.mount.haze".into();
    let mounted = assets.draw_panel(&p, 2, 0, true)?;
    assert!(
        mounted != plain,
        "the verified mount must be composed beneath the rider"
    );
    assert!(assets.draw_panel(&p, 1, 0, true)? != mounted);
    assert!(assets.draw_panel(&p, 2, 300, true)? != mounted);
    p.catalogue
        .as_mut()
        .ok_or("catalogue")?
        .context
        .cosmetics_digest = [0; 32];
    assert_eq!(
        assets.draw_panel(&p, 2, 0, true).err().as_deref(),
        Some("Mount artwork needs updating")
    );
    p.catalogue
        .as_mut()
        .ok_or("catalogue")?
        .context
        .cosmetics_digest = digest;
    p.fields[2] = "horse".into();
    assert_eq!(
        assets.draw_panel(&p, 2, 0, true).err().as_deref(),
        Some("Mount artwork unavailable")
    );
    p.fields[2] = "not-owned".into();
    assert_eq!(
        assets.draw_panel(&p, 2, 0, true).err().as_deref(),
        Some("Choose an owned mount")
    );
    Ok(())
}
