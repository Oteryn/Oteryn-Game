#[test]
fn analyser_resolves_following_pages_and_new_items() -> TestResult {
    use oteryn_session::{
        analyser_catalogue::{ItemInfo, decode_request, encode_reply},
        analyser_transport::AnalyserItemCount,
    };
    use std::num::{NonZeroU16, NonZeroU32};
    let (mut app, outbox) = app()?;
    let items = (1000..1100)
        .map(|id| {
            Ok(AnalyserItemCount {
                item_type: NonZeroU32::new(id).ok_or("id")?,
                count: NonZeroU16::MIN,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 2,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: items,
            gold: 0,
        }],
    })]);
    select(&mut app, Tool::Loot)?;
    select(&mut app, Tool::Session)?;
    let reset = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.reset && c.tool == Tool::Session).then_some(entity))
        .ok_or("session reset")?;
    let panel = |app: &App| -> Result<_, Box<dyn std::error::Error>> {
        Ok(app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel())
    };
    let loot = panel(&app)?.loot;
    assert!((1000..1100).all(|id| loot.contains_key(&id)));
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    let reset_panel = panel(&app)?;
    assert!(reset_panel.loot.is_empty());
    let retained = &reset_panel
        .reports
        .get(Tool::Loot.key())
        .ok_or("retained loot report")?
        .loot;
    assert_eq!(retained.len(), loot.len());
    for (id, item) in loot {
        assert_eq!(retained.get(&id).map(|row| row.count), Some(item.count));
    }
    for _ in 0..10 {
        let command = {
            let mut commands = app.world_mut().non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>>();
            std::iter::from_fn(|| commands.try_recv().ok())
                .find_map(|command| match command {
                    crate::combat_input::PlayCommand::Feature(command) if command.kind == 734 => {
                        Some(command)
                    }
                    _ => None,
                })
                .ok_or("missing identity page")?
        };
        let ids = decode_request(&command.payload)?;
        let rows = ids
            .iter()
            .map(|id| ItemInfo {
                item_type: *id,
                name: format!("Item {id}"),
                coin_worth: 0,
            })
            .collect::<Vec<_>>();
        outbox.bevy_analyser_catalogue_result(&oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 3,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_reply(&rows)?,
        });
        app.update();
        let panel = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel();
        if panel.item_info.len() == 102 {
            break;
        }
    }
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel()
            .item_info
            .len(),
        102
    );
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 4,
        base_revision: 3,
        new_revision: 4,
        value: vec![AnalyserFact::SupplyUsed(AnalyserItemCount {
            item_type: NonZeroU32::new(2000).ok_or("id")?,
            count: NonZeroU16::MIN,
        })],
    })]);
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel()
            .catalogue_pending,
        vec![2000]
    );
    Ok(())
}

#[test]
fn analyser_configuration_reaches_validated_goal_and_price_owners() -> TestResult {
    use bevy::text::EditableText;
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::Session)?;
    outbox.bevy_analyser_catalogue_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: oteryn_session::analyser_catalogue::encode_reply(&[202, 203].map(|item_type| {
            oteryn_session::analyser_catalogue::ItemInfo {
                item_type,
                name: format!("Item {item_type}"),
                coin_worth: 0,
            }
        }))?,
    });
    app.update();
    let find = |app: &mut App, label: &str| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == label).then_some(parent.parent()))
            .ok_or_else(|| format!("missing {label}").into())
    };
    let goal = find(&mut app, "Set XP goal")?;
    let mut fields = app
        .world_mut()
        .query_filtered::<Entity, With<EditableText>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    fields.sort_by_key(|entity| entity.index());
    assert_eq!(fields.len(), 3);
    for (entity, value) in fields.iter().zip(["1000", "202", "10"]) {
        app.world_mut()
            .get_mut::<EditableText>(*entity)
            .ok_or("field")?
            .editor_mut()
            .set_text(value);
    }
    let price = find(&mut app, "Set loot value")?;
    app.world_mut().trigger(Activate { entity: goal });
    app.world_mut().trigger(Activate { entity: price });
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel()
            .goal_xp,
        Some(1150)
    );
    app.update();
    let text = report(&mut app)?;
    assert!(text.contains("Priced balance: 107 gold"), "{text}");
    app.world_mut()
        .get_mut::<EditableText>(fields[0])
        .ok_or("field")?
        .editor_mut()
        .set_text("invalid");
    app.world_mut().trigger(Activate { entity: goal });
    app.update();
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_panel()
            .goal_xp,
        Some(1150)
    );
    Ok(())
}

