use super::*;

#[test]
fn moved_logout_suppresses_all_original_modifier_variants() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::ReferencePreferences;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    preferences.rebind("0021-logout-logout-l", "ALT+SHIFT+L")?;
    for bits in 0..16 {
        let modifiers = Modifiers::from_bits(bits)?;
        if !modifiers.contains(Modifier::Control) {
            continue;
        }
        let routed = preferences.route_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(15)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }])?;
        assert!(!crate::input::logout_key(&routed), "modifier bits {bits}");
    }
    Ok(())
}

#[test]
fn t010_hud_layout_is_validated_and_restored() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let mut json = serde_json::to_value(play.preferences())?;
    json["hud_bounds"] = serde_json::json!([20, 80, 640, 60]);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    assert_eq!(
        serde_json::to_value(play.preferences())?["hud_bounds"],
        json["hud_bounds"]
    );
    json["hud_bounds"] = serde_json::json!([20, 80, 0, 60]);
    assert!(play.set_preferences(serde_json::from_value(json)?).is_err());
    Ok(())
}

#[test]
fn t011_vitals_bars_reach_live_scene_and_can_be_hidden() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    play.rebuild()?;
    let shown = play.scene().sprites().instances().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["hud_bars"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().instances().len() + 4, shown);
    Ok(())
}

#[test]
fn t012_visual_preferences_control_live_effect_projection() -> Result<(), Box<dyn std::error::Error>>
{
    let mut play = view()?;
    let now = std::time::Instant::now();
    let baseline = play.scene().sprites().len();
    play.receive_presentation(
        &[oteryn_session::PresentationEventKind::MagicEffect {
            position: oteryn_session::ActorPosition {
                x: play.own.x,
                y: play.own.y,
                floor: play.floor,
            },
            effect_id: 29,
            source: oteryn_session::PresentationSource::Own,
        }],
        now,
    );
    play.project_presentation(now)?;
    assert!(play.scene().sprites().len() > baseline);
    let mut json = serde_json::to_value(play.preferences())?;
    json["show_effects"] = serde_json::json!(false);
    json["show_lights"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline);
    assert!(play.scene().sprites().lighting.sources.is_empty());
    Ok(())
}

#[test]
fn t162_monk_hud_has_independent_visibility() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        harmony: 3,
        serene: true,
        ..Default::default()
    });
    play.rebuild()?;
    let shown = play.scene().sprites().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["show_harmony"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    assert_eq!(play.scene().sprites().len() + 5, shown);
    json["show_serene"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len() + 6, shown);
    Ok(())
}

#[test]
fn o010_menu_uses_live_window_inputs_after_rebinding() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = crate::play::SettingsState::new(None);
    state
        .preferences
        .rebind("0287-actor-spell-catalog-open", "Alt+U")?;
    assert!(state.queue_window("0287-actor-spell-catalog-open"));
    assert!(!state.queue_window("unknown"));
    let raw = state.take_window_events();
    let (events, _) = state.input(&raw);
    let (link, _, _) = crate::play::play_channel();
    assert!(link.spell_catalog_input(&events));
    assert!(link.spell_catalog_panel().open);
    assert!(state.take_window_events().is_empty());
    Ok(())
}

#[test]
fn o021_live_health_and_party_marks_project_at_entity_position()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let mut json = serde_json::to_value(play.preferences())?;
    json["creature_bars"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    let entity = oteryn_session::EntityRef {
        identity: [8; 16],
        generation: 2,
    };
    play.entities.push(oteryn_session::WorldSpatialEntity {
        entity,
        display_name: None,
        kind: oteryn_session::EntityKind::Creature,
        position: oteryn_session::ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        },
        detail: oteryn_session::EntityDetail::Actor {
            direction: oteryn_session::StepDirection::South,
            appearance_ref: 1,
            health_percent: 50,
        },
    });
    play.rebuild()?;
    let baseline = play.scene().sprites().len();
    json["creature_bars"] = serde_json::json!(true);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline + 2);
    play.party_icons = vec![oteryn_client_runtime::party_skull::PartyIcon {
        entity,
        shield: 3,
        skull: 4,
    }];
    play.rebuild()?;
    assert!(play.scene().sprites().len() > baseline + 2);
    play.party_icons[0].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(play.scene().sprites().len(), baseline + 2);
    Ok(())
}

