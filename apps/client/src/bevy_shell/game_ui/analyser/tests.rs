use super::*;
use crate::{
    bevy_shell::{
        owner::{ActivePlay, HostOwner, Phase},
        ui,
    },
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, prelude::*, ui_widgets::Activate};
use oteryn_client_ui::analyser::Tool;
use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app() -> Result<(App, crate::play::Outbox), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>();
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    app.insert_non_send(commands);
    link.use_shared_frontend();
    outbox.bevy_hud_fixture(&[], None);
    use oteryn_session::analyser_transport::AnalyserItemCount;
    use std::num::{NonZeroU16, NonZeroU32};
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Experience {
                raw: 100,
                gained: 150,
            },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![AnalyserItemCount {
                    item_type: NonZeroU32::new(202).ok_or("item")?,
                    count: NonZeroU16::new(3).ok_or("count")?,
                }],
                gold: 77,
            },
            AnalyserFact::SupplyUsed(AnalyserItemCount {
                item_type: NonZeroU32::new(203).ok_or("item")?,
                count: NonZeroU16::new(2).ok_or("count")?,
            }),
        ],
    })]);
    host.phase = Phase::Playing;
    host.play = Some(ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    app.insert_non_send(host);
    app.add_systems(Startup, (ui::setup, super::super::setup).chain());
    super::super::hud::install(&mut app);
    install(&mut app);
    app.update();
    // The full shell admission projection normally reveals this root.
    app.world_mut()
        .query_filtered::<&mut Node, With<super::super::GameRoot>>()
        .single_mut(app.world_mut())?
        .display = Display::Flex;
    Ok((app, outbox))
}
fn select(app: &mut App, tool: Tool) -> TestResult {
    // The selector button's caption; the skills widget also has an "Experience" row label.
    let selector = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .filter(|(_, parent)| {
            app.world()
                .get::<bevy::input_focus::tab_navigation::TabIndex>(parent.parent())
                .is_some()
        })
        .find_map(|(text, parent)| (text.0 == tool.title()).then_some(parent.parent()))
        .ok_or("selector")?;
    app.world_mut().trigger(Activate { entity: selector });
    app.update();
    Ok(())
}
fn report(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    let tool = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .analyser_panel()
        .tool;
    app.world_mut()
        .query_filtered::<(Entity, &Text), With<Report>>()
        .iter(app.world())
        .find_map(|(entity, text)| {
            let mut current = Some(entity);
            while let Some(entity) = current {
                if app
                    .world()
                    .get::<Overlay>(entity)
                    .is_some_and(|overlay| overlay.0 == tool)
                {
                    return Some(text.0.clone());
                }
                current = app.world().get::<ChildOf>(entity).map(ChildOf::parent);
            }
            None
        })
        .ok_or("selected analyser report".into())
}
fn fields(tool: Tool, expected: &[&str]) -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, tool)?;
    let text = report(&mut app)?;
    for field in expected {
        assert!(text.contains(field), "{tool:?} missing {field}: {text}");
    }
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Overlay>>()
            .iter(app.world())
            .count(),
        1
    );
    Ok(())
}
macro_rules! tool_test {
    ($name:ident, $tool:ident, $($field:literal),+) => {
        #[test] fn $name() -> TestResult { fields(Tool::$tool, &[$($field),+]) }
    };
}
tool_test!(
    analyser_overview_fields,
    Overview,
    "Kills: 23",
    "Experience: 100 raw / 150 gained",
    "Loot: 3 items / 77 gold",
    "Supplies used: 2"
);
tool_test!(
    analyser_hunting_fields,
    Hunting,
    "Kills: 23",
    "Kills/h:",
    "Session clock:",
    "XP/h:",
    "Loot value/h:",
    "Supply cost/h:",
    "Priced balance:"
);
tool_test!(
    analyser_xp_fields,
    Experience,
    "Elapsed:",
    "XP/h:",
    "Raw XP/h:"
);
tool_test!(
    analyser_loot_fields,
    Loot,
    "Valued loot: 77",
    "Loot value/h:",
    "Unpriced items: 3",
    "Item 202: 3"
);
tool_test!(
    analyser_supply_fields,
    Supplies,
    "Supplies used: 2",
    "Supply cost:",
    "Supply cost/h:",
    "Item 203: 2"
);
tool_test!(
    analyser_damage_fields,
    Damage,
    "Damage dealt:",
    "Damage received:",
    "DPS:"
);
tool_test!(analyser_healing_fields, Healing, "Healing:", "HPS:");
tool_test!(
    analyser_impact_fields,
    Impact,
    "DPS:",
    "HPS:",
    "Damage dealt:",
    "Healing given:"
);
tool_test!(
    analyser_input_fields,
    Input,
    "Incoming DPS:",
    "Damage types",
    "Sources"
);
tool_test!(
    analyser_session_fields,
    Session,
    "Session clock:",
    "XP/h:",
    "Loot value/h:",
    "Supply cost/h:",
    "Priced balance:"
);
tool_test!(analyser_party_fields, Party, "Party");
tool_test!(analyser_loot_tracker_fields, LootTracker, "Loot");
tool_test!(analyser_kills_fields, Kills, "Kill");
tool_test!(analyser_bosses_fields, Bosses, "Boss");

