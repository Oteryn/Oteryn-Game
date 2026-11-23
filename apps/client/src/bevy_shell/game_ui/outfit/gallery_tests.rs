use super::*;
fn click_caption(app: &mut App, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(caption, parent)| (caption.0 == name).then_some(parent.parent()))
        .ok_or_else(|| format!("missing outfit button {name}"))?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    finish_draw(app)?;
    Ok(())
}

#[test]
fn outfit_preview_layers_change_pixels_without_changing_selection()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _) = editor_app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    {
        let mut state = app.world_mut().resource_mut::<State>();
        state.artwork = Some(preview::Assets::load(&root)?);
        state.dirty = true;
    }
    app.update();
    click_caption(&mut app, "Movement off")?;
    let pixels = |app: &App| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let handle = app
            .world()
            .resource::<State>()
            .image
            .as_ref()
            .ok_or("preview handle")?;
        Ok(app
            .world()
            .resource::<Assets<Image>>()
            .get(handle)
            .ok_or("preview image")?
            .data
            .clone()
            .ok_or("pixels")?)
    };
    let original = pixels(&app)?;
    click_caption(&mut app, "Hide character")?;
    assert_ne!(pixels(&app)?, original);
    click_caption(&mut app, "Show character")?;
    assert_eq!(pixels(&app)?, original);
    click_caption(&mut app, "Show floor")?;
    assert_ne!(pixels(&app)?, original);
    click_caption(&mut app, "Hide floor")?;
    assert_eq!(pixels(&app)?, original);
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .fields,
        panel().fields
    );
    Ok(())
}

#[test]
fn preset_buttons_create_update_apply_rename_and_delete() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut app, _) = editor_app()?;
    click_caption(&mut app, "Create preset")?;
    click_caption(&mut app, "mage")?;
    click_caption(&mut app, "Update preset")?;
    click_caption(&mut app, "knight")?;
    click_caption(&mut app, "New Preset")?;
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .fields[0],
        "mage"
    );
    let editor = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "outfit.preset.name").then_some(entity))
        .ok_or("preset name")?;
    app.world_mut()
        .get_mut::<bevy::text::EditableText>(editor)
        .ok_or("editor")?
        .editor_mut()
        .set_text("Walking");
    click_caption(&mut app, "Rename preset")?;
    click_caption(&mut app, "Walking")?;
    click_caption(&mut app, "Delete preset")?;
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|label| label.0 == "Walking")
    );
    Ok(())
}

#[test]
fn outfit_gallery_shows_unowned_metadata_only_for_matching_catalogue()
-> Result<(), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let (mut app, outbox) = editor_app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut source: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/cosmetics/outfits/authoring.json"),
    )?)?;
    source["document"]["schema"] = "OTERYN_CHARACTER_COSMETICS_CANDIDATE/v1".into();
    let key = source["document"]["outfits"][0]["key"]
        .as_str()
        .ok_or("catalogue outfit")?;
    let locked = format!("{key} (not owned)");
    let mut value = panel();
    value.open = true;
    value
        .catalogue
        .as_mut()
        .ok_or("catalogue")?
        .context
        .cosmetics_digest = Sha256::digest(serde_json::to_vec(&source["document"])?).into();
    outbox.bevy_outfit_test_panel(value);
    app.world_mut().resource_mut::<State>().artwork = Some(preview::Assets::load(&root)?);
    app.update();
    click_caption(&mut app, "Show all appearances")?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == locked)
    );
    click_caption(&mut app, "Show owned appearances")?;
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == locked)
    );
    Ok(())
}

#[test]
fn outfit_preview_buttons_project_confirmed_title_and_familiar()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::{
        character_info_panel::CharacterTitle,
        cyclopedia_character::gallery::{Entry, Kind, Page},
    };
    let (mut app, outbox) = editor_app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    app.world_mut().resource_mut::<State>().artwork = Some(preview::Assets::load(&root)?);
    let title = CharacterTitle::from_id(1).ok_or("title")?;
    outbox.outfit_preview_fixture(
        title,
        Page {
            offset: 0,
            total: 1,
            entries: vec![Entry {
                kind: Kind::Familiar,
                key: "catalogue familiar".into(),
                look_type: 130,
            }],
        },
    );
    app.update();
    click_caption(&mut app, "Show title")?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == title.name())
    );
    let title_parent = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == title.name()).then_some(parent.parent()))
        .ok_or("preview title")?;
    assert!(app.world().get::<ImageNode>(title_parent).is_some());
    click_caption(&mut app, "Hide title")?;
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == title.name())
    );
    click_caption(&mut app, "Movement off")?;
    let pixels = |app: &App| -> Option<Vec<u8>> {
        let handle = app.world().resource::<State>().image.as_ref()?;
        app.world()
            .resource::<Assets<Image>>()
            .get(handle)?
            .data
            .clone()
    };
    let before = pixels(&app).ok_or("image")?;
    click_caption(&mut app, "Preview catalogue familiar")?;
    assert_ne!(pixels(&app), Some(before.clone()));
    click_caption(&mut app, "Hide familiar")?;
    assert_eq!(pixels(&app), Some(before));
    Ok(())
}

