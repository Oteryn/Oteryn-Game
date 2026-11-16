use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_feature_forms_selected();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}

/// One key press through the shell's real router, then one frame.
fn press(app: &mut App, code: u16, modifiers: Modifiers) -> TestResult {
    let event = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(code)?,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    Ok(())
}

fn has_text(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0 == value)
}

fn click(app: &mut App, action: Option<usize>) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.action == action).then_some(entity))
        .ok_or("form button missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}

fn form_open(app: &App, title: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let form = FeatureForm::all()
        .iter()
        .copied()
        .find(|form| form.title(link) == title)
        .ok_or("form missing")?;
    Ok(link.feature_form_view(form).open)
}

#[test]
fn every_form_has_a_field_per_label_and_a_unique_title() -> TestResult {
    let (link, _commands, _outbox) = play_channel();
    let mut titles = std::collections::BTreeSet::new();
    for form in FeatureForm::all() {
        let view = link.feature_form_view(*form);
        assert!(titles.insert(view.title), "duplicate form {}", view.title);
        assert!(!view.open, "{} opens without its key", view.title);
        assert!(
            view.fields.len() >= view.labels.len(),
            "{} has fewer fields than labels",
            view.title
        );
    }
    Ok(())
}

#[test]
fn party_chat_key_opens_form_and_send_reaches_session() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        21,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Party Chat")?);
    assert!(has_text(&mut app, "Party Chat"));
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        assert_eq!(field.0, 0);
        *text = EditableText::new("hello party");
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    let PlayCommand::ChatPublic(_) = commands.try_recv()? else {
        return Err("party chat message missing".into());
    };
    click(&mut app, None)?;
    assert!(!form_open(&app, "Party Chat")?);
    app.update();
    assert!(!has_text(&mut app, "Party Chat"));
    Ok(())
}

#[test]
fn party_effects_key_requests_status_and_disables_buttons_while_pending() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        19,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("party effects request missing".into());
    };
    assert_eq!((command.capability, command.kind), (280, 496));
    assert!(form_open(&app, "Party effects — Ctrl+Alt+Shift+P")?);
    assert!(has_text(&mut app, "Party effects — Ctrl+Alt+Shift+P"));
    assert_eq!(
        app.world_mut()
            .query_filtered::<&Control, With<InteractionDisabled>>()
            .iter(app.world())
            .filter(|control| control.action.is_some())
            .count(),
        1
    );
    // The pending refresh refuses a second press; nothing more reaches the session.
    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn staff_and_cast_keys_open_their_forms_through_the_router() -> TestResult {
    let (mut app, _commands, _outbox) = fixture()?;
    let chord = Modifiers::one(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    for (code, modifiers, title) in [
        (
            39,
            Modifiers::one(Modifier::Control).with(Modifier::Alt),
            "Staff authorized actions",
        ),
        (22, chord, "Cast settings"),
        (11, chord, "Live cast chat"),
    ] {
        press(&mut app, code, modifiers)?;
        assert!(form_open(&app, title)?, "{title} did not open");
        assert!(has_text(&mut app, title), "{title} is not shown");
        click(&mut app, None)?;
        assert!(!form_open(&app, title)?, "{title} did not close");
        app.update();
    }
    Ok(())
}

#[test]
fn cast_settings_reject_invalid_values_without_reaching_the_session() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    press(
        &mut app,
        22,
        Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Cast settings")?);
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 2 {
            *text = EditableText::new("maybe");
        }
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    assert!(commands.try_recv().is_err());
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert_eq!(
        link.cast_control_panel().feedback,
        tr!("Enter valid saved cast settings")
    );
    Ok(())
}

#[test]
fn communication_lists_form_edits_the_chat_lists_it_was_opened_for() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture()?;
    let lists = |app: &App| -> Result<_, Box<dyn std::error::Error>> {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        Ok(link.chat_panel().console.communication)
    };
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.chat_lists_open(Some("Alice"));
    }
    app.update();
    assert!(form_open(&app, "Communication lists")?);
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 0 && text.value().to_string() == "Alice")
    );
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(1))?;
    assert!(lists(&app)?.whitelist.contains("alice"));
    click(&mut app, Some(3))?;
    let after = lists(&app)?;
    assert!(!after.whitelist.contains("alice") && !after.ignored.contains("alice"));
    assert!(commands.try_recv().is_err(), "the lists stay local");
    click(&mut app, None)?;
    assert!(!form_open(&app, "Communication lists")?);
    Ok(())
}

#[test]
fn boss_rewards_key_browses_the_typed_death_and_shows_the_server_rows() -> TestResult {
    use oteryn_client_runtime::boss_rewards::{
        BossRewardsCommand, BossRewardsReply, BossRewardsResponse, CommandStatus, boss_rewards,
    };
    let (mut app, mut commands, outbox) = fixture()?;
    press(
        &mut app,
        30,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?;
    assert!(form_open(&app, "Boss rewards")?);
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            *text = EditableText::new("09".repeat(16));
        }
    }
    while commands.try_recv().is_ok() {}
    click(&mut app, Some(0))?;
    let PlayCommand::BossRewards(BossRewardsCommand::Browse(request)) = commands.try_recv()? else {
        return Err("boss reward browse missing".into());
    };
    assert_eq!(
        request,
        boss_rewards::BrowseRequest {
            death: [9; 16],
            after_ordinal: 0,
        }
    );
    outbox.bevy_boss_rewards_result(&BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Browse(boss_rewards::BrowseReply {
            status: boss_rewards::BossRewardStatus::Ok,
            death: [9; 16],
            expires_at_unix_ms: 99,
            rows: vec![boss_rewards::RewardRow {
                ordinal: 4,
                definition_reference: 700,
                quantity: 3,
            }],
            more: false,
        })),
    });
    app.update();
    let shown = app
        .world_mut()
        .query_filtered::<&Text, With<Summary>>()
        .iter(app.world())
        .any(|text| text.0.contains("#4: item 700 x3") && text.0.contains(&"09".repeat(16)));
    assert!(shown, "the browsed rewards are not shown");
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 1 {
            *text = EditableText::new("4");
        }
    }
    click(&mut app, Some(1))?;
    let PlayCommand::BossRewards(BossRewardsCommand::Take(take)) = commands.try_recv()? else {
        return Err("boss reward take missing".into());
    };
    assert_eq!((take.death, take.ordinal), ([9; 16], 4));
    outbox.bevy_boss_rewards_result(&BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Collect(boss_rewards::CollectReply {
            status: boss_rewards::BossRewardStatus::Ok,
            moved: 1,
            remaining: 0,
        })),
    });
    app.update();
    click(&mut app, Some(3))?;
    let PlayCommand::BossRewards(BossRewardsCommand::CollectAll(all)) = commands.try_recv()? else {
        return Err("boss reward collect all missing".into());
    };
    assert_eq!(all.death, None);
    Ok(())
}
