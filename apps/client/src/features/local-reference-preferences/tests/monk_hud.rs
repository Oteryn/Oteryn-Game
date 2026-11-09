#[test]
fn t162_monk_combo_tracks_server_harmony_and_has_independent_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.hud_bars = false;
    preferences.show_harmony = false;
    preferences.show_serene = false;
    preferences.resource_hud.combo = true;
    play.set_preferences(preferences.clone())?;
    let base = play.scene().sprites().len();
    let event = |sequence, harmony, serene| {
        oteryn_session::SessionEvent::ActorVitals(oteryn_session::AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence - 1,
            new_revision: sequence,
            value: oteryn_session::ActorVitals {
                health: 100,
                max_health: 100,
                mana: 50,
                max_mana: 50,
                harmony,
                serene,
                ..Default::default()
            },
        })
    };
    outbox.push(vec![event(1, 3, false)]);
    play.tick(&link).map_err(|_| "session ended")?;
    assert!(play.resource_status().contains("Combo charges: 3/5"));
    assert!(!play.resource_status().contains("Serene:"));
    assert_eq!(play.scene().sprites().len(), base + 5);
    let charged = play.scene().sprites().instances()[base..].to_vec();
    play.cast(std::num::NonZeroU32::MIN);
    assert!(play.resource_status().contains("Combo charges: 3/5"));
    outbox.push(vec![event(2, 0, true)]);
    play.tick(&link).map_err(|_| "session ended")?;
    assert!(play.resource_status().contains("Combo charges: 0/5"));
    assert_ne!(play.scene().sprites().instances()[base].uv, charged[0].uv);
    preferences.resource_hud.combo = false;
    preferences.show_serene = true;
    play.set_preferences(preferences)?;
    assert!(!play.resource_status().contains("Combo charges:"));
    assert!(play.resource_status().contains("Serene: active"));
    assert_eq!(play.scene().sprites().len(), base + 1);
    Ok(())
}
