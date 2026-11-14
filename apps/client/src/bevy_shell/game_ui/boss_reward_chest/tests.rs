use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::boss_rewards::{
    BossRewardsCommand, BossRewardsReply, BossRewardsResponse, boss_rewards,
};
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_boss_reward_chest_selected();
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
fn toggle(app: &mut App) {
    let entity = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.id() == "shell.boss_reward_chest").then_some(entity));
    if let Some(entity) = entity {
        app.world_mut().trigger(Activate { entity });
    }
    app.update();
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn draft(app: &mut App, death: &str, ordinal: &str) {
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        text.editor.set_text(match field {
            Field::Death => death,
            Field::Ordinal => ordinal,
        });
        text.pending_edits.clear();
    }
}
fn panel(app: &App) -> std::result::Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .boss_rewards_panel())
}
fn reply(outbox: &Outbox) {
    outbox.bevy_boss_reward_chest_reply(&BossRewardsResponse {
        status: CommandStatus::Accepted,
        reply: Some(BossRewardsReply::Browse(boss_rewards::BrowseReply {
            status: boss_rewards::BossRewardStatus::Ok,
            death: [9; 16],
            expires_at_unix_ms: 987654,
            rows: vec![boss_rewards::RewardRow {
                ordinal: 4,
                definition_reference: 700,
                quantity: 3,
            }],
            more: true,
        })),
    });
}
fn browse(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result {
    draft(app, &"09".repeat(16), "4");
    click(app, Kind::Request(Intent::Browse))?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::BossRewards(BossRewardsCommand::Browse(value)) if value.death==[9;16] && value.after_ordinal==0)
    );
    Ok(())
}
#[test]
fn boss_reward_chest_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    assert!(panel(&app)?.browse.is_none());
    browse(&mut app, &mut commands)?;
    reply(&outbox);
    app.update();
    let expected = oteryn_client_ui::boss_rewards::panel_text(&panel(&app)?);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert!(expected.contains("#4: item 700 x3"));
    assert!(expected.contains("987654"));
    assert!(expected.contains("More rewards available"));
    Ok(())
}
#[test]
fn boss_reward_chest_actions_are_bound_and_confirmed() -> Result {
    for intent in [Intent::Take, Intent::CollectGroup, Intent::CollectAll] {
        let (mut app, mut commands, outbox) = fixture()?;
        toggle(&mut app);
        browse(&mut app, &mut commands)?;
        reply(&outbox);
        app.update();
        let before = panel(&app)?.browse;
        draft(&mut app, &"09".repeat(16), "4");
        click(&mut app, Kind::Request(intent))?;
        assert!(commands.try_recv().is_err());
        click(&mut app, Kind::Confirm)?;
        let PlayCommand::BossRewards(command) = commands.try_recv()? else {
            return Err("wrong command".into());
        };
        assert_eq!(
            command,
            match intent {
                Intent::Take => BossRewardsCommand::Take(boss_rewards::TakeRequest {
                    death: [9; 16],
                    ordinal: 4
                }),
                Intent::CollectGroup =>
                    BossRewardsCommand::CollectAll(boss_rewards::CollectAllRequest {
                        death: Some([9; 16])
                    }),
                Intent::CollectAll =>
                    BossRewardsCommand::CollectAll(boss_rewards::CollectAllRequest { death: None }),
                Intent::Browse => return Err("unexpected browse".into()),
            }
        );
        assert!(commands.try_recv().is_err());
        assert!(panel(&app)?.pending);
        assert_eq!(panel(&app)?.browse, before);
        click(&mut app, Kind::Request(intent))?;
        assert!(commands.try_recv().is_err());
        outbox.bevy_boss_reward_chest_reply(&BossRewardsResponse {
            status: CommandStatus::Rejected,
            reply: None,
        });
        app.update();
        assert!(!panel(&app)?.pending);
        assert!(panel(&app)?.feedback.is_some());
        assert_eq!(panel(&app)?.browse, before);
    }
    Ok(())
}
#[test]
fn boss_reward_chest_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    browse(&mut app, &mut commands)?;
    reply(&outbox);
    app.update();
    draft(&mut app, &"09".repeat(16), "4");
    click(&mut app, Kind::Request(Intent::Take))?;
    draft(&mut app, &"08".repeat(16), "4");
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    draft(&mut app, &"09".repeat(16), "4");
    click(&mut app, Kind::Request(Intent::Take))?;
    let overlay = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(overlay).despawn();
    app.update();
    let stale = app
        .world()
        .resource::<State>()
        .snapshot
        .clone()
        .ok_or("snapshot")?;
    let (new_link, mut new_commands, new_outbox) = play_channel();
    new_link.use_shared_frontend();
    new_outbox.bevy_boss_reward_chest_selected();
    new_link.boss_reward_chest_dispatch_bound(&new_link.boss_reward_chest_bound(), Action::Toggle);
    new_link.boss_reward_chest_dispatch_bound(
        &new_link.boss_reward_chest_bound(),
        Action::Request {
            intent: Intent::Browse,
            death: [9; 16],
            ordinal: 4,
            confirmed: false,
        },
    );
    assert!(matches!(
        new_commands.try_recv()?,
        PlayCommand::BossRewards(BossRewardsCommand::Browse(_))
    ));
    reply(&new_outbox);
    assert_eq!(new_link.boss_rewards_panel(), stale.bound.panel);
    let old = std::mem::replace(
        &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .link,
        new_link,
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .boss_reward_chest_dispatch_bound(
                &stale.bound,
                Action::Request {
                    intent: Intent::Take,
                    death: [9; 16],
                    ordinal: 4,
                    confirmed: true
                }
            )
    );
    assert!(new_commands.try_recv().is_err());
    old.boss_reward_chest_dispatch_bound(&stale.bound, Action::Close);
    assert!(!old.boss_reward_chest_dispatch_bound(
        &stale.bound,
        Action::Request {
            intent: Intent::Take,
            death: [9; 16],
            ordinal: 4,
            confirmed: true
        }
    ));
    let replacement = panel(&app)?;
    reply(&outbox);
    assert_eq!(panel(&app)?, replacement);
    assert!(!old.boss_rewards_panel().open);
    outbox.bevy_boss_reward_chest_end();
    assert!(!old.boss_reward_chest_dispatch_bound(&stale.bound, Action::Toggle));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn boss_reward_chest_window_registered() -> Result {
    let (mut app, _commands, _outbox) = fixture()?;
    assert!(
        app.world_mut()
            .query::<&crate::bevy_shell::ui::Control>()
            .iter(app.world())
            .any(|c| c.id() == "shell.boss_reward_chest")
    );
    toggle(&mut app);
    assert!(panel(&app)?.open);
    toggle(&mut app);
    assert!(!panel(&app)?.open);
    owner_shortcut(&mut app)?;
    assert!(panel(&app)?.open);
    owner_shortcut(&mut app)?;
    assert!(!panel(&app)?.open);
    toggle(&mut app);
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app)?.open);
    Ok(())
}

fn owner_shortcut(app: &mut App) -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(30)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    assert!(oteryn_client_ui::boss_rewards::open_key(
        std::slice::from_ref(&event)
    ));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), false);
    app.update();
    Ok(())
}
#[test]
fn boss_reward_chest_ordinal_validation_stays_with_owner() -> Result {
    let (mut app, mut commands, _outbox) = fixture()?;
    toggle(&mut app);
    draft(&mut app, &"09".repeat(16), "1001");
    click(&mut app, Kind::Request(Intent::Take))?;
    assert!(app.world().resource::<State>().confirmation.is_some());
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app)?.pending);
    Ok(())
}
#[test]
fn boss_reward_chest_invalid_target_uses_owner_feedback() -> Result {
    let (mut app, mut commands, _outbox) = fixture()?;
    toggle(&mut app);
    draft(&mut app, "", "1");
    click(&mut app, Kind::Request(Intent::Browse))?;
    assert_eq!(
        app.world().resource::<State>().feedback,
        tr!("Enter a valid nonzero death id and reward ordinal")
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
