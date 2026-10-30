//! PKG5 I17: server-selected equipment intent through the existing fenced owner.
use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::equipment_move::{self, read_current_equipment};
use super::item_move::{EquipmentMove, EquipmentMoveReplay, EquipmentMoveResult};
use super::item_ref_admission::pinned_index;
use super::item_view::ItemKey;
use crate::durability::equipment_policy_abi::EquipmentPolicyLookup;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::hotkey_equip::Request;
use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome, wire_slot};

macro_rules! authority_methods {
    () => {
        fn hotkey_equip(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _request: oteryn_protocol_oteryn::hotkey_equip::Request,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::item_view::ItemMoveOutcome>> {
            async { Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Rejected) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn hotkey_equip(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            request: oteryn_protocol_oteryn::hotkey_equip::Request,
        ) -> Option<oteryn_protocol_oteryn::item_view::ItemMoveOutcome> {
            $crate::gameplay_transport::hotkey_equip::execute(self, actor, command, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: Request,
) -> Option<ItemMoveOutcome> {
    // Foundation binds command id, type and payload before this route. Replay must precede
    // lookup: the selected item has already left its original location after a commit.
    match equipment_move::committed_equipment_move(owner, command).await? {
        EquipmentMoveReplay::Moved => return Some(ItemMoveOutcome::Moved),
        EquipmentMoveReplay::Conflicting => return Some(ItemMoveOutcome::Rejected),
        EquipmentMoveReplay::Absent => {}
    }
    let Some(fence) = command.item_fence else {
        return Some(ItemMoveOutcome::Rejected);
    };
    let (definition, slot) = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let native = equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        )?;
        runtime
            .player_control_facts(actor, command.game_session_id)
            .ok()?;
        if !condition_allows_equip(
            &runtime,
            actor,
            command.game_session_id,
            owner.owner_now().get(),
        ) {
            return Some(ItemMoveOutcome::Rejected);
        }
        let index = pinned_index(&runtime, owner.active_generation)?;
        let Some((key, revision)) = index.definition_at(request.definition) else {
            return Some(ItemMoveOutcome::Stale);
        };
        let Some(policy) = native.equipment_policy(key, revision) else {
            return Some(ItemMoveOutcome::SlotMismatch);
        };
        let Some((slot, _)) = policy
            .claims()
            .iter()
            .find(|(slot, claim)| *slot != 10 && claim.is_ok())
        else {
            return Some(ItemMoveOutcome::SlotMismatch);
        };
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        };
        (definition, *slot)
    };
    let Some(slot_wire) = wire_slot(slot) else {
        return Some(ItemMoveOutcome::SlotMismatch);
    };
    if !action_ready(owner, actor, command, slot).await? {
        return Some(ItemMoveOutcome::Rejected);
    }
    // A queued command resolves custody after its wait, like the source's rescheduled call.
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .ok()?;
    let equipment = read_current_equipment(owner, command.game_session_id).await?;
    // Owner D1: Canary 52550d5e game.cpp:4210 toggles a worn match even with a spare.
    let worn = equipment
        .items
        .iter()
        .find(|item| item.slot == slot && item.definition == definition)
        .map(|item| ItemKey::Instance(item.item_instance_id));
    let direct = inventory.backpack.as_ref().and_then(|bag| {
        bag.entries
            .iter()
            .find(|entry| entry.item.definition == definition)
    });
    let request = if let Some(request) = select_move(
        worn,
        direct.map(|entry| ItemKey::Instance(entry.item.item_instance_id)),
        slot_wire,
    ) {
        request
    } else if inventory
        .nested
        .iter()
        .any(|(_, entry)| entry.item.definition == definition)
    {
        // The current equipment receipt does not admit a nested source. Never stage a
        // non-atomic intermediate move. A worn match has already taken precedence.
        return Some(ItemMoveOutcome::NotSupported);
    } else {
        return Some(if inventory.backpack.is_none() {
            ItemMoveOutcome::NoBackpack
        } else {
            ItemMoveOutcome::Stale
        });
    };
    match equipment_move::move_hotkey_equipment(owner, actor, command, request).await {
        EquipmentMoveResult::Committed => {
            owner.states.world_gate.lock().ok()?.equip_committed(
                actor,
                slot,
                owner.clock_origin.elapsed(),
            );
            Some(ItemMoveOutcome::Moved)
        }
        EquipmentMoveResult::Refused(refusal) => Some(super::item_move::equipment_outcome(refusal)),
        EquipmentMoveResult::Unknown => None,
    }
}

/// D1 applies before carried-copy selection; the equipment owner still validates custody.
fn select_move(
    worn: Option<ItemKey>,
    carried: Option<ItemKey>,
    slot: EquipmentSlot,
) -> Option<EquipmentMove> {
    if let Some(item) = worn {
        Some(EquipmentMove::Unequip { item, from: slot })
    } else {
        carried.map(|item| EquipmentMove::Equip { item, to: slot })
    }
}

pub(super) fn condition_allows_equip(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: ExactActorRef,
    session: crate::foundation::GameSessionId,
    now: u64,
) -> bool {
    runtime
        .actor_conditions(actor, Some(session))
        .is_ok_and(|store| {
            store.accepts_time(now) && !store.has_status(crate::foundation::StatusKind::Feared, now)
        })
}

/// Canary 52550d5e game.cpp:4171-4190 refuses jewelry during its own clock and
/// schedules an ordinary slot after the shared action delay. No owner lock spans the wait.
async fn action_ready(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    slot: u8,
) -> Option<bool> {
    loop {
        let delay = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            if states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
                || !condition_allows_equip(
                    &runtime,
                    actor,
                    command.game_session_id,
                    owner.owner_now().get(),
                )
            {
                return Some(false);
            }
            owner.states.world_gate.lock().ok()?.equip_delay(
                actor,
                slot,
                owner.clock_origin.elapsed(),
            )
        };
        if !wait_action(slot, delay).await {
            return Some(false);
        }
        if delay.is_none() {
            return Some(true);
        }
    }
}

async fn wait_action(slot: u8, delay: Option<std::time::Duration>) -> bool {
    if let Some(delay) = delay {
        if matches!(slot, 7 | 8) {
            return false;
        }
        tokio::time::sleep(delay).await;
    }
    true
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn ordinary_exhaust_waits_but_jewelry_refuses_without_waiting()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(async {
                tokio::time::pause();
                let start = tokio::time::Instant::now();
                for slot in [7, 8] {
                    assert!(!wait_action(slot, Some(Duration::from_millis(200))).await);
                }
                assert_eq!(tokio::time::Instant::now(), start);
                assert!(wait_action(5, None).await);
                let mut queued = Box::pin(wait_action(5, Some(Duration::from_millis(200))));
                assert!(
                    tokio::time::timeout(Duration::from_millis(199), &mut queued)
                        .await
                        .is_err()
                );
                tokio::time::advance(Duration::from_millis(1)).await;
                assert!(queued.await);
                // Tokio's timer wheel may round the wake up by one tick. The rule is a
                // minimum delay; the exact ready boundary is covered by the gate test.
                assert!(tokio::time::Instant::now() - start >= Duration::from_millis(200));
            });
        Ok(())
    }

    #[test]
    fn equip_timers_share_ordinary_slots_and_isolate_ring_necklace_and_actors()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::time::Duration;
        let (mut runtime, actor, _) = super::super::actor_spell::tests::runtime_with_player(0x95);
        let (_, _, other_session) = super::super::actor_spell::tests::runtime_with_player(0x96);
        let reservation = runtime
            .reserve_fresh_session(other_session)
            .map_err(|e| format!("{e:?}"))?;
        let other = runtime
            .commit_fresh_session(reservation)
            .map_err(|e| format!("{e:?}"))?;
        let mut gate = super::super::map_door::WorldActionGate::default();
        let at = Duration::from_millis(1000);
        for slot in 1..=10 {
            assert_eq!(gate.equip_delay(actor, slot, at), None);
        }
        gate.equip_committed(actor, 5, at);
        for slot in [1, 2, 3, 4, 5, 6, 10] {
            assert_eq!(
                gate.equip_delay(actor, slot, at),
                Some(Duration::from_millis(200))
            );
            assert_eq!(
                gate.equip_delay(actor, slot, at + Duration::from_millis(199)),
                Some(Duration::from_millis(1))
            );
            assert_eq!(
                gate.equip_delay(actor, slot, at + Duration::from_millis(200)),
                None
            );
        }
        assert_eq!(gate.equip_delay(actor, 7, at), None);
        assert_eq!(gate.equip_delay(actor, 8, at), None);
        assert_eq!(gate.equip_delay(other, 5, at), None);
        gate.equip_committed(actor, 7, at + Duration::from_millis(50));
        assert_eq!(
            gate.equip_delay(actor, 7, at + Duration::from_millis(200)),
            Some(Duration::from_millis(50))
        );
        assert_eq!(
            gate.equip_delay(actor, 8, at + Duration::from_millis(200)),
            None
        );
        gate.equip_committed(actor, 8, at + Duration::from_millis(100));
        assert_eq!(
            gate.equip_delay(actor, 8, at + Duration::from_millis(200)),
            Some(Duration::from_millis(100))
        );
        assert_eq!(
            gate.equip_delay(actor, 7, at + Duration::from_millis(250)),
            None
        );
        assert_eq!(
            gate.equip_delay(actor, 5, at + Duration::from_millis(250)),
            None
        );
        assert_eq!(
            gate.equip_delay(actor, 8, at + Duration::from_millis(300)),
            None
        );
        // World-object use and ordinary equip share the existing action clock in both directions.
        assert!(gate.admit(actor, at + Duration::from_millis(300)));
        assert_eq!(
            gate.equip_delay(actor, 5, at + Duration::from_millis(300)),
            Some(Duration::from_millis(200))
        );
        assert_eq!(
            gate.equip_delay(actor, 7, at + Duration::from_millis(300)),
            None
        );
        gate.equip_committed(actor, 5, at + Duration::from_millis(350));
        assert!(!gate.admit(actor, at + Duration::from_millis(549)));
        assert!(gate.admit(actor, at + Duration::from_millis(550)));
        Ok(())
    }

    #[test]
    fn feared_refuses_equipment_until_expiry_but_rooted_does_not()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::foundation::StatusKind;
        use oteryn_simulation_determinism::SemanticTimeMicros;
        for kind in [StatusKind::Rooted, StatusKind::Feared] {
            let (mut runtime, actor, session) =
                super::super::actor_spell::tests::runtime_with_player(0x95);
            assert!(condition_allows_equip(&runtime, actor, session, 0));
            apply_status(
                &mut runtime,
                actor,
                session,
                kind,
                SemanticTimeMicros::from_micros(100),
            )?;
            assert!(!condition_allows_equip(&runtime, actor, session, 99));
            assert_eq!(
                condition_allows_equip(&runtime, actor, session, 100),
                kind != StatusKind::Feared
            );
            assert_eq!(
                condition_allows_equip(&runtime, actor, session, 60_000_099),
                kind != StatusKind::Feared
            );
            assert!(condition_allows_equip(&runtime, actor, session, 60_000_100));
            let (_, stale_actor, stale_session) =
                super::super::actor_spell::tests::runtime_with_player(0x96);
            assert!(!condition_allows_equip(
                &runtime,
                stale_actor,
                stale_session,
                60_000_100
            ));
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) fn apply_status(
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        status: crate::foundation::StatusKind,
        now: oteryn_simulation_determinism::SemanticTimeMicros,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::foundation::{
            ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
            ConditionSourceKind, ConditionValues,
        };
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let definition = ConditionDefinition::new(
            "test.hotkey.status",
            1,
            ConditionValues::TimedStatus {
                kind: status,
                duration_ms: 60_000,
            },
        )
        .ok_or("status definition")?;
        let root = GameplayDecisionRoot::from_bytes([81; 32]);
        let plan = runtime
            .prepare_actor_condition(
                actor,
                Some(session),
                ActorConditionTransition::Apply {
                    definition: &definition,
                    source: ConditionSource {
                        actor,
                        session: Some(session),
                        kind: ConditionSourceKind::Player,
                    },
                    immunities: &[],
                    facts: ApplicationFacts {
                        now: now.get(),
                        base_speed: 110,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([81; 16]),
                    },
                },
                now,
            )
            .map_err(|e| format!("{e:?}"))?;
        if !runtime
            .commit_actor_condition(&plan, now)
            .map_err(|e| format!("{e:?}"))?
        {
            return Err("status not committed".into());
        }
        Ok(())
    }

    #[test]
    fn worn_match_always_unequips_including_when_a_spare_is_carried() {
        let worn = ItemKey::Instance([1; 16]);
        for spare in [None, Some(ItemKey::Instance([2; 16]))] {
            assert_eq!(
                select_move(Some(worn), spare, EquipmentSlot::RightHand),
                Some(EquipmentMove::Unequip {
                    item: worn,
                    from: EquipmentSlot::RightHand,
                })
            );
        }
    }

    #[test]
    fn carried_copy_equips_only_without_a_worn_match() {
        let spare = ItemKey::Instance([2; 16]);
        assert_eq!(
            select_move(None, Some(spare), EquipmentSlot::RightHand),
            Some(EquipmentMove::Equip {
                item: spare,
                to: EquipmentSlot::RightHand,
            })
        );
        assert_eq!(select_move(None, None, EquipmentSlot::RightHand), None);
    }
}