#[test]
fn mount_visibility_keeps_the_selected_mount() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, outbox) = editor_app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let artwork = preview::Assets::load(&root)?;
    let metadata = artwork.mounts.as_ref().ok_or("mount metadata")?;
    let key = "oteryn:content.mount.haze";
    let mut value = panel();
    value.open = true;
    value.fields[2] = key.into();
    let catalogue = value.catalogue.as_mut().ok_or("catalogue")?;
    catalogue.context.cosmetics_digest = metadata.digest;
    catalogue.mounts.push(key.into());
    outbox.bevy_outfit_test_panel(value);
    app.world_mut().resource_mut::<State>().artwork = Some(artwork);
    app.update();
    click_caption(&mut app, "Movement off")?;
    let pixels = |app: &App| -> Option<Vec<u8>> {
        let handle = app.world().resource::<State>().image.as_ref()?;
        app.world()
            .resource::<Assets<Image>>()
            .get(handle)?
            .data
            .clone()
    };
    let mounted = pixels(&app).ok_or("mounted image")?;
    click_caption(&mut app, "Hide mount")?;
    assert_ne!(pixels(&app), Some(mounted.clone()));
    click_caption(&mut app, "Show mount")?;
    assert_eq!(pixels(&app), Some(mounted));
    click_caption(&mut app, "Hide character")?;
    let mount_only = pixels(&app).ok_or("mount artwork")?;
    click_caption(&mut app, "Hide mount")?;
    assert_ne!(pixels(&app), Some(mount_only));
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .appearance_panel()
            .fields[2],
        key
    );
    Ok(())
}

#[test]
fn review_preview_redraw_preserves_the_unsubmitted_preset_name()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _) = editor_app()?;
    click_caption(&mut app, "Create preset")?;
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<presets::NameField>>()
        .single(app.world())?;
    app.world_mut()
        .get_mut::<bevy::text::EditableText>(entity)
        .ok_or("editor")?
        .editor_mut()
        .set_text("Travelling");
    click_caption(&mut app, "Show title")?;
    click_caption(&mut app, "Rename preset")?;
    click_caption(&mut app, "Travelling")
}

#[test]
fn accepting_a_preset_after_clearing_its_mount_keeps_the_actor_on_foot()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
        actor_motion::ActorMotion,
    };
    let (mut app, _) = editor_app()?;
    let (link, mut receiver, outbox) = crate::play::play_channel();
    let mut draft = panel();
    draft.open = true;
    draft.fields[2] = "horse".into();
    outbox.bevy_outfit_test_panel(draft.clone());
    outbox.bevy_mount_test_actor(WorldSpatialEntity {
        motion: Some(ActorMotion::default()),
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    });
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play.as_mut().ok_or("play")?.link = link;
        owner.settings.preferences.outfit_presets.insert(
            "Mounted".into(),
            (std::array::from_fn(|i| draft.fields[i].clone()), Some(true)),
        );
    }
    app.update();
    click_caption(&mut app, "Mounted")?;
    click_caption(&mut app, "No mount")?;
    click_caption(&mut app, "OK")?;
    let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("missing appearance change".into());
    };
    let change = oteryn_client_runtime::appearance::Change::decode(&command.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert!(change.selection.mount_key.is_none());
    assert!(
        receiver.try_recv().is_err(),
        "clearing the mount cancels the riding request"
    );
    Ok(())
}

#[test]
fn deleting_a_preset_preserves_riding_until_the_appearance_is_confirmed()
-> Result<(), Box<dyn std::error::Error>> {
    for accepted in [false, true] {
        use oteryn_session::{
            ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
            actor_motion::ActorMotion,
        };
        let (mut app, _) = editor_app()?;
        let (link, mut receiver, outbox) = crate::play::play_channel();
        let mut draft = panel();
        draft.open = true;
        draft.fields[2] = "horse".into();
        outbox.bevy_outfit_test_panel(draft.clone());
        outbox.bevy_mount_test_actor(WorldSpatialEntity {
            motion: Some(ActorMotion::default()),
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        });
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            owner.play.as_mut().ok_or("play")?.link = link;
            owner.settings.preferences.outfit_presets.insert(
                "Mounted".into(),
                (std::array::from_fn(|i| draft.fields[i].clone()), Some(true)),
            );
        }
        app.update();
        click_caption(&mut app, "Mounted")?;
        click_caption(&mut app, "Delete preset")?;
        click_caption(&mut app, "OK")?;
        let crate::combat_input::PlayCommand::Feature(command) = receiver.try_recv()? else {
            return Err("appearance request".into());
        };
        assert_eq!(command.capability, 25);
        assert!(
            receiver.try_recv().is_err(),
            "riding changed before appearance acknowledgement"
        );
        let result = if accepted {
            oteryn_client_runtime::appearance::ChangeResult::Committed(2)
        } else {
            oteryn_client_runtime::appearance::ChangeResult::Rejected
        };
        outbox.bevy_properties_result(
            &command,
            &oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: result.encode().map_err(|error| format!("{error:?}"))?,
            },
        );
        app.update();
        assert_eq!(
            matches!(receiver.try_recv(), Ok(crate::combat_input::PlayCommand::Feature(request)) if request.capability == 502),
            accepted
        );
    }
    Ok(())
}
