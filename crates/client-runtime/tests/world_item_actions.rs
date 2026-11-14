//! Exercise hotkey freshness through the real Session stores and runtime send boundary.
#[path = "support/world_item_actions.rs"]
mod support;

use oteryn_client_runtime::world_interaction::{self, Action, Command, Selection, Target};
use oteryn_session::{ActorPosition, container_tree, read_frame, write_frame};
use tokio::io::AsyncReadExt;

use support::{BoxError, ORIGIN, OWN, envelope, field, handle, joined, message, run, scalar};

fn plain_use(source: u64) -> Command {
    Command {
        target: Target::Item(handle(source)),
        action: Action::Use,
        selection: None,
    }
}

fn use_on_self(source: u64) -> Command {
    Command {
        target: Target::Creature(OWN),
        action: Action::UseOn(handle(source)),
        selection: Some(Selection {
            position: ORIGIN,
            binding: None,
        }),
    }
}

async fn accepted(command: Command) -> Result<(), BoxError> {
    accepted_with_equipment(command, None).await
}

async fn accepted_with_equipment(command: Command, equipped: Option<u64>) -> Result<(), BoxError> {
    let (mut session, mut peer) = support::joined_with_equipment(equipped).await?;
    if let Some(source) = equipped {
        assert!(world_interaction::item_current(&session, handle(source)));
    }
    let payload = match command.action {
        Action::UseOn(source) => world_interaction::use_with_payload(source, &command.target)?,
        _ => {
            let Target::Item(handle) = command.target else {
                return Err("expected item target".into());
            };
            world_interaction::world_object::encode_use_item_intent(handle)
        }
    };
    let server = tokio::spawn(async move {
        let expected = envelope(
            7,
            1,
            0,
            &message(&[scalar(1, 7), scalar(2, 2), field(4, &payload)]),
        );
        assert_eq!(read_frame(&mut peer).await?, expected);
        let result = message(&[scalar(1, 7), scalar(2, 1), field(5, &[8, 1])]);
        write_frame(&mut peer, &envelope(8, 1, 41, &result)).await?;
        Ok::<_, BoxError>(peer)
    });
    let expected = if matches!(command.action, Action::UseOn(_)) {
        "Use with: completed"
    } else {
        "Use: completed"
    };
    assert_eq!(
        world_interaction::send(&mut session, &command).await?,
        expected
    );
    let _peer = server.await??;
    Ok(())
}

#[test]
fn plain_use_hotkey_sends_current_inventory_and_open_container_items() -> Result<(), BoxError> {
    run(|| async {
        // The view's container handle may itself be absent from the direct inventory.
        for source in [2, 7, 10] {
            accepted(plain_use(source)).await?;
        }
        Ok(())
    })
}

#[test]
fn use_on_hotkey_sends_current_nested_container_source() -> Result<(), BoxError> {
    run(|| accepted(use_on_self(10)))
}

#[test]
fn queued_container_changes_reject_removed_and_closed_hotkey_sources() -> Result<(), BoxError> {
    run(|| async {
        for close in [false, true] {
            for command in [plain_use(10), use_on_self(10)] {
                let (mut session, mut peer) = joined().await?;
                assert!(world_interaction::item_current(&session, handle(10)));
                let views = if close {
                    container_tree::ContainerViews::default()
                } else {
                    support::views(11)
                };
                let payload = container_tree::encode_container_views(&views)
                    .map_err(|error| format!("container delta: {error:?}"))?;
                let delta = message(&[
                    scalar(1, 14),
                    scalar(2, 1),
                    scalar(3, 2),
                    scalar(4, 1),
                    field(5, &payload),
                ]);
                write_frame(&mut peer, &envelope(9, 1, 41, &delta)).await?;
                assert_eq!(
                    world_interaction::send(&mut session, &command).await?,
                    "Target changed"
                );
                assert!(!world_interaction::item_current(&session, handle(10)));
                assert_eq!(world_interaction::item_current(&session, handle(7)), !close);
                drop(session);
                let mut sent = Vec::new();
                peer.read_to_end(&mut sent).await?;
                assert!(sent.is_empty(), "stale command was written");
            }
        }
        Ok(())
    })
}

#[test]
fn unknown_handles_and_world_targets_without_selection_remain_refused() -> Result<(), BoxError> {
    run(|| async {
        for command in [plain_use(99), plain_use(40), use_on_self(99)] {
            let (mut session, mut peer) = joined().await?;
            // Handle 40 is current in the map, but cannot borrow inventory's no-selection path.
            assert!(world_interaction::item_current(&session, handle(40)));
            assert_eq!(
                world_interaction::send(&mut session, &command).await?,
                "Target changed"
            );
            drop(session);
            let mut sent = Vec::new();
            peer.read_to_end(&mut sent).await?;
            assert!(sent.is_empty());
        }
        Ok(())
    })
}

#[test]
fn nested_source_does_not_bypass_target_position_generation_or_map_binding() -> Result<(), BoxError>
{
    run(|| async {
        let mut moved = use_on_self(10);
        moved.selection = Some(Selection {
            position: ActorPosition { x: 1, ..ORIGIN },
            binding: None,
        });
        let mut stale_actor = use_on_self(10);
        stale_actor.target = Target::Creature(oteryn_session::EntityRef {
            generation: 2,
            ..OWN
        });
        let mut stale_map = use_on_self(10);
        stale_map.selection = Some(Selection {
            position: ORIGIN,
            binding: Some(([1; 32], [2; 32], 1)),
        });
        for command in [moved, stale_actor, stale_map] {
            let (mut session, mut peer) = joined().await?;
            assert!(world_interaction::item_current(&session, handle(10)));
            assert_eq!(
                world_interaction::send(&mut session, &command).await?,
                "Target changed"
            );
            drop(session);
            let mut sent = Vec::new();
            peer.read_to_end(&mut sent).await?;
            assert!(sent.is_empty());
        }
        Ok(())
    })
}

#[test]
fn equipped_use_with_source_is_current_and_reaches_the_session_command() -> Result<(), BoxError> {
    run(|| accepted_with_equipment(use_on_self(20), Some(20)))
}

#[test]
fn queued_equipment_changes_reject_removed_and_replaced_use_with_sources() -> Result<(), BoxError> {
    run(|| async {
        for replacement in [None, Some(21)] {
            let (mut session, mut peer) = support::joined_with_equipment(Some(20)).await?;
            assert!(world_interaction::item_current(&session, handle(20)));
            let payload = support::inventory(replacement)?;
            let delta = message(&[
                scalar(1, 9),
                scalar(2, 1),
                scalar(3, 2),
                scalar(4, 1),
                field(5, &payload),
            ]);
            write_frame(&mut peer, &envelope(9, 1, 41, &delta)).await?;
            assert_eq!(
                world_interaction::send(&mut session, &use_on_self(20)).await?,
                "Target changed"
            );
            assert!(!world_interaction::item_current(&session, handle(20)));
            assert_eq!(
                world_interaction::item_current(&session, handle(21)),
                replacement.is_some()
            );
            assert!(world_interaction::item_current(&session, handle(10)));
            drop(session);
            let mut sent = Vec::new();
            peer.read_to_end(&mut sent).await?;
            assert!(sent.is_empty(), "stale equipped command was written");
        }
        Ok(())
    })
}
