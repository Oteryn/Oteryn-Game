#[test]
fn o025_session_vitals_drive_health_circle_in_live_tick() -> Result<(), Box<dyn std::error::Error>>
{
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.health_circle = true;
    preferences.hud_bars = false;
    preferences.show_harmony = false;
    preferences.show_serene = false;
    play.set_preferences(preferences)?;
    let baseline = play.scene().sprites().len();
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::ActorVitals {
                health: 50,
                max_health: 100,
                mana: 20,
                max_mana: 40,
                ..Default::default()
            },
        },
    )]);
    play.tick(&link).map_err(|_| "session ended")?;
    assert_eq!(play.scene().sprites().len(), baseline + 32);
    Ok(())
}
