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
        value: ActorCooldowns {
            server_now_ms: now,
            rtt_ms: None,
            entries,
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
