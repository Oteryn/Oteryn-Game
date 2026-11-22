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
