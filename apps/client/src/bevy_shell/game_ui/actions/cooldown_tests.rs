use super::{tests::fixture, *};
use oteryn_session::{
    ActorCooldown, ActorCooldownKind, ActorCooldowns, AppliedDelta, SessionEvent,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn named(app: &mut App, wanted: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == wanted).then_some(entity))
}

fn replace(
    app: &mut App,
    outbox: &crate::play::Outbox,
    revision: u64,
    now: u64,
    entries: Vec<ActorCooldown>,
) -> TestResult {
    outbox.push(vec![SessionEvent::ActorCooldowns(AppliedDelta {
        server_sequence: revision,
        base_revision: revision - 1,
        new_revision: revision,
        value: oteryn_session::ActorCooldownUpdate {
            value: ActorCooldowns {
                server_now_ms: now,
                rtt_ms: None,
                entries,
            },
            received_at: std::time::Instant::now(),
            snapshot: true,
        },
    })]);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|_| "tick")?;
    }
    app.update();
    Ok(())
}

fn fill_widths(app: &mut App) -> Vec<Val> {
    app.world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "Cooldown remaining")
        .map(|(_, node)| node.width)
        .collect()
}

fn has_image_rect(app: &mut App, rect: Rect) -> bool {
    app.world_mut()
        .query::<&ImageNode>()
        .iter(app.world())
        .any(|image| image.rect == Some(rect))
}

#[test]
fn native_cooldown_strip_keeps_groups_expires_spells_and_clears_on_disconnect() -> TestResult {
    let (mut app, _) = fixture()?;
    let (link, _, outbox) = crate::play::play_channel();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let group = || ActorCooldown {
        kind: ActorCooldownKind::Group,
        id: 2,
        expires_at_ms: 100100,
    };
    replace(
        &mut app,
        &outbox,
        1,
        100,
        vec![
            group(),
            ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: 71,
                expires_at_ms: 50100,
            },
        ],
    )?;
    // The old text projection has no icons or graphical timer fills.
    let entity = named(&mut app, "Cooldown group 2").ok_or("missing persistent group icon")?;
    assert!(named(&mut app, "Cooldown spell 71").is_some());
    let fills = fill_widths(&mut app);
    assert_eq!(fills.len(), 2);
    assert!(
        fills
            .iter()
            .all(|width| matches!(width, Val::Percent(value) if *value > 0. && *value <= 100.))
    );
    assert!(has_image_rect(&mut app, Rect::new(20., 20., 40., 40.)));
    // Confirmed replacements advance the clock without sleeps or local cast prediction.
    replace(&mut app, &outbox, 2, 50100, vec![group()])?;
    assert_eq!(named(&mut app, "Cooldown group 2"), Some(entity));
    assert!(
        fill_widths(&mut app)
            .iter()
            .any(|width| matches!(width, Val::Percent(value) if *value > 0. && *value <= 50.))
    );
    replace(&mut app, &outbox, 3, 100200, vec![])?;
    assert_eq!(named(&mut app, "Cooldown group 2"), Some(entity));
    assert!(named(&mut app, "Cooldown spell 71").is_none());
    assert_eq!(fill_widths(&mut app), [percent(0.)]);
    assert!(has_image_rect(&mut app, Rect::new(20., 0., 40., 20.)));
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.action_bar.cooldown_bar_enabled = false;
        owner
            .settings
            .sync_local_preferences(&owner.play.as_ref().ok_or("play")?.link);
    }
    app.update();
    assert!(named(&mut app, "Cooldown group 2").is_none());
    assert!(
        app.world_mut()
            .query_filtered::<&Node, With<CooldownLine>>()
            .iter(app.world())
            .all(|node| node.display == Display::None)
    );
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.action_bar.cooldown_bar_enabled = true;
        owner.settings.preferences.action_bar.visible = false;
        owner
            .settings
            .sync_local_preferences(&owner.play.as_ref().ok_or("play")?.link);
    }
    app.update();
    assert!(named(&mut app, "Cooldown group 2").is_some());
    assert!(
        app.world_mut()
            .query_filtered::<&Node, With<BarRow>>()
            .iter(app.world())
            .all(|node| node.display == Display::None)
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(named(&mut app, "Cooldown group 2").is_none());
    Ok(())
}

#[test]
fn slot_timer_formats_follow_independent_settings() -> TestResult {
    let (mut app, _) = fixture()?;
    let (link, _, outbox) = crate::play::play_channel();
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "".into(), "".into()]));
    let entry = oteryn_session::spell_catalog::SpellCatalogEntry {
        cast_index: 71,
        name: "Heal".into(),
        words: "exura".into(),
        level: 1,
        mana: 0,
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
    assert!(link.assign_bar_action(
        1,
        crate::play::BarAction::Spell {
            name: "Heal".into(),
            words: "exura".into(),
            aim: Default::default(),
        }
    ));
    assert!(link.assign_bar_action(
        2,
        crate::play::BarAction::Text {
            text: "exura".into(),
            send_automatically: true
        }
    ));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    for (numbers, graphical) in [(true, false), (false, true)] {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.action_bar.cooldown_numbers = numbers;
        owner.settings.preferences.action_bar.graphical_cooldown = graphical;
        owner
            .settings
            .sync_local_preferences(&owner.play.as_ref().ok_or("play")?.link);
        replace(
            &mut app,
            &outbox,
            if numbers { 1 } else { 2 },
            100,
            vec![ActorCooldown {
                kind: ActorCooldownKind::Spell,
                id: 71,
                expires_at_ms: 50100,
            }],
        )?;
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .spell_slot_labels()[0]
                .contains(" ms"),
            numbers
        );
        let labels = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels();
        assert_eq!(labels[1].contains(" ms"), numbers);
        assert_eq!(labels[2].contains(" ms"), numbers);
        let spoken = named(&mut app, "Action slot 3 cooldown").ok_or("spoken spell timer")?;
        assert_eq!(
            app.world().get::<Node>(spoken).ok_or("overlay")?.display != Display::None,
            graphical
        );
        let saved = named(&mut app, "Action slot 2 cooldown").ok_or("saved slot timer")?;
        assert_eq!(
            app.world()
                .get::<Node>(saved)
                .ok_or("saved overlay")?
                .display
                != Display::None,
            graphical
        );
        let overlay = named(&mut app, "Action slot 1 cooldown").ok_or("slot timer")?;
        let node = app.world().get::<Node>(overlay).ok_or("overlay node")?;
        assert_eq!(node.display != Display::None, graphical);
        if graphical {
            assert!(matches!(node.height, Val::Percent(p) if p > 0. && p <= 100.));
        }
    }
    Ok(())
}
