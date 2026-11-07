use crate::input::StepDir;
use crate::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{ActorVitals, AppliedDelta, SessionEvent};

#[test]
fn death_blocks_input_and_respawn_does_not_replay_pending_actions() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    let event = |health| {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })
    };
    view.arrow(StepDir::North);
    outbox.push(vec![event(0)]);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(view.hud_status().contains("You are dead"));
    link.request_combat(crate::combat_input::CombatCommand::Attack(
        oteryn_session::EntityRef {
            identity: [7; 16],
            generation: 1,
        },
    ));
    link.request(StepDir::West);
    view.arrow(StepDir::East);
    view.cast(std::num::NonZeroU32::new(1).ok_or("spell")?);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_err());
    assert!(outbox.death_command(&mut commands).is_err());
    outbox.push(vec![event(150)]);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.hud_status().contains("You are dead"));
    assert!(commands.try_recv().is_err());
    view.arrow(StepDir::South);
    assert_eq!(view.tick(&link), Ok(()));
    assert!(commands.try_recv().is_ok());
    Ok(())
}

#[test]
fn coalesced_death_resets_targets_and_both_input_queues() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    link.request(StepDir::North);
    view.arrow(StepDir::West);
    view.in_flight = true;
    let target = crate::input::Targetable {
        tile: TileCoord::new(2, 1),
        kind: crate::input::TargetKind::Entity,
        glyph: true,
        entity: Some(oteryn_session::EntityRef {
            identity: [7; 16],
            generation: 1,
        }),
        item: None,
    };
    view.scene = crate::scene::Scene::centered_with(
        std::sync::Arc::new(crate::map::World::builtin().map_err(|e| e.to_string())?),
        None,
        TileCoord::new(1, 1),
        StepDir::South,
        &[],
        &[target],
    )
    .map_err(|e| e.to_string())?;
    view.scene
        .select_tile(target.tile)
        .map_err(|e| e.to_string())?;
    assert!(view.scene.target().is_some());
    for health in [0, 150] {
        outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: ActorVitals {
                health,
                max_health: 150,
                ..Default::default()
            },
        })]);
    }
    assert!(outbox.death_command(&mut commands).is_err());
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.death.dead);
    assert!(!view.in_flight);
    assert!(view.scene.target().is_none());
    assert!(view.inputs.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn default_vitals_are_not_death_and_death_clears_target_walk() {
    let (_, _, outbox) = play_channel();
    let mut mailbox = crate::play::mailbox::lock(&outbox.0);
    mailbox.apply_death_vitals(ActorVitals::default());
    assert!(!mailbox.death.dead);
    mailbox.target_walk.pending = true;
    mailbox.target_walk.feedback = Some("old target");
    mailbox.apply_death_vitals(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    });
    assert!(mailbox.death.dead);
    assert!(!mailbox.target_walk.pending);
    assert!(mailbox.target_walk.feedback.is_none());
}

#[test]
fn queued_world_actions_are_discarded_while_logout_remains_available() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    link.request_combat(crate::combat_input::CombatCommand::Cast(
        std::num::NonZeroU32::new(1).ok_or("spell")?,
    ));
    crate::play::mailbox::lock(&outbox.0).daily_reward.selected = true;
    link.daily_reward_request(oteryn_client_ui::daily_reward::Intent::Refresh);
    assert!(crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    link.request_logout();
    link.request(StepDir::North);
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    assert!(!crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    link.daily_reward_request(oteryn_client_ui::daily_reward::Intent::Refresh);
    assert!(!crate::play::mailbox::lock(&outbox.0).daily_reward.pending);
    assert_eq!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Logout)
    );
    assert!(outbox.death_command(&mut commands).is_err());
    Ok(())
}

#[test]
fn coalesced_respawn_before_reset_drain_cannot_leave_movement_pending() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    outbox.target_walk_initialize(true);
    for health in [0, 150] {
        outbox.vitals(Some(ActorVitals {
            health,
            max_health: 150,
            ..Default::default()
        }));
    }
    assert_eq!(view.tick(&link), Ok(()));
    view.arrow(StepDir::East);
    assert_eq!(view.tick(&link), Ok(()));
    // The view runs before the session loop drains its reset barrier.
    assert!(!view.in_flight);
    assert!(commands.try_recv().is_err());
    assert!(link.request_target_walk(oteryn_session::ActorPosition {
        x: 2,
        y: 1,
        floor: 0
    }));
    assert!(!crate::play::mailbox::lock(&outbox.0).target_walk.pending);
    assert!(outbox.death_command(&mut commands).is_err());
    assert_eq!(view.tick(&link), Ok(()));
    view.arrow(StepDir::East);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Step(StepDir::East))
    );
    outbox.stepped(crate::play::tests::moved_to(2, 1, 0));
    assert_eq!(view.tick(&link), Ok(()));
    assert!(!view.in_flight);
    assert!(link.request_target_walk(oteryn_session::ActorPosition {
        x: 3,
        y: 1,
        floor: 0
    }));
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::TargetWalk(_))
    ));
    Ok(())
}

