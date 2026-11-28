use super::*;

#[test]
fn named_catalogue_search_paging_and_activation_use_current_server_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _) = super::super::super::tests::fixture()?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    let entries: Vec<_> = (1..=13)
        .map(
            |cast_index| oteryn_session::spell_catalog::SpellCatalogEntry {
                cast_index,
                name: format!("Healing {cast_index}"),
                words: format!("exura {cast_index}"),
                level: 8,
                mana: 20,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![2],
                rune_item: None,
            },
        )
        .collect();
    let payload = oteryn_session::spell_catalog::encode_catalog(&entries)
        .map_err(|error| format!("{error:?}"))?;
    crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
        &outbox, payload,
    )?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    assert_eq!(
        app.world_mut().query::<&Choice>().iter(app.world()).count(),
        12
    );
    let next = app
        .world_mut()
        .query::<(Entity, &Edit)>()
        .iter(app.world())
        .find_map(|(entity, edit)| matches!(edit, Edit::NextSpells).then_some(entity))
        .ok_or("next")?;
    app.world_mut().trigger(Activate { entity: next });
    app.update();
    assert_eq!(
        app.world_mut().query::<&Choice>().iter(app.world()).count(),
        1
    );
    let choice = app
        .world_mut()
        .query::<(Entity, &Choice)>()
        .iter(app.world())
        .find_map(|(entity, choice)| (choice.cast_index == 13).then_some(entity))
        .ok_or("last spell")?;
    app.world_mut().trigger(Activate { entity: choice });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Assigned Healing 13"
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("Healing 13")
    );
    assert!(
        commands.try_recv().is_err(),
        "assignment never casts a spell"
    );
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            text.editor_mut().set_text("exura 7");
        }
    }
    app.update();
    assert_eq!(
        app.world_mut().query::<&Choice>().iter(app.world()).count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&Choice>()
            .iter(app.world())
            .next()
            .ok_or("filtered")?
            .name,
        "Healing 7"
    );
    Ok(())
}

#[test]
fn stale_catalogue_choice_cannot_assign_unobserved_spell() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut app, mut commands) = super::super::super::tests::fixture()?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    let button = app
        .world_mut()
        .spawn(Choice {
            slot: 0,
            generation,
            cast_index: 71,
            name: "Unobserved spell".into(),
            words: "test".into(),
        })
        .observe(choose)
        .id();
    // Keep the stale choice inside the visible editor so the catalogue identity
    // check, rather than the modal perimeter, is the rejection exercised here.
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<CatalogRoot>>()
        .iter(app.world())
        .next()
        .ok_or("catalogue root")?;
    app.world_mut().entity_mut(root).add_child(button);
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        app.world()
            .resource::<EditorState>()
            .message
            .starts_with("Spell catalogue changed")
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("Unobserved spell")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn assigned_incantation_parameters_follow_the_slot_display_setting()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _) = super::super::super::tests::fixture()?;
    let (link, _, outbox) = crate::play::play_channel();
    let spell = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 71,
        name: "Find Person".into(),
        words: "exiva".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![],
        rune_item: None,
    };
    let plain = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 72,
        name: "Light Healing".into(),
        words: "exura".into(),
        ..spell.clone()
    };
    crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
        &outbox,
        oteryn_session::spell_catalog::encode_catalog(&[spell, plain])
            .map_err(|e| format!("{e:?}"))?,
    )?;
    assert!(link.assign_bar_action(
        0,
        BarAction::Text {
            text: "exiva \"Alice\"".into(),
            send_automatically: true
        }
    ));
    assert!(link.assign_bar_action(
        1,
        BarAction::Text {
            text: "exura Alice".into(),
            send_automatically: true,
        }
    ));
    assert!(link.spell_slot_labels()[1].contains("exura Alice"));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    for show in [true, false] {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.action_bar.show_spell_parameters = show;
        owner
            .settings
            .sync_local_preferences(&owner.play.as_ref().ok_or("play")?.link);
        app.update();
        let label = app
            .world_mut()
            .query::<(&super::super::super::SlotCaption, &Text)>()
            .iter(app.world())
            .find(|(s, _)| s.0 == 0)
            .ok_or("caption")?
            .1;
        assert_eq!(label.lines().any(|s| s == "Alice"), show);
    }
    Ok(())
}

#[test]
fn gap_0608_native_parameter_control_sends_the_typed_cast_on_activation()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::actor_spell_v2 as wire;

    let (mut app, _) = super::super::super::tests::fixture()?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    let entry = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 71,
        name: "Find Person".into(),
        words: "exiva".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![],
        rune_item: None,
    };
    crate::play::feature_0287_actor_spell_catalog::protocol::bevy_catalog_fixture(
        &outbox,
        oteryn_session::spell_catalog::encode_catalog(&[entry]).map_err(|e| format!("{e:?}"))?,
    )?;
    outbox.push(vec![oteryn_session::SessionEvent::SpellParametersSelected]);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("active play")?
        .link = link;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut()
        .resource_mut::<EditorState>()
        .open(0, generation);
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == PARAMETER_FIELD {
            text.editor_mut().set_text("Alice Smith");
        }
    }
    let choice = app
        .world_mut()
        .query_filtered::<Entity, With<Choice>>()
        .iter(app.world())
        .next()
        .ok_or("catalogue choice")?;
    app.world_mut().trigger(Activate { entity: choice });
    assert_eq!(
        app.world().resource::<EditorState>().message,
        "Assigned Find Person"
    );
    assert!(commands.try_recv().is_err(), "assigning must not cast");
    app.world_mut().resource_mut::<EditorState>().selected = None;
    app.update();
    let slot = super::super::super::tests::slot(&mut app, 0)?;
    app.world_mut().trigger(Activate { entity: slot });
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("expected parameter cast".into());
    };
    assert_eq!(
        command.capability,
        wire::CAPABILITY_ACTOR_SPELL_PARAMETERS_V2
    );
    assert_eq!(
        command.kind,
        wire::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2
    );
    let intent =
        wire::decode_parameter_spell_cast_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.intent.spell.get(), 71);
    assert_eq!(intent.parameter.as_deref(), Some("Alice Smith"));
    let feedback = "Alice Smith is far to the north.";
    outbox.push(vec![oteryn_session::SessionEvent::ParameterSpellCast {
        result: wire::ParameterSpellCastResult {
            disposition: oteryn_session::SpellCastDisposition::Cast,
            feedback: Some(wire::PrivateSpellFeedback {
                text: feedback.into(),
                effect: wire::PrivateSpellEffect::MagicBlue,
            }),
            editor: None,
        },
        position: oteryn_session::ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
    }]);
    app.update();
    assert!(
        app.world_mut()
            .query_filtered::<&Text, With<super::super::super::ActionHint>>()
            .iter(app.world())
            .any(|text| text.0 == feedback)
    );
    Ok(())
}
