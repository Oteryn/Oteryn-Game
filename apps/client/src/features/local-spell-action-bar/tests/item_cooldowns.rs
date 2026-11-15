use super::*;
use crate::play::{BarAction, ItemUseMode, SpellAim};
use oteryn_client_runtime::{
    action_bar_cooldowns as wire,
    world_interaction::{Selection, Target},
};
use oteryn_session::spell_catalog::*;
use std::num::{NonZeroU32, NonZeroU64};
type Fixture = (
    crate::play::PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
    crate::play::PlayView,
    oteryn_session::ActorPosition,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    let view = view()?;
    outbox.bar_item_cooldowns_initialize(true);
    outbox.targeting_initialize(true);
    outbox.spell_catalog_initialize(
        true,
        Some(
            encode_catalog(&[SpellCatalogEntry {
                cast_index: 71,
                name: "Fallback".into(),
                words: "fallback".into(),
                level: 1,
                mana: 1,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: true,
                cooldown_groups: vec![],
                rune_item: None,
            }])
            .map_err(|e| format!("{e:?}"))?,
        ),
    )?;
    let definition = NonZeroU32::new(256).ok_or("definition")?;
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                appearance_id: 0,
                handle: NonZeroU64::MIN,
                item_definition_ref: definition,
                count: NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    let original = oteryn_session::ActorPosition {
        x: 100,
        y: 101,
        floor: 7,
    };
    assert!(link.assign_bar_action(
        0,
        BarAction::Multi {
            actions: vec![
                BarAction::Use {
                    definition,
                    sub_type: None,
                    mode: ItemUseMode::CursorPosition
                },
                BarAction::Spell {
                    name: "Fallback".into(),
                    words: "fallback".into(),
                    aim: SpellAim::Target
                }
            ]
        }
    ));
    crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = Some((
        Target::Position(original),
        Selection {
            position: original,
            binding: None,
        },
    ));
    Ok((link, commands, outbox, view, original))
}
fn result(
    remaining_ms: u64,
) -> Result<oteryn_session::GatedCommandResult, Box<dyn std::error::Error>> {
    Ok(oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: wire::encode_reply(&[wire::ItemCooldown {
            definition: 256,
            remaining_ms,
        }])
        .ok_or("reply")?,
    })
}