fn control(app: &mut App, reset: bool) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, action)| (action.reset == reset).then_some(entity))
        .ok_or("control".into())
}
#[test]
fn analyser_reset() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::Hunting)?;
    let reset = control(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    assert!(report(&mut app)?.contains("Kills: 0"));
    let host = app.world().non_send::<HostOwner>();
    let panel = host.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(panel.tool, Tool::Hunting);
    assert!(panel.open);
    Ok(())
}
#[test]
fn analyser_live_update_and_close() -> TestResult {
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::Hunting)?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Kill { race: None }],
    })]);
    app.update();
    assert!(report(&mut app)?.contains("Kills: 24"));
    let close = control(&mut app, false)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Overlay>>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}

#[test]
fn analyser_live_damage_sources() -> TestResult {
    use oteryn_session::analyser_transport::{AnalyserDamageSource, AnalyserImpact};
    use std::num::NonZeroU32;
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::Input)?;
    let amount = NonZeroU32::new(120).ok_or("amount")?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::DamageInput {
                value: amount,
                element: oteryn_protocol_oteryn::damage_element::DamageElement::Fire,
                source: AnalyserDamageSource::Creature(NonZeroU32::new(42)),
            },
            AnalyserFact::Impact(AnalyserImpact::Damage {
                value: amount,
                element: oteryn_protocol_oteryn::damage_element::DamageElement::Fire,
            }),
            AnalyserFact::Impact(AnalyserImpact::Healing(amount)),
        ],
    })]);
    app.update();
    let text = report(&mut app)?;
    assert!(text.contains("Creature race #42: 120"));
    assert!(text.contains("Fire: 120"));
    assert!(text.contains("Damage received: 120"));
    // Select through the retained owner while the modal fences the background selector.
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .analyser_select("impact")
    );
    app.update();
    let text = report(&mut app)?;
    assert!(text.contains("Damage dealt: 120"));
    assert!(text.contains("Healing given: 120"));
    Ok(())
}

#[test]
fn analyser_shortcut_and_stale_reset() -> TestResult {
    let (mut app, _outbox) = app()?;
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(28)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), true);
    app.update();
    assert!(report(&mut app).is_err());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(report(&mut app)?.contains("Kills: 23"));
    let reset = control(&mut app, true)?;
    app.world_mut()
        .get_mut::<Control>(reset)
        .ok_or("reset")?
        .generation = u64::MAX;
    app.world_mut().trigger(Activate { entity: reset });
    app.update();
    assert!(report(&mut app)?.contains("Kills: 23"));
    Ok(())
}

fn click_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or("tracker button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn tracker_item(app: &mut App, value: &str) -> TestResult {
    let mut field = app
        .world_mut()
        .query::<&mut bevy::text::EditableText>()
        .single_mut(app.world_mut())?;
    *field = bevy::text::EditableText::new(value);
    Ok(())
}
#[test]
fn analyser_tracker_controls_filter_confirmed_drops_and_keep_selection() -> TestResult {
    let (mut app, outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    click_caption(&mut app, "Show tracked only")?;
    assert!(!report(&mut app)?.contains("#202"));
    tracker_item(&mut app, "202")?;
    click_caption(&mut app, "Track item")?;
    let text = report(&mut app)?;
    assert!(text.contains("#202 Name unresolved (tracked)"));
    assert!(text.contains("3 confirmed items"));
    assert!(text.contains("#202 × 3"));
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 2,
        base_revision: 2,
        new_revision: 3,
        value: vec![AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![oteryn_session::analyser_transport::AnalyserItemCount {
                item_type: std::num::NonZeroU32::new(204).ok_or("item")?,
                count: std::num::NonZeroU16::MIN,
            }],
            gold: 0,
        }],
    })]);
    app.update();
    assert!(!report(&mut app)?.contains("#204"));

    click_caption(&mut app, "Untrack item")?;
    assert!(!report(&mut app)?.contains("#202"));
    click_caption(&mut app, "Show all drops")?;
    assert!(report(&mut app)?.contains("#202 Name unresolved (observed)"));
    assert!(report(&mut app)?.contains("#204 Name unresolved (observed)"));
    tracker_item(&mut app, "999")?;
    click_caption(&mut app, "Track item")?;
    assert!(report(&mut app)?.contains("Choose an item from confirmed loot"));
    tracker_item(&mut app, "202")?;
    click_caption(&mut app, "Track item")?;
    click_caption(&mut app, "Close")?;
    select(&mut app, Tool::LootTracker)?;
    assert!(report(&mut app)?.contains("(tracked)"));
    click_caption(&mut app, "Reset session")?;
    assert!(report(&mut app)?.contains("(tracked) · no confirmed drops"));
    Ok(())
}