#[test]
fn boss_tracker_refreshes_cooldowns_after_browsing_gallery() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::{boss_encounter::catalogue::Page, raw_feature::GatedCommandResult};
    use oteryn_client_ui::boss_encounter::Intent;
    let (mut app, outbox) = app()?;
    outbox.bevy_bosstiary_initialize();
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.boss_encounter_request(Intent::Catalogue);
    outbox.bevy_bosstiary_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: Page {
            next: None,
            points: 0,
            entries: vec![],
        }
        .encode()
        .ok_or("page")?,
    });
    app.world_mut()
        .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>()
        .try_recv()?;
    select(&mut app, Tool::Bosses)?;
    {
        let mut commands = app
            .world_mut()
            .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>();
        assert!(std::iter::from_fn(|| commands.try_recv().ok()).any(|c| matches!(c, PlayCommand::Feature(c) if c.kind == 133 && !c.payload.starts_with(b"BCT1"))));
    }
    outbox.bevy_bosstiary_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![8, 100],
    });
    app.update();
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(t, p)| (t.0 == "Refresh cooldowns").then_some(p.parent()))
        .ok_or("refresh")?;
    app.world_mut().trigger(Activate { entity });
    let mut commands = app
        .world_mut()
        .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>();
    assert!(std::iter::from_fn(|| commands.try_recv().ok()).any(
        |c| matches!(c, PlayCommand::Feature(c) if c.kind == 133 && !c.payload.starts_with(b"BCT1"))
    ));
    Ok(())
}

#[test]
fn parity_a_analyser_windows_and_resets_are_independent() -> TestResult {
    let (mut app, _) = app()?;
    select(&mut app, Tool::Loot)?;
    select(&mut app, Tool::Experience)?;
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Overlay>>()
            .iter(app.world())
            .count(),
        2
    );
    let reset = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.tool == Tool::Loot && control.reset).then_some(entity)
        })
        .ok_or("loot reset")?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(
        panel
            .reports
            .get(Tool::Loot.key())
            .ok_or("loot interval")?
            .gold,
        0
    );
    assert_eq!(panel.gold, 77);
    assert_eq!(panel.gained_xp, 150);
    assert_eq!(panel.supplies, 2);
    Ok(())
}

#[test]
fn parity_a_loot_reset_retains_xp_and_supplies() -> TestResult {
    let (mut app, _) = app()?;
    select(&mut app, Tool::Loot)?;
    let reset = control(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    let panel = owner.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(
        panel
            .reports
            .get(Tool::Loot.key())
            .ok_or("loot interval")?
            .gold,
        0
    );
    assert_eq!(panel.gold, 77);
    assert_eq!(panel.gained_xp, 150);
    assert_eq!(panel.supplies, 2);
    Ok(())
}

#[test]
fn boss_tracker_refreshes_cooldowns_after_pending_gallery_reply() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_client_runtime::{boss_encounter::catalogue::Page, raw_feature::GatedCommandResult};
    use oteryn_client_ui::boss_encounter::Intent;
    let (mut app, outbox) = app()?;
    outbox.bevy_bosstiary_initialize();
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.boss_encounter_request(Intent::Catalogue);
    select(&mut app, Tool::Bosses)?;
    outbox.bevy_bosstiary_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: Page {
            next: None,
            points: 0,
            entries: vec![],
        }
        .encode()
        .ok_or("page")?,
    });
    app.world_mut()
        .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>()
        .try_recv()?;
    app.update();
    {
        let mut commands = app
            .world_mut()
            .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<PlayCommand>>();
        assert!(std::iter::from_fn(|| commands.try_recv().ok()).any(|c| matches!(c, PlayCommand::Feature(c) if c.kind == 133 && !c.payload.starts_with(b"BCT1"))));
    }
    Ok(())
}
