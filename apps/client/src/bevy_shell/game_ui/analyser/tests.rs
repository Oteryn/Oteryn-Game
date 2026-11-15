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
    let selector = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == tool.title()).then_some(parent.parent()))
        .ok_or("selector")?;
    app.world_mut().trigger(Activate { entity: selector });
    app.update();
    Ok(())
}
fn report(app: &mut App) -> Result<String, Box<dyn std::error::Error>> {
    Ok(app
        .world_mut()
        .query_filtered::<&Text, With<Report>>()
        .single(app.world())?
        .0
        .clone())
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