#[test]
fn o025_health_circle_tracks_confirmed_resources_and_hides_without_vitals()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let baseline = play.scene().sprites().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["health_circle"] = serde_json::json!(true);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline);
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 0,
        max_mana: 0,
        ..Default::default()
    });
    play.rebuild()?;
    // 32 circle segments, four HP/MP quads and six monk indicators.
    assert_eq!(play.scene().sprites().len(), baseline + 42);
    let circle = &play.scene().sprites().instances()[baseline + 10..];
    assert_eq!(circle.len(), 32);
    assert!(circle.iter().any(|segment| segment.uv != circle[31].uv));
    Ok(())
}

#[test]
fn o037_shortcut_panel_collapse_and_order_survive_restart() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::env::temp_dir().join(format!("cg-mainpanel-{}.json", std::process::id()));
    let mut state = crate::play::SettingsState::new(Some(path.clone()));
    let mut json = serde_json::to_value(&state.preferences)?;
    json["shortcuts_collapsed"] = serde_json::json!(true);
    json["shortcut_order"] =
        serde_json::json!(["0287-actor-spell-catalog-open", "0279-skill-progress-open"]);
    state.apply(serde_json::from_value(json.clone())?);
    let restored = crate::play::SettingsState::new(Some(path.clone()));
    assert_eq!(
        serde_json::to_value(&restored.preferences)?["shortcut_order"],
        json["shortcut_order"]
    );
    assert_eq!(
        serde_json::to_value(&restored.preferences)?["shortcuts_collapsed"],
        true
    );
    json["shortcut_order"] = serde_json::json!(["unknown"]);
    assert!(
        view()?
            .set_preferences(serde_json::from_value(json)?)
            .is_err()
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn t056_own_spell_filter_changes_live_projection() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let now = std::time::Instant::now();
    let baseline = play.scene().sprites().len();
    play.receive_presentation(&[oteryn_session::PresentationEventKind::MagicEffect {
        position: oteryn_session::ActorPosition { x: play.own.x, y: play.own.y, floor: play.floor },
        effect_id: 29,
        source: oteryn_session::PresentationSource::Own,
    }], now);
    play.project_presentation(now)?;
    assert!(play.scene().sprites().len() > baseline);
    let mut preferences = play.preferences().clone();
    preferences.game_window.own_spells = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(), baseline);
    Ok(())
}

#[test]
fn t057_target_frame_setting_keeps_target_selected() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let target = play.scene().target();
    let mut preferences = play.preferences().clone();
    preferences.game_window.target_frame = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().target(), target);
    Ok(())
}

#[test]
fn t057_pvp_frames_follow_exact_server_entity_generation() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let entity = oteryn_session::EntityRef { identity: [8;16], generation: 2 };
    play.entities.push(oteryn_session::WorldSpatialEntity {
        entity, display_name: None, kind: oteryn_session::EntityKind::Creature,
        position: oteryn_session::ActorPosition { x:101,y:200,floor:0 },
        detail: oteryn_session::EntityDetail::Actor { direction: oteryn_session::StepDirection::South, appearance_ref:1, health_percent:50 },
    });
    play.party_icons = vec![oteryn_client_runtime::party_skull::PartyIcon {entity, shield:0, skull:4}];
    play.rebuild()?;
    let baseline = play.scene().sprites().len();
    let mut preferences = play.preferences().clone();
    preferences.game_window.pvp_frame = true;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(), baseline + 4);
    play.party_icons[0].entity.generation += 1;
    play.rebuild()?;
    let mut preferences = play.preferences().clone();
    preferences.game_window.pvp_frame = false;
    play.set_preferences(preferences)?;
    let without = play.scene().sprites().len();
    let mut preferences = play.preferences().clone();
    preferences.game_window.pvp_frame = true;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(), without);
    Ok(())
}

#[test]
fn t057_loot_highlight_tracks_live_corpse_visibility() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.entities.push(oteryn_session::WorldSpatialEntity {
        entity: oteryn_session::EntityRef { identity: [9;16],generation:1 },
        display_name:None,kind:oteryn_session::EntityKind::Corpse,
        position:oteryn_session::ActorPosition {x:101,y:200,floor:0},
        detail:oteryn_session::EntityDetail::Object {item_definition_ref:1,quantity:1,item_handle:None},
    });
    play.rebuild()?;
    let baseline=play.scene().sprites().len();
    let mut preferences=play.preferences().clone();
    preferences.game_window.loot_highlight=true;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(),baseline+4);
    play.entities[0].position.floor=1;
    play.rebuild()?;
    let hidden=play.scene().sprites().len();
    let mut preferences=play.preferences().clone();
    preferences.game_window.loot_highlight=false;
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().sprites().len(),hidden);
    Ok(())
}