#[test]
fn multiaction_reads_real_item_exhaustion_before_one_queued_action()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox, mut view, original) = fixture()?;
    for remaining_ms in [500, 0] {
        crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = Some((
            Target::Position(original),
            Selection {
                position: original,
                binding: None,
            },
        ));
        view.cast(NonZeroU32::MIN);
        view.tick(&link).map_err(|_| "tick query")?;
        let crate::combat_input::PlayCommand::Feature(query) = commands.try_recv()? else {
            return Err("expected bounded item cooldown query before an action".into());
        };
        assert_eq!((query.capability, query.kind), (463, 6147));
        assert_eq!(wire::decode_query(&query.payload), Some(vec![256]));
        assert!(commands.try_recv().is_err());
        crate::play::mailbox::lock(&link.mailbox).bar_actions.cursor = None;
        outbox.bar_item_cooldowns_result(
            outbox.bar_action_scope(),
            &oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: wire::encode_reply(&[wire::ItemCooldown {
                    definition: 256,
                    remaining_ms,
                }])
                .ok_or("reply")?,
            },
        );
        view.tick(&link).map_err(|_| "tick action")?;
        if remaining_ms > 0 {
            assert!(
                matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==71)
            );
        } else {
            assert!(
                matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.target==Target::Position(original))
            );
        }
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn pending_item_read_cannot_execute_an_edited_assignment() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "query tick")?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    assert!(link.assign_bar_action(
        0,
        BarAction::Text {
            text: "changed".into(),
            send_automatically: true
        }
    ));
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &result(0)?);
    view.tick(&link).map_err(|_| "edited tick")?;
    assert!(commands.try_recv().is_err());
    assert!(!link.bar_activation_pending());
    Ok(())
}
#[test]
fn rejected_item_read_releases_the_real_input_queue_without_firing_an_alternative()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    view.cast(NonZeroU32::MIN);
    view.cast(NonZeroU32::new(2).ok_or("slot")?);
    view.tick(&link).map_err(|_| "query tick")?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    assert!(commands.try_recv().is_err());
    outbox.bar_item_cooldowns_result(
        outbox.bar_action_scope(),
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Rejected,
            payload: vec![],
        },
    );
    view.tick(&link).map_err(|_| "refused tick")?;
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==2)
    );
    assert!(commands.try_recv().is_err());
    assert!(!link.bar_activation_pending());
    Ok(())
}
#[test]
fn item_query_reply_must_match_the_requested_definitions() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "query tick")?;
    let _ = commands.try_recv()?;
    let mut reply = result(0)?;
    reply.payload = wire::encode_reply(&[wire::ItemCooldown {
        definition: 257,
        remaining_ms: 0,
    }])
    .ok_or("mismatch")?;
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &reply);
    view.tick(&link).map_err(|_| "mismatch tick")?;
    assert!(commands.try_recv().is_err());
    assert!(!link.bar_activation_pending());
    Ok(())
}
#[test]
fn item_query_preserves_action_order_before_later_casts() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut commands, outbox, mut view, original) = fixture()?;
    view.cast(NonZeroU32::MIN);
    view.cast(NonZeroU32::new(2).ok_or("slot")?);
    view.tick(&link).map_err(|_| "query tick")?;
    let _ = commands.try_recv()?;
    assert!(commands.try_recv().is_err());
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &result(0)?);
    view.tick(&link).map_err(|_| "action tick")?;
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::WorldContext(command) if command.target==Target::Position(original))
    );
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get()==2)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn preview_item_read_never_casts_or_uses_an_item() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    view.tick(&link).map_err(|_| "preview tick")?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &result(0)?);
    view.tick(&link).map_err(|_| "preview reply")?;
    assert!(commands.try_recv().is_err());
    assert!(!link.bar_activation_pending());
    Ok(())
}
#[test]
fn unselected_item_cooldown_capability_sends_no_item_or_spell()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    outbox.bar_item_cooldowns_initialize(false);
    view.cast(NonZeroU32::MIN);
    view.tick(&link).map_err(|_| "unselected tick")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn activity_transition_cancels_pending_and_deferred_item_reads()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::activity_scope::*;
    let (link, mut commands, outbox, mut view, _) = fixture()?;
    view.tick(&link).map_err(|_| "tick")?;
    assert!(link.bar_item_read_pending());
    let action = crate::play::mailbox::lock(&link.mailbox)
        .bar_actions
        .saved
        .0[&0]
        .clone();
    assert!(link.assign_bar_action(1, action));
    let origin_scope = outbox.bar_action_scope();
    // A different activation waits behind the preview query.
    link.cast_spell_slot(NonZeroU32::new(2).ok_or("slot")?);
    assert!(link.bar_activation_pending());
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut successor = id;
    successor[15] = 2;
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id,
        source_session: id,
        session: successor,
        scope: ActivityScope::Instance {
            world: id,
            instance: id,
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 2,
        selection: vec![117, 463],
        requires_ack: false,
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![SessionEvent::GatedSnapshotReplaced(AppliedDelta {
        server_sequence: 3,
        base_revision: 0,
        new_revision: 1,
        value: oteryn_session::GatedSnapshot {
            domain_id: STATE_DOMAIN_ACTIVITY_SCOPE,
            snapshot_type: 1,
            revision: 1,
            payload,
        },
    })]);
    assert_ne!(outbox.bar_action_scope(), origin_scope);
    assert!(outbox.death_command(&mut commands).is_err());
    assert!(!link.bar_item_read_pending());
    assert!(!link.bar_activation_pending());
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &result(0)?);
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .bar_actions
            .item_cooldowns
            .is_empty()
    );
    view.tick(&link).map_err(|_| "scope tick")?;
    outbox.inventory_initialize(
        Some(oteryn_session::CharacterInventory {
            entries: vec![oteryn_session::ItemEntry {
                handle: NonZeroU64::MIN,
                item_definition_ref: NonZeroU32::new(256).ok_or("definition")?,
                count: NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        }),
        true,
    );
    link.pump_bar_item_cooldowns(true);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Feature(_)
    ));
    outbox.bar_item_cooldowns_result(origin_scope, &result(0)?);
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .bar_actions
            .item_cooldowns
            .is_empty()
    );
    assert!(
        !crate::play::mailbox::lock(&link.mailbox)
            .bar_actions
            .item_cooldown_read
            .as_ref()
            .ok_or("read")?
            .ready
    );
    outbox.bar_item_cooldowns_result(outbox.bar_action_scope(), &result(0)?);
    link.pump_bar_item_cooldowns(false);
    view.logout();
    view.tick(&link).map_err(|_| "tick")?;
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Logout
    ));
    Ok(())
}