#[test]
fn analyser_tracker_rejects_stale_controls_and_preserves_counts() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    tracker_item(&mut app, "202")?;
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Track item").then_some(parent.parent()))
        .ok_or("track")?;
    // A queued activation from a previous session must not alter this session.
    app.world_mut()
        .get_mut::<tracker::TrackerControl>(button)
        .ok_or("control")?
        .generation = u64::MAX;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(!report(&mut app)?.contains("(tracked)"));
    assert!(report(&mut app)?.contains("3 confirmed items"));
    let host = app.world().non_send::<HostOwner>();
    let panel = host.play.as_ref().ok_or("play")?.link.analyser_panel();
    assert_eq!(panel.loot_items, 3);
    assert_eq!(panel.gold, 77);
    Ok(())
}

#[test]
fn analyser_tracker_waits_for_pending_text_edits() -> TestResult {
    let (mut app, _outbox) = app()?;
    select(&mut app, Tool::LootTracker)?;
    tracker_item(&mut app, "202")?;
    app.world_mut()
        .query::<&mut bevy::text::EditableText>()
        .single_mut(app.world_mut())?
        .pending_edits
        .push(bevy::text::TextEdit::Paste);
    click_caption(&mut app, "Track item")?;
    assert!(!report(&mut app)?.contains("(tracked)"));
    // Resolve the edit without another activation; the original action completes.
    tracker_item(&mut app, "202")?;
    app.update();
    assert!(report(&mut app)?.contains("(tracked)"));
    Ok(())
}

#[test]
fn analyser_window_does_not_fence_gameplay() -> TestResult {
    let (mut app, _) = app()?;
    select(&mut app, Tool::Loot)?;
    let overlay_entity = app
        .world_mut()
        .query_filtered::<Entity, With<Overlay>>()
        .single(app.world())?;
    assert!(
        app.world()
            .get::<super::super::GameModal>(overlay_entity)
            .is_none()
    );
    let overlay = app
        .world_mut()
        .query_filtered::<&Node, With<Overlay>>()
        .single(app.world())?;
    assert_ne!(overlay.width, percent(100.));
    let outside = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(outside, bevy::input_focus::FocusCause::Navigated);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(
        report(&mut app).is_ok(),
        "a nonmodal analyser must not consume another control's Escape"
    );
    Ok(())
}

#[test]
fn party_hunt_open_requests_current_accounting() -> TestResult {
    let (mut app, outbox) = app()?;
    outbox.bevy_party_selected(true, true, "Leader");
    select(&mut app, Tool::Party)?;
    let commands = &mut *app
        .world_mut()
        .non_send_mut::<tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>>();
    assert!(std::iter::from_fn(|| commands.try_recv().ok()).any(|command| matches!(command, crate::combat_input::PlayCommand::Feature(command) if command.kind == 735)));
    Ok(())
}

#[test]
fn boss_tracker_query_controls_reach_retained_owner() -> TestResult {
    let (mut app, _) = app()?;
    select(&mut app, Tool::Bosses)?;
    assert!(report(&mut app)?.contains("Personal drop chests"));
    let button = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Refresh cooldowns").then_some(parent.parent()))
        .ok_or("cooldown refresh control missing")?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .boss_encounter_panel()
            .feedback
            .is_some()
    );
    Ok(())
}

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
    let price = find(&mut app, "Set loot value")?;
    app.world_mut().trigger(Activate { entity: price });
    app.update();
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
