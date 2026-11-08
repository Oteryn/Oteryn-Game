#[test]
fn settings_rebind_conflicts_routes_and_survives_restart() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, preference_bindings};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let path = std::env::temp_dir().join(format!("oteryn-c25-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = PreferenceStore::load(path.clone())?;
    let bindings = preference_bindings()?;
    let highscores = "0009-highscores-open";
    assert!(bindings.iter().any(|binding| binding.id == highscores));
    let mut changed = store.preferences().clone();
    assert!(changed.rebind(highscores, "Ctrl+L").is_err());
    assert!(changed.rebind(highscores, "F10").is_err());
    changed.rebind(highscores, "Alt+H")?;
    changed.ui_scale = 125;
    store.apply(changed)?;
    let restored = PreferenceStore::load(path.clone())?;
    assert_eq!(restored.preferences(), store.preferences());
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    let routed = restored.preferences().route_input(&[event])?;
    assert!(oteryn_client_ui::highscores::open_key(&routed));
    let old = NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    assert!(restored.preferences().route_input(&[old])?.is_empty());
    let mut play = view()?;
    play.set_preferences(restored.preferences().clone())?;
    assert_eq!(play.scene().view().tile_px(), 49);
    play.set_world_viewport([600, 330])?;
    assert_eq!(play.scene().view().tile_px(), 30);
    assert_eq!(play.preferences().ui_scale, 125);
    std::fs::remove_file(path)?;
    Ok(())
}
