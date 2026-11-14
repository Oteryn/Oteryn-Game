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
