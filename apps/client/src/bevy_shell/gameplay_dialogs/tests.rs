use super::*;
use crate::play::{PlayView, play_channel};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn shared_logout_cancel_duplicate_and_stale_confirm_reach_existing_queue() -> Result {
    // Test-only play fixture; shipping admission still comes exclusively from Reentry.
    let mut host = HostOwner::new(None)?;
    let (link, mut commands, _events) = play_channel();
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = Phase::Playing;
    host.request_logout_confirmation();
    let first = host.dialogs.pending.as_ref().ok_or("confirmation")?.id;
    assert!(host.choose_dialog(first, Choice::Cancel));
    assert!(!host.dialogs.active());
    assert!(commands.try_recv().is_err());
    host.request_logout_confirmation();
    let second = host
        .dialogs
        .pending
        .as_ref()
        .ok_or("second confirmation")?
        .id;
    assert!(!host.choose_dialog(first, Choice::Logout));
    assert!(
        host.dialogs.active(),
        "late click must not dismiss a newer dialog"
    );
    assert!(host.choose_dialog(second, Choice::Logout));
    assert!(!host.choose_dialog(second, Choice::Logout));
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("unexpected end {end:?}"))?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Logout
    ));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn previous_generation_and_wrong_context_target_cannot_dispatch() -> Result {
    let mut host = HostOwner::new(None)?;
    let (link, mut commands, _events) = play_channel();
    host.play = Some(super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = Phase::Playing;
    host.request_logout_confirmation();
    let request = host.dialogs.pending.as_mut().ok_or("confirmation")?;
    request.generation = request.generation.saturating_add(1);
    let id = request.id;
    assert!(!host.choose_dialog(id, Choice::Logout));
    let menu = Menu {
        tile: Some(TileCoord::new(i32::MAX, i32::MAX)),
        binding: None,
        target: oteryn_client_runtime::world_interaction::Target::Position(
            oteryn_session::ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        ),
        entries: vec![(
            "Look",
            oteryn_client_runtime::world_interaction::Action::Look,
        )],
    };
    host.dialogs.open(
        host.generation(),
        Content::Context {
            menu: Some(Box::new(menu)),
            party: None,
            inspection: None,
            report: None,
            trade: None,
            browse: None,
            npc: None,
            player: None,
            attack: None,
        },
    );
    let id = host.dialogs.pending.as_ref().ok_or("context")?.id;
    assert!(!host.choose_dialog(id, Choice::Context(0)));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn context_menu_follows_tibia_order_with_shortcuts_and_uses_a_bed() -> Result {
    use oteryn_client_runtime::world_interaction::{
        Action, Target, world_object::WorldObjectTarget,
    };
    let tile = TileCoord::new(100, 100);
    let menu = Menu {
        tile: Some(tile),
        binding: None,
        target: Target::Object(WorldObjectTarget {
            placement: vec![1; 40],
            expected_revision: 0,
        }),
        entries: vec![
            ("Look", Action::Look),
            ("Use", Action::Use),
            ("Open", Action::Open),
            ("Bed", Action::BedInteract),
        ],
    };
    let content = Content::Context {
        menu: Some(Box::new(menu)),
        party: None,
        inspection: None,
        report: None,
        trade: None,
        browse: Some((tile, 0)),
        npc: None,
        player: Some("Bryn".into()),
        attack: Some((
            tile,
            (
                oteryn_session::EntityRef {
                    identity: [1; 16],
                    generation: 1,
                },
                false,
            ),
        )),
    };
    let items = projection::context_items(&content, true, false);
    let rows: Vec<String> = items
        .iter()
        .map(|(item, _)| match item {
            super::popup::Item::Entry {
                label, shortcut, ..
            } => shortcut.map_or_else(|| label.clone(), |key| format!("{label}\t{key}")),
            super::popup::Item::Separator => "-".into(),
        })
        .collect();
    assert_eq!(
        rows,
        [
            "Attack\tAlt",
            "Look\tShift",
            "Use\tCtrl",
            "Open",
            "Bed",
            "-",
            "Rotate",
            "Browse Field",
            "-",
            "Message to Bryn",
            "Add to VIP list",
            "Ignore Bryn",
            "-",
            "Copy Name",
        ]
    );
    // Plain Use stays reachable on a bed, and the bed action keeps its own row.
    assert!(matches!(items[2].1, Choice::Context(1)));
    assert!(matches!(items[4].1, Choice::Context(3)));
    assert!(matches!(items[0].1, Choice::Attack));
    let Content::Context { menu, .. } = &content else {
        return Err("context".into());
    };
    let current = Content::Context {
        menu: menu.clone(),
        party: None,
        inspection: None,
        report: None,
        trade: None,
        browse: None,
        npc: None,
        player: None,
        attack: Some((
            tile,
            (
                oteryn_session::EntityRef {
                    identity: [1; 16],
                    generation: 1,
                },
                true,
            ),
        )),
    };
    let rows = projection::context_items(&current, false, false);
    assert!(matches!(
        &rows[0].0,
        super::popup::Item::Entry { label, .. } if label == "Stop Attack"
    ));
    Ok(())
}

#[test]
fn context_attack_rejects_a_replacement_target_or_changed_combat_state() -> Result {
    use oteryn_session::{
        ActorCombatState, ActorPosition, EntityDetail, EntityKind, EntityRef, FightModes,
        StepDirection, WorldSpatialEntity,
    };

    let own = TileCoord::new(100, 100);
    let tile = TileCoord::new(101, 100);
    let creature = WorldSpatialEntity {
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        kind: EntityKind::Creature,
        position: ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: 7,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
        motion: None,
        display_name: None,
    };
    for attacking in [false, true] {
        let mut host = HostOwner::new(None)?;
        let (link, mut commands, outbox) = play_channel();
        outbox.bevy_world_look_selected();
        let combat = attacking.then_some(ActorCombatState {
            target: Some(creature.entity),
            modes: FightModes::DEFAULT,
            in_fight: true,
        });
        outbox.battle_publish(combat, Some(7), std::slice::from_ref(&creature));
        host.play = Some(super::super::owner::ActivePlay {
            view: PlayView::new(own, 7, &[])?.with_entities(vec![creature])?,
            link,
        });
        host.phase = Phase::Playing;
        host.request_world_context(tile);
        let request = host.dialogs.pending.as_ref().ok_or("attack menu")?;
        assert!(matches!(
            request.content,
            Content::Context {
                attack: Some(_),
                ..
            }
        ));
        let id = request.id;
        if attacking {
            outbox.battle_publish(None, Some(7), std::slice::from_ref(&creature));
        } else {
            let mut moved = creature;
            moved.position.x += 1;
            let mut replacement = creature;
            replacement.kind = EntityKind::Player;
            replacement.entity.identity = [4; 16];
            host.play.as_mut().ok_or("play")?.view =
                PlayView::new(own, 7, &[])?.with_entities(vec![moved, replacement])?;
        }
        assert!(!host.choose_dialog(id, Choice::Attack));
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
