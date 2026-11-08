use super::*;
#[test]
fn replies_keywords_and_close_follow_server() {
    let npc = EntityRef {
        identity: [7; 16],
        generation: 2,
    };
    let mut panel = DialoguePanel {
        selected: true,
        ..Default::default()
    };
    assert!(panel.request(npc, "hi").is_some());
    panel.apply(npc_service::NpcConversation {
        open: Some((npc, npc_service::NpcConversationState::Open)),
        lines: vec!["Ask about {job}, {name}, or {job}.".into()],
    });
    assert_eq!(panel.keywords(), vec!["job", "name"]);
    assert!(panel.text().contains("Ask about"));
    assert!(panel.request(npc, " ").is_none());
    assert!(panel.request(npc, &"x".repeat(256)).is_none());
    panel.apply(Default::default());
    assert!(!panel.open);
    assert!(panel.target.is_none());
}
#[test]
fn input_requires_exact_chord_and_no_repeat() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::*;
    let code = KeyCode::new(27)?;
    let event = |repeat, modifiers| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat,
    };
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    assert!(open_key(&[event(false, modifiers)]));
    assert!(!crate::world_event::open_key(&[event(false, modifiers)]));
    assert!(!crate::tutorial_notices::open_key(&[event(
        false, modifiers
    )]));
    assert!(!open_key(&[event(true, modifiers)]));
    assert!(!open_key(&[event(
        false,
        Modifiers::one(Modifier::Control)
    )]));
    Ok(())
}

#[test]
fn capability_pending_refusals_and_target_switching() {
    let first = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let second = EntityRef {
        identity: [2; 16],
        generation: 2,
    };
    let mut panel = DialoguePanel::default();
    assert!(panel.request(first, "hi").is_none());
    panel.selected = true;
    assert!(panel.request(first, "hi").is_some());
    assert!(panel.request(second, "hi").is_none());
    panel.result(NpcIntentDisposition::RateLimited);
    assert!(panel.feedback.is_some());
    panel.lines = vec!["Old reply".into()];
    assert!(panel.request(second, "hi").is_some());
    assert!(panel.lines.is_empty());
    panel.result(NpcIntentDisposition::NotInRange);
    assert!(!panel.open);
    assert!(panel.target.is_none());
}

#[test]
fn terminal_refusal_survives_the_servers_closed_update() {
    let npc = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    for refusal in [
        NpcIntentDisposition::NotInRange,
        NpcIntentDisposition::NoConversation,
        NpcIntentDisposition::Stale,
    ] {
        let mut panel = DialoguePanel {
            selected: true,
            ..Default::default()
        };
        assert!(panel.request(npc, "hi").is_some());
        panel.result(refusal);
        panel.apply(Default::default());
        assert!(panel.text().contains(&format!("{refusal:?}")));
        assert!(panel.visible());
        assert!(!panel.open);
        assert!(panel.target.is_none());
        assert!(panel.request(npc, "hi").is_some());
        assert!(panel.feedback.is_none());
    }
}

#[test]
fn dialogue_shortcut_avoids_altgr() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::*;
    let event = |code, modifiers| {
        Ok::<_, InputError>(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        })
    };
    assert!(!open_key(&[event(
        17,
        Modifiers::one(Modifier::Control).with(Modifier::Alt)
    )?]));
    assert!(open_key(&[event(
        27,
        Modifiers::one(Modifier::Control).with(Modifier::Shift)
    )?]));
    Ok(())
}

#[test]
fn farewell_remains_visible_without_an_active_conversation() {
    let mut panel = DialoguePanel::default();
    assert!(!panel.visible());
    panel.apply(npc_service::NpcConversation {
        open: None,
        lines: vec!["Farewell, traveller!".into()],
    });
    assert!(!panel.open);
    assert!(panel.target.is_none());
    assert!(panel.visible());
    assert!(panel.text().contains("Farewell, traveller!"));
    assert!(panel.visible());
    panel = DialoguePanel::default();
    assert!(!panel.visible());
}

#[test]
fn w05d_seasonal_npc_topics_decode_and_render_in_usable_native_dialogue()
-> Result<(), Box<dyn std::error::Error>> {
    let santa = EntityRef {
        identity: [7; 16],
        generation: 2,
    };
    let baker = EntityRef {
        identity: [8; 16],
        generation: 2,
    };
    let mut panel = DialoguePanel {
        selected: true,
        ..Default::default()
    };
    for (npc, topic, reply) in [
        (santa, "present", "Merry Christmas!"),
        (santa, "present", "You can't get other present."),
        (
            baker,
            "mass",
            "So far, 200 of the 200 Sweet and Sugary Substances have been delivered here in Thais.",
        ),
        (
            baker,
            "base",
            "So far, 200 of the 200 Cake Golems have been lured to the machines.",
        ),
        (
            baker,
            "deliver",
            "Wonderful, thank you! You delivered 200 substance(s). (200/200 total delivered)",
        ),
    ] {
        let intent = panel.request(npc, topic).ok_or("native topic request")?;
        let bytes =
            npc_service::encode_npc_talk_intent(&intent).map_err(|_| "encode native talk")?;
        let decoded =
            npc_service::decode_npc_talk_intent(&bytes).map_err(|_| "decode native talk")?;
        assert_eq!((decoded.npc_actor, decoded.text.as_str()), (npc, topic));
        let conversation = npc_service::NpcConversation {
            open: Some((npc, npc_service::NpcConversationState::Open)),
            lines: vec![reply.into()],
        };
        let payload = npc_service::encode_npc_conversation(&conversation)
            .map_err(|_| "encode native conversation")?;
        panel.apply(
            npc_service::decode_npc_conversation(&payload)
                .map_err(|_| "decode native conversation")?,
        );
        assert!(panel.visible());
        assert_eq!(panel.text(), reply);
        assert_eq!(panel.target, Some(npc));
        assert!(!panel.pending, "the next native topic must remain usable");
    }
    Ok(())
}
