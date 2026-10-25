//! Live depot composition. No town or Character authority comes from a client.
use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::equipment_move::native_for_move;
use super::item_ref_admission::{NativeItemWeights, definition_ref};
use super::item_view::ItemKey;
use crate::durability::item_loop_move::{DepotMove, LoopMoveRefusal, LoopMoveReplay};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::depot::{
    DepotAction, DepotEntry, DepotIntent, DepotOutcome, DepotReply,
};
use sha2::{Digest, Sha256};

fn empty(intent: DepotIntent, outcome: DepotOutcome) -> DepotReply {
    DepotReply {
        outcome,
        town: None,
        box_number: intent.box_number,
        page: intent.page,
        total: 0,
        entries: Vec::new(),
    }
}

/// The nearest visible authored locker, with deterministic position/ordinal tie-breaking.
fn locker(at: &super::bundle_map_position::BundleMapPosition<'_>) -> Option<u16> {
    let mut found = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let pos = TilePos {
                x: match at.at.x.checked_add(dx).and_then(|x| u16::try_from(x).ok()) {
                    Some(x) => x,
                    None => continue,
                },
                y: match at.at.y.checked_add(dy).and_then(|y| u16::try_from(y).ok()) {
                    Some(y) => y,
                    None => continue,
                },
                floor: i8::try_from(at.at.floor).ok()?,
            };
            let Some(tile) = at.map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
                continue;
            };
            let overlay = at.map.overlay.tile(pos);
            for (ordinal, _) in tile
                .depths()
                .iter()
                .filter(|depth| **depth == 0)
                .enumerate()
            {
                let Ok(ordinal) = u8::try_from(ordinal) else {
                    continue;
                };
                if overlay
                    .as_ref()
                    .is_some_and(|overlay| overlay.is_hidden(ordinal))
                {
                    continue;
                }
                if let Some(town) = at.map.facts.depot_at(pos, ordinal) {
                    found.push((dx.abs().max(dy.abs()), pos.x, pos.y, ordinal, town));
                }
            }
        }
    }
    found.sort_unstable();
    found.first().map(|entry| entry.4)
}

pub(super) async fn execute(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: DepotIntent,
    selected_item: Option<ItemKey>,
) -> Option<DepotReply> {
    let Some(fence) = command.item_fence else {
        return Some(empty(intent, DepotOutcome::Rejected));
    };
    let current = super::FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(command.game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != super::GameSessionState::Active
        || super::item_fence_of(current) != Some(fence)
    {
        return Some(empty(intent, DepotOutcome::Rejected));
    }
    let command_ref = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    let access = DepotMove {
        town: intent.town.unwrap_or(0),
        premium: crate::domain::premium::PremiumGate::CLOSED,
        box_number: intent.box_number,
        deposit: matches!(intent.action, DepotAction::Deposit(_)),
        request_digest: Sha256::digest(
            oteryn_protocol_oteryn::depot::encode_depot_intent(intent).ok()?,
        )
        .into(),
    };
    // Replay before resolving ephemeral source handles/ordinals. No second effect after a lost reply.
    let replay = if intent.action == DepotAction::Browse {
        LoopMoveReplay::Absent
    } else {
        admission
            .root
            .read_depot_move_replay(admission.character, fence.character_id, command_ref, access)
            .await
            .ok()?
    };
    if replay == LoopMoveReplay::Conflicting {
        return Some(empty(intent, DepotOutcome::Rejected));
    }
    let refused = |outcome| {
        empty(
            intent,
            if replay == LoopMoveReplay::Moved {
                DepotOutcome::Moved
            } else {
                outcome
            },
        )
    };
    let permit = admission.spell_lane_permit().await?;
    if intent.action != DepotAction::Browse
        && !admission
            .states
            .player_trades
            .physical_service(command.game_session_id)
    {
        return Some(refused(DepotOutcome::Rejected));
    }
    let (native, town) = {
        let mut runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        let native = native_for_move(
            &runtime,
            admission.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        );
        let town = super::bundle_map_position(&mut runtime, admission.movement_cells, actor)
            .as_ref()
            .and_then(locker);
        (native, town)
    };
    let Some(native) = native else {
        return Some(refused(DepotOutcome::Rejected));
    };
    let Some(town) = town else {
        return Some(refused(DepotOutcome::NoLocker));
    };
    if intent.town.is_some_and(|expected| expected != town) {
        return Some(refused(DepotOutcome::Stale));
    }
    let mut outcome = match replay {
        LoopMoveReplay::Absent => DepotOutcome::Opened,
        LoopMoveReplay::Moved => DepotOutcome::Moved,
        LoopMoveReplay::Conflicting => DepotOutcome::Rejected,
    };
    let mut page = admission
        .root
        .read_depot_page(
            admission.character,
            fence.character_id,
            town,
            intent.box_number,
            intent.page,
        )
        .await
        .ok()?;
    if replay == LoopMoveReplay::Absent && intent.action != DepotAction::Browse {
        let item = match intent.action {
            DepotAction::Deposit(_) => match selected_item {
                Some(ItemKey::Instance(item)) => Some(item),
                _ => None,
            },
            DepotAction::Withdraw(ordinal) => page
                .entries
                .iter()
                .find(|entry| entry.ordinal == ordinal)
                .map(|entry| entry.item.item_instance_id),
            DepotAction::Browse => None,
        };
        outcome = if let Some(item) = item {
            // Read the existing native gate at this decision, never a client tier or stored grant.
            let access = DepotMove {
                premium: admission
                    .session_premium_gate(command.game_session_id)
                    .await,
                ..access
            };
            let committed = admission
                .root
                .commit_depot_move(
                    &permit,
                    admission.character,
                    admission.holder,
                    &fence,
                    command_ref,
                    &NativeItemWeights(native),
                    access,
                    item,
                )
                .await
                .ok()?;
            match committed {
                Ok(_) => DepotOutcome::Moved,
                Err(LoopMoveRefusal::Stale) => DepotOutcome::Stale,
                Err(LoopMoveRefusal::NoRoom) => DepotOutcome::Full,
                Err(LoopMoveRefusal::NoBackpack) => DepotOutcome::NoBackpack,
                Err(LoopMoveRefusal::Overweight) => DepotOutcome::Overweight,
                Err(LoopMoveRefusal::NotSupported | LoopMoveRefusal::WeightUnknown) => {
                    DepotOutcome::Unsupported
                }
                Err(LoopMoveRefusal::Rejected | LoopMoveRefusal::Blocked) => DepotOutcome::Rejected,
            }
        } else {
            DepotOutcome::Stale
        };
        if outcome == DepotOutcome::Moved {
            page = admission
                .root
                .read_depot_page(
                    admission.character,
                    fence.character_id,
                    town,
                    intent.box_number,
                    intent.page,
                )
                .await
                .ok()?;
            admission
                .states
                .attack
                .lock()
                .await
                .forget_weapon(actor, command.game_session_id);
        }
    }
    let index = native.item_index()?;
    let entries = page
        .entries
        .into_iter()
        .map(|entry| {
            Some(DepotEntry {
                ordinal: entry.ordinal,
                definition: definition_ref(index, &entry.item.definition)?.get(),
                count: entry.item.quantity,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(DepotReply {
        outcome,
        town: Some(town),
        box_number: intent.box_number,
        page: intent.page,
        total: page.total,
        entries,
    })
}
