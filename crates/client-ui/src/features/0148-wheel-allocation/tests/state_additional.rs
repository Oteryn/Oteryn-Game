use super::*;

#[test]
fn stale_or_incomplete_live_queries_never_partially_replace_the_view() -> Result<(), Error> {
    let mut panel = panel();
    let before = (
        panel.confirmed_wheel.clone(),
        panel.confirmed_atelier.clone(),
    );
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (mut wheel, atelier) = current(3);
    wheel.points[14] = 25;
    let stale = response(
        161,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    assert!(!apply_result(&mut panel, &stale));
    assert_eq!(
        (&panel.confirmed_wheel, &panel.confirmed_atelier),
        (&before.0, &before.1)
    );
    assert!(!panel.ready());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (wheel, atelier) = current(5);
    let mut incomplete = response(
        161,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    incomplete.atelier = Some(vec![255]);
    assert!(!apply_result(&mut panel, &incomplete));
    assert_eq!(
        (&panel.confirmed_wheel, &panel.confirmed_atelier),
        (&before.0, &before.1)
    );
    assert!(!panel.synchronized);
    Ok(())
}

#[test]
fn uncertain_outcome_retains_exact_request_across_refresh_and_retry() -> Result<(), Error> {
    let mut panel = panel();
    let command = request(&mut panel, Intent::Reveal, occurrence(1)).ok_or("reveal")?;
    let (wheel, atelier) = current(5);
    let expired = response(
        424,
        CommandStatus::DuplicateOutcomeExpired,
        vec![],
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &expired));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    assert!(request(&mut panel, Intent::Reveal, occurrence(2)).is_none());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    assert!(apply_result(
        &mut panel,
        &response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&wheel))?,
            &wheel,
            &atelier
        )?
    ));
    assert_eq!(panel.retry.as_ref(), Some(&command));
    let retry = request(&mut panel, Intent::Retry, occurrence(99)).ok_or("retry")?;
    assert_eq!(retry, command);
    assert!(apply_result(
        &mut panel,
        &response(
            424,
            CommandStatus::DuplicateReplay,
            wire(atelier::encode_reply(&atelier))?,
            &wheel,
            &atelier
        )?
    ));
    assert!(panel.retry.is_none());
    assert!(panel.ready());
    Ok(())
}

#[test]
fn panel_displays_server_quotes_and_the_matching_modifier_grade() {
    let mut panel = panel();
    panel.quality = 2;
    panel.modifier = Some((1, 5));
    let text = panel_text(&panel);
    assert!(text.contains("123458 gold"));
    assert!(
        text.contains(
            "Removing points or resetting is blocked while you are in a fight (PZ lock)."
        )
    );
    assert!(!text.contains("654321 gold"), "no domain switch price");
    assert!(text.contains("9–11 supreme fragments"));
    assert!(text.contains("222222 gold and 777 supreme fragments"));
    assert!(modifier_label(&panel, 1, 5).contains("Grade II"));
    assert!(!slot_label(&panel, 14).contains("Green"));
    assert!(!slot_label(&panel, 20).contains("Blue"));
}

#[test]
fn modifier_labels_distinguish_weaknesses_and_paid_spell_variants() {
    let panel = panel();
    assert!(modifier_label(&panel, 0, 7).contains("Holy Resistance + Death Weakness"));
    assert!(modifier_label(&panel, 0, 15).contains("Fire Resistance + Earth Weakness"));
    assert!(modifier_label(&panel, 0, 9).contains("Fire Resistance + Earth Resistance"));
    let cooldown = modifier_label(&panel, 1, 7);
    let damage = modifier_label(&panel, 1, 8);
    let critical = modifier_label(&panel, 1, 9);
    assert!(cooldown.contains("Cooldown"));
    assert!(damage.contains("Damage"));
    assert!(critical.contains("Critical damage"));
    assert_ne!(cooldown, damage);
    assert_ne!(damage, critical);
    // Crystal's executed Flurry fields are reversed relative to the source enum names.
    assert!(modifier_label(&panel, 1, 82).contains("Critical damage"));
    assert!(modifier_label(&panel, 1, 83).contains("Damage"));
}

#[test]
fn open_shortcut_requires_a_fresh_control_w_press() -> Result<(), Error> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(26)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let control = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, control, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    assert!(!open_key(&[event(ButtonState::Pressed, control, true)]));
    assert!(!open_key(&[event(ButtonState::Released, control, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        control.with(Modifier::Shift),
        false
    )]));
    Ok(())
}

#[test]
fn wheel_build_exchange_is_bounded_and_only_applies_as_a_server_proposal() -> Result<(), Error> {
    let mut panel = panel();
    panel.points[0] = 25;
    let code = panel.build_code();
    panel.points = [0; 36];
    panel.import_build(&code)?;
    assert_eq!(panel.points[0], 25);
    assert_eq!(
        panel
            .confirmed_wheel
            .as_ref()
            .ok_or("missing snapshot")?
            .points[0],
        0
    );
    let command = request(&mut panel, Intent::Save, occurrence(3)).ok_or("missing command")?;
    let decoded = wire(wheel::decode_wheel_request(&command.payload))?;
    assert_eq!(decoded.points[0], 25);
    assert_eq!(decoded.expected_revision, 4);
    assert!(panel.import_build(&code).is_err());
    panel.pending = None;
    panel.import_build(&format!(
        "oteryn://wheel/v1/{}",
        code.strip_prefix("OW1:").ok_or("prefix")?
    ))?;
    let before = panel.points;
    for invalid in ["OW1:1", "OW1:65535", "https://untrusted.invalid/OW1:0"] {
        assert!(panel.import_build(invalid).is_err());
        assert_eq!(panel.points, before);
    }
    Ok(())
}

#[test]
fn wheel_named_presets_roundtrip_and_reject_corrupt_storage() -> Result<(), Error> {
    use super::super::presets::PresetBook;
    let path = std::env::temp_dir().join(format!(
        "oteryn-wheel-presets-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut book = PresetBook::default();
    book.insert("Hunting", &panel().build_code())?;
    book.save(&path)?;
    assert_eq!(PresetBook::load(&path)?, book);
    assert!(book.insert("", &panel().build_code()).is_err());
    assert!(book.insert("Broken", "OW1:9999").is_err());
    std::fs::write(&path, br#"{"version":99,"presets":{}}"#)?;
    assert!(PresetBook::load(&path).is_err());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn t120_named_presets_load_into_the_wheel_by_code_and_by_url() -> Result<(), Error> {
    use super::super::presets::PresetBook;
    let mut source = panel();
    source.points[0] = 12;
    let mut book = PresetBook::default();
    book.insert("  Hunting ", &source.build_code())?;
    book.insert("Boss", &panel().build_code())?;
    let saved = book.0.get("Hunting").ok_or("trimmed preset name")?.clone();
    assert_eq!(book.0.len(), 2);
    let mut target = panel();
    target.import_build(&saved)?;
    assert_eq!(target.points[0], 12);
    let mut by_url = panel();
    by_url.import_build(&saved.replacen("OW1:", "oteryn://wheel/v1/", 1))?;
    assert_eq!(by_url.points, target.points);
    assert!(book.0.remove("Boss").is_some());
    assert!(!book.0.contains_key("Boss"));
    Ok(())
}
