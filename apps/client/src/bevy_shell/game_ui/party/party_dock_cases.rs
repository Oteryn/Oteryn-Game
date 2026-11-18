#[test]
fn party_portrait_projects_pinned_outfit_artwork_into_bevy_image() -> Result {
    use oteryn_client_runtime::appearance::{Binding, Context, Look};
    let (mut app, mut commands, outbox) = app()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let assets = super::super::outfit::preview::Assets::load(&root)?;
    *app.world_mut().resource_mut::<combat::Artwork>() = combat::Artwork {
        assets: Some(assets),
        attempted: true,
    };
    let entity = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let mut result = state();
    result.members[0].list.push(party::PartyListEntity {
        entity,
        name: "Leader".into(),
        vocation: 1,
        summon: false,
        distance: 0,
        health_percent: Some(100),
        mana_percent: Some(100),
        skull: None,
        portrait: Some(Binding {
            context: Context {
                content_generation: [1; 32],
                cosmetics_digest: [2; 32],
            },
            appearance_ref: 1,
            entity,
            root_revision: 1,
            condition_revision: 0,
            owner_time_us: 1,
            look: Look::Outfit {
                look_type: 128,
                addons: 3,
                colours: [0; 4],
            },
            selection_revision: None,
            forge_stacks: None,
        }),
        portrait_colours: [[255, 100, 50]; 4],
    });
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    let _ = command(&mut commands)?;
    publish(&outbox, &result, party::PartyDisposition::Ok)?;
    app.update();
    let portrait = app
        .world_mut()
        .query::<(&Name, &ImageNode)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "party.portrait")
        .map(|(_, image)| image.clone())
        .ok_or("portrait image")?;
    assert_eq!(portrait.rect, Some(Rect::new(32., 32., 96., 96.)));
    let image = app
        .world()
        .resource::<Assets<Image>>()
        .get(&portrait.image)
        .ok_or("image asset")?;
    assert_eq!(image.size(), UVec2::splat(96));
    assert!(
        image
            .data
            .as_ref()
            .ok_or("pixel data")?
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[..3] != [32, 32, 32]),
        "portrait must contain verified outfit pixels"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn open_party_refreshes_live_vitals_without_overlapping_requests_or_polling_when_closed() -> Result
{
    let (mut app, mut commands, outbox) = app()?;
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(std::time::Duration::from_secs(2));
    app.update();
    app.update();
    assert_eq!(
        command(&mut commands)?.kind,
        party::COMMAND_TYPE_PARTY_QUERY
    );
    for _ in 0..3 {
        app.update();
    }
    assert!(
        commands.try_recv().is_err(),
        "a pending query must not overlap"
    );
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    // A closed dock must stop the refresh stream immediately.
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    for _ in 0..3 {
        app.update();
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn a_summon_row_cannot_select_its_owner_for_kicking() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    let mut result = state();
    result.members[1].list.push(party::PartyListEntity {
        entity: EntityRef { identity: [3; 16], generation: 1 },
        name: "Summoned rat".into(),
        vocation: 1,
        summon: true,
        distance: 1,
        health_percent: Some(100),
        mana_percent: None,
        skull: None,
        portrait: None,
        portrait_colours: [[0; 3]; 4],
    });
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    let _ = command(&mut commands)?;
    publish(&outbox, &result, party::PartyDisposition::Ok)?;
    app.update();
    let row = app.world_mut().query::<(Entity, &Name)>().iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "party.combat-row").then_some(entity))
        .ok_or("summon row")?;
    app.world_mut().trigger(bevy::ui_widgets::Activate { entity: row });
    app.update();
    let play = app.world().non_send::<HostOwner>().play.as_ref().ok_or("play")?;
    assert_eq!(play.link.party_panel().selected_member, None);
    click(&mut app, |kind| matches!(kind, Kind::Kick))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