#[test]
fn death_notice_preserves_logout_feedback_and_server_warning() -> Result<(), String> {
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.logout_line = Some("Logout refused".into());
    outbox.vitals(Some(ActorVitals {
        health: 0,
        max_health: 150,
        ..Default::default()
    }));
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: oteryn_session::ServerMessage::Warning("Server warning".into()),
    })]);
    assert_eq!(view.tick(&link), Ok(()));
    let status = view.hud_status();
    assert!(status.contains("You are dead"));
    assert!(status.contains("Logout refused"));
    assert!(status.contains("Server warning"));
    Ok(())
}

#[test]
fn death_protection_is_requested_at_login_and_after_respawn_and_shown_in_hud() -> Result<(), String>
{
    use oteryn_session::death_protection::{DeathProtection, DeathProtectionReply};
    let (link, mut commands, outbox) = play_channel();
    outbox.death_protection_initialize(true);
    let command = outbox
        .death_command(&mut commands)
        .map_err(|e| e.to_string())?;
    assert!(
        matches!(command, crate::combat_input::PlayCommand::Feature(ref value) if value.capability == 382 && value.kind == 664)
    );
    let value = DeathProtectionReply(Some(DeathProtection {
        occurrence: [7; 16],
        consumed: vec!["oteryn:blessing.twist_of_fate".into()],
    }));
    outbox.death_protection_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: value.encode().ok_or("encode")?,
    });
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(view.hud_status().contains("Twist Of Fate"));
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .death
            .protection
            .as_ref()
            .ok_or("death receipt")?
            .occurrence,
        [7; 16]
    );
    for health in [0, 150] {
        outbox.vitals(Some(ActorVitals {
            health,
            max_health: 150,
            ..Default::default()
        }));
    }
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Feature(_))
    ));
    outbox.death_protection_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![0, 1],
    });
    assert!(outbox.death_command(&mut commands).is_err());
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(
        view.hud_status().contains("Twist Of Fate"),
        "malformed reply must not invent consumption"
    );
    Ok(())
}

#[test]
fn coalesced_alive_vitals_refreshes_death_receipt_without_query_feedback() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    outbox.death_protection_initialize(true);
    assert!(outbox.death_command(&mut commands).is_ok());
    let result = |payload| oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload,
    };
    outbox.death_protection_result(&result(vec![0]));
    let alive = ActorVitals {
        health: 150,
        max_health: 150,
        ..Default::default()
    };
    // Publishing the cached current vitals after a command never causes feedback.
    outbox.vitals(Some(alive));
    assert!(outbox.death_command(&mut commands).is_err());
    // The wire may coalesce dead and respawn revisions into one alive delta.
    outbox.push(vec![SessionEvent::ActorVitals(AppliedDelta {
        server_sequence: 2,
        base_revision: 1,
        new_revision: 3,
        value: alive,
    })]);
    assert!(matches!(
        outbox.death_command(&mut commands),
        Ok(crate::combat_input::PlayCommand::Feature(_))
    ));
    assert!(
        outbox.death_command(&mut commands).is_err(),
        "single query in flight"
    );
    let value = oteryn_session::death_protection::DeathProtectionReply(Some(
        oteryn_session::death_protection::DeathProtection {
            occurrence: [8; 16],
            consumed: vec!["oteryn:blessing.spiritual_shielding".into()],
        },
    ));
    outbox.death_protection_result(&result(value.encode().ok_or("encode")?));
    outbox.vitals(Some(alive));
    assert!(outbox.death_command(&mut commands).is_err());
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[]).map_err(|e| e.to_string())?;
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert!(view.hud_status().contains("Spiritual Shielding"));
    assert_eq!(
        crate::play::mailbox::lock(&outbox.0)
            .death
            .protection
            .as_ref()
            .ok_or("death receipt")?
            .occurrence,
        [8; 16]
    );
    Ok(())
}
