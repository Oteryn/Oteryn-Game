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
fn outfit_apply_explains_missing_server_content_revision_without_sending()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut receiver, outbox) = crate::play::play_channel();
    let mut value = panel();
    value.fields[4].clear();
    outbox.bevy_outfit_test_panel(value.clone());
    send(&link, &Kind::Colour(0, 132));
    send(&link, &Kind::Ok);
    let after = link.appearance_panel();
    assert!(!after.pending);
    assert!(receiver.try_recv().is_err());
    assert_eq!(after.fields[3], "132,0,0,0");
    assert_eq!(after.catalogue, value.catalogue);
    assert_eq!(
        after.feedback,
        "Appearance changes are unavailable: the server has not supplied the required outfit data"
    );
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
