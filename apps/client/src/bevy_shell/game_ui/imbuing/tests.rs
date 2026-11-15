use super::*;
use oteryn_client_ui::imbue::imbuement;
use std::num::NonZeroU64;
fn panel() -> ImbuePanel {
    ImbuePanel {
        selected: true,
        open: true,
        item: Some(NonZeroU64::MIN),
        success_selected: true,
        success: vec![75],
        window: Some(imbuement::ImbuementWindow {
            slot_count: 2,
            applied: vec![imbuement::AppliedImbuement {
                slot: 0,
                family: 1,
                tier: 2,
                remaining_ms: 123_000,
            }],
            offers: vec![imbuement::ImbuementOffer {
                family: 1,
                tier: 2,
                gold: 500,
                requires_premium: false,
            }],
            clear_gold: 100,
        }),
        ..Default::default()
    }
}
#[test]
fn imbuing_slots_from_model() {
    let slots = slot_rows(&panel());
    assert_eq!(slots.len(), 2);
    assert!(slots[0].contains("123 seconds"));
    assert!(slots[1].contains("empty"));
}
#[test]
fn imbuing_list_costs() {
    let p = panel();
    let rows = offer_rows(&p, Some(2));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, 0);
    assert!(rows[0].1.contains("500 gold"));
    assert!(rows[0].1.contains("75%"));
    assert!(offer_rows(&p, Some(3)).is_empty());
}
#[test]
fn imbuing_clear_needs_confirm() {
    let p = panel();
    let mut selection = Selection::default();
    assert_eq!(selection.intent(&p, Kind::Clear), None);
    assert_eq!(selection.intent(&p, Kind::Confirm), Some(Intent::Clear));
    assert_eq!(selection.intent(&p, Kind::Confirm), None);
    selection.intent(&p, Kind::Clear);
    let mut changed = p.clone();
    changed.slot = 1;
    assert_eq!(selection.intent(&changed, Kind::Confirm), None);
}
#[test]
fn imbuing_apply_guard() {
    let mut p = panel();
    let mut selection = Selection::default();
    assert_eq!(selection.intent(&p, Kind::Apply), None);
    p.slot = 1;
    assert_eq!(selection.intent(&p, Kind::Apply), None);
    selection.slot = Some(1);
    selection.offer = Some(0);
    assert_eq!(selection.intent(&p, Kind::Apply), Some(Intent::Apply));
    p.pending = true;
    assert_eq!(selection.intent(&p, Kind::Apply), None);
}
#[test]
fn imbuing_widgets_render_model_and_disable_apply_until_selected()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let mut shown = panel();
    shown.feedback = Some("Insufficient funds");
    app.insert_resource(TestPanel(shown));
    app.add_systems(Update, render_fixture);
    app.update();
    let world = app.world_mut();
    let text: Vec<_> = world
        .query::<&Text>()
        .iter(world)
        .map(|text| text.0.clone())
        .collect();
    assert!(text.iter().any(|text| text.contains("123 seconds")));
    assert!(
        text.iter()
            .any(|text| text.contains("500 gold") && text.contains("75%"))
    );
    assert!(text.iter().any(|text| text == "Insufficient funds"));
    assert!(
        text.iter()
            .any(|text| text.contains("Apply total: 500 gold")
                && text.contains("Success: 75% — failure: 25%"))
    );
    assert!(
        text.iter()
            .any(|text| text.contains("Protection: unavailable"))
    );
    assert!(
        text.iter()
            .any(|text| text.contains("Balance: not supplied"))
    );
    let mut controls = world.query::<(&Control, Option<&bevy::ui::InteractionDisabled>)>();
    assert!(
        controls
            .iter(world)
            .any(|(control, disabled)| matches!(control.kind, Kind::Apply) && disabled.is_some())
    );
    Ok(())
}
#[derive(Resource)]
struct TestPanel(ImbuePanel);
fn render_fixture(mut commands: Commands, skins: Res<Skins>, panel: Res<TestPanel>) {
    widgets::build(
        &mut commands,
        &skins,
        0,
        1,
        &panel.0,
        &[],
        &Selection {
            offer: Some(panel.0.offer),
            ..Default::default()
        },
    );
}

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    HostOwner,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
fn host() -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    outbox.bevy_imbue_initialize();
    Ok((owner, commands, outbox))
}
fn publish(outbox: &crate::play::Outbox, panel: &ImbuePanel) {
    outbox.bevy_imbue_result(&oteryn_client_runtime::imbue::ImbueResponse {
        status: oteryn_client_runtime::imbue::CommandStatus::Accepted,
        success: panel.success.clone(),
        reply: Some(imbuement::ImbuementReply {
            result: imbuement::ImbuementResult::Ok,
            window: panel.window.clone(),
        }),
    });
}
fn current(owner: &HostOwner) -> State {
    State {
        generation: owner.generation(),
        serial: 1,
        panel: owner.play.as_ref().map(|play| play.link.imbue_panel()),
        ..Default::default()
    }
}
fn click(owner: &HostOwner, state: &mut State, kind: Kind) {
    perform(
        owner,
        state,
        &Control {
            generation: owner.generation(),
            serial: state.serial,
            kind,
        },
    );
    state.panel = owner.play.as_ref().map(|play| play.link.imbue_panel());
}
#[test]
fn imbuing_shortcut_requests_window_and_respects_input_fence() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut owner, mut commands, _) = host()?;
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .imbue_item(NonZeroU64::MIN);
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(12)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    owner.route_input(&[key], false);
    let crate::combat_input::PlayCommand::Imbue(command) = commands.try_recv()? else {
        return Err("query missing".into());
    };
    assert_eq!(command.0.action, imbuement::ImbuementAction::Query);
    assert!(owner.play.as_ref().ok_or("play")?.link.imbue_panel().open);
    Ok(())
}
#[test]
fn imbuing_commands_require_confirmation_and_show_result() -> TestResult {
    let (owner, mut commands, outbox) = host()?;
    let link = &owner.play.as_ref().ok_or("play")?.link;
    link.imbue_item(NonZeroU64::MIN);
    link.open_imbue();
    let _query = commands.try_recv()?;
    publish(&outbox, &panel());
    let mut state = current(&owner);
    click(&owner, &mut state, Kind::Slot(0));
    click(&owner, &mut state, Kind::Clear);
    assert!(commands.try_recv().is_err());
    click(&owner, &mut state, Kind::Cancel);
    click(&owner, &mut state, Kind::Confirm);
    assert!(commands.try_recv().is_err());
    click(&owner, &mut state, Kind::Clear);
    click(&owner, &mut state, Kind::Confirm);
    let crate::combat_input::PlayCommand::Imbue(command) = commands.try_recv()? else {
        return Err("clear missing".into());
    };
    assert_eq!(
        command.0.action,
        imbuement::ImbuementAction::Clear { slot: 0 }
    );
    publish(&outbox, &panel());
    state = current(&owner);
    click(&owner, &mut state, Kind::Slot(1));
    click(&owner, &mut state, Kind::Apply);
    assert!(commands.try_recv().is_err());
    click(&owner, &mut state, Kind::Offer(0));
    click(&owner, &mut state, Kind::Apply);
    let crate::combat_input::PlayCommand::Imbue(command) = commands.try_recv()? else {
        return Err("apply missing".into());
    };
    assert_eq!(
        command.0.action,
        imbuement::ImbuementAction::Apply {
            slot: 1,
            imbuement: 0
        }
    );
    outbox.bevy_imbue_result(&oteryn_client_runtime::imbue::ImbueResponse {
        status: oteryn_client_runtime::imbue::CommandStatus::Accepted,
        success: vec![],
        reply: Some(imbuement::ImbuementReply {
            result: imbuement::ImbuementResult::InsufficientFunds,
            window: None,
        }),
    });
    assert_eq!(link.imbue_panel().feedback, Some("Insufficient funds"));
    Ok(())
}
#[test]
fn imbuing_stale_controls_cannot_send_or_confirm() -> TestResult {
    let (owner, mut commands, outbox) = host()?;
    let link = &owner.play.as_ref().ok_or("play")?.link;
    link.imbue_item(NonZeroU64::MIN);
    link.open_imbue();
    let _query = commands.try_recv()?;
    publish(&outbox, &panel());
    let mut state = current(&owner);
    let stale = state.panel.clone().ok_or("panel")?;
    link.imbue_slot(1);
    assert!(!link.imbue_dispatch_bound(&stale, ImbueViewAction::Request(Intent::Clear)));
    click(&owner, &mut state, Kind::Clear);
    assert!(state.selection.confirmation.is_none());
    perform(
        &owner,
        &mut state,
        &Control {
            generation: owner.generation().saturating_add(1),
            serial: 1,
            kind: Kind::Refresh,
        },
    );
    perform(
        &owner,
        &mut state,
        &Control {
            generation: owner.generation(),
            serial: 0,
            kind: Kind::Refresh,
        },
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn imbuement_tracker_shortcut_opens_independently_in_bevy() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut owner, _, _) = host()?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(12)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .imbue_panel()
            .tracker_open
    );
    owner.route_input(&[key], false);
    let panel = owner.play.as_ref().ok_or("play")?.link.imbue_panel();
    assert!(panel.tracker_open);
    assert!(!panel.open);
    Ok(())
}
