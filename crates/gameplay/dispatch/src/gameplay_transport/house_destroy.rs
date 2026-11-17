//! Selective USE_WITH probe. Observations never grant mutation authority.
use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::house_furnishing::{destroy_remains, destroy_roll_succeeds, is_destroy_weapon};
use super::item_view::{ItemKey, SessionItemView};
use crate::content::ReferenceItemField;
use crate::durability::house_furniture_destroy::{HouseDestroyCommand, HouseDestroyOutcome};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct Candidate {
    pub command: HouseDestroyCommand,
    pub decision_root: [u8; 32],
}
fn item_id(definition: &TypedDefinitionRef) -> Option<u32> {
    (definition.family == "Item").then_some(())?;
    definition
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()
}
pub(super) async fn dispatch_map_use<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    view: Option<&SessionItemView>,
) -> Option<UseOutcome> {
    let UseTarget::Item(source_handle) = intent.target else {
        return None;
    };
    let Some(MapUseTarget::Item(target_handle)) = intent.use_with_map_item else {
        return None;
    };
    let ItemKey::Instance(source) = view?.resolve(source_handle)? else {
        return None;
    };
    let ItemKey::Instance(target) = view?.resolve(target_handle)? else {
        return None;
    };
    let mut candidate = authority
        .observe_house_destroy(actor, command, source, target)
        .await?;
    let input = &mut candidate.command;
    if input.source != source
        || input.target != target
        || input.command != command.command_id
        || input.game_session_id != command.game_session_id
        || !is_destroy_weapon(item_id(&input.source_definition)?)
        || !(0..=65535).contains(&input.attack)
    {
        return None;
    }
    let remains = destroy_remains(item_id(&input.target_definition)?)?;
    if item_id(&input.remains)? != remains.item_id
        || item_id(&input.decay_target)? != remains.decay_target
        || input.duration_ms != remains.duration_ms
    {
        return None;
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn.house.destroy.v1")
        .chain_update(command.game_session_id.as_bytes())
        .chain_update(command.command_id.to_be_bytes())
        .chain_update(target)
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    // Rejection sampling removes the modulo bias of an eighty-way u64 draw.
    let root = GameplayDecisionRoot::from_bytes(candidate.decision_root);
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let mut draw_index = 0;
    input.roll = loop {
        let draw = deterministic_decision_u64(
            &root,
            occurrence,
            "house.furniture.destroy.roll",
            draw_index,
        )
        .ok()?;
        if draw < u64::MAX - u64::MAX % 80 {
            break u8::try_from(draw % 80 + 1).ok()?;
        }
        draw_index = draw_index.checked_add(1)?;
    };
    // The durable owner rechecks the inclusive rule; this also exercises the shared parity rule.
    input.expected_success = destroy_roll_succeeds(input.roll, input.attack);
    authority
        .destroy_house_furniture(actor, command, candidate.command)
        .await
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_house_destroy_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: [u8; 16],
        target: [u8; 16],
    ) -> Option<Candidate> {
        let fence = command.item_fence?;
        let house = {
            let runtime = self.runtime.lock().await;
            let state = runtime
                .house_actor_state(actor, command.game_session_id)
                .ok()?;
            if state.scope != fence.runtime_scope
                || state.generation != fence.scope_ownership_generation
                || state.lease.generation() != fence.character_lease_generation
                || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            {
                return None;
            }
            HouseId::new(fence.runtime_scope.world_id(), &state.house)?
        };
        let observation = self
            .root
            .observe_house_destroy(
                house.clone(),
                source,
                target,
                command.game_session_id,
                command.command_id,
            )
            .await
            .ok()??;
        let native = self.active_generation?.native_gameplay()?;
        let index = native.item_index()?;
        let exact = |definition: &TypedDefinitionRef| {
            index.definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
        };
        exact(&observation.source)?;
        exact(&observation.target)?;
        if !is_destroy_weapon(item_id(&observation.source)?) {
            return None;
        }
        let remains = destroy_remains(item_id(&observation.target)?)?;
        let policy = native.item_policy(
            &observation.source.production_key,
            &observation.source.revision_ref,
        )?;
        let ReferenceItemField::Known(weapon) = &policy.record().semantics.weapon else {
            return None;
        };
        let ReferenceItemField::Known(attack) = weapon.attack else {
            return None;
        };
        if !(0..=65535).contains(&attack.0) {
            return None;
        }
        let definition = |id| {
            let key = format!("oteryn:item.tibia.i{id}");
            Some(TypedDefinitionRef {
                family: "Item".into(),
                revision_ref: index.revision(&key)?.into(),
                production_key: key,
            })
        };
        Some(Candidate {
            decision_root: native.source_digest(),
            command: HouseDestroyCommand {
                house,
                command: command.command_id,
                game_session_id: command.game_session_id,
                source,
                source_definition: observation.source,
                target,
                target_definition: observation.target,
                expected_revision: observation.revision,
                expected_acl_revision: observation.acl_revision,
                roll: 1,
                attack: attack.0,
                expected_success: false,
                remains: definition(remains.item_id)?,
                decay_target: definition(remains.decay_target)?,
                duration_ms: remains.duration_ms,
            },
        })
    }
    pub(in crate::gameplay_transport) async fn destroy_house_furniture_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        input: HouseDestroyCommand,
    ) -> Option<UseOutcome> {
        let fence = command.item_fence?;
        let _permit = self.spell_lane_permit().await?;
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Some(UseOutcome::rejected());
            }
        }
        let result = self
            .root
            .destroy_house_furniture(self.holder, fence, input)
            .await
            .ok()?;
        Some(UseOutcome {
            disposition: match result {
                HouseDestroyOutcome::Committed | HouseDestroyOutcome::FailedRoll => {
                    UseDisposition::Committed
                }
                HouseDestroyOutcome::NoAccess => UseDisposition::Rejected,
                HouseDestroyOutcome::Stale => UseDisposition::StaleState,
            },
            ..UseOutcome::rejected()
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn expire_house_furniture_on_tick(
        &self,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) -> Option<()> {
        let house = {
            let runtime = self.runtime.lock().await;
            let state = runtime.house_actor_state(actor, session).ok()?;
            HouseId::new(state.scope.world_id(), &state.house)?
        };
        let fence = self.current_quest_fence(session).await.ok()??.item_fence();
        let _permit = self.spell_lane_permit().await?;
        self.root
            .expire_house_furniture(self.holder, fence, house)
            .await
            .ok()?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::connection::{AdmissionRefusal, AdmittedSession, FreshAdmissionAttempt};
    use super::*;
    use crate::foundation::{ChannelId, GameSessionId, WorldId};
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use std::cell::Cell;
    type TestResult = Result<(), Box<dyn std::error::Error>>;
    struct Authority {
        candidate: Candidate,
        calls: Cell<u32>,
        next_calls: Cell<u32>,
    }
    impl FreshAdmissionAuthority for Authority {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }
        async fn observe_house_destroy(
            &self,
            _: ExactActorRef,
            _: UseCommand,
            _: [u8; 16],
            _: [u8; 16],
        ) -> Option<Candidate> {
            Some(self.candidate.clone())
        }
        async fn destroy_house_furniture(
            &self,
            _: ExactActorRef,
            _: UseCommand,
            input: HouseDestroyCommand,
        ) -> Option<UseOutcome> {
            assert!((1..=80).contains(&input.roll));
            self.calls.set(self.calls.get() + 1);
            Some(UseOutcome {
                disposition: UseDisposition::Committed,
                ..UseOutcome::rejected()
            })
        }
        async fn use_pick(
            &self,
            _: ExactActorRef,
            _: UseCommand,
            _: UseIntent,
            _: Option<ItemKey>,
            _: Option<ItemKey>,
        ) -> Option<UseOutcome> {
            self.next_calls.set(self.next_calls.get() + 1);
            Some(UseOutcome {
                disposition: UseDisposition::Full,
                ..UseOutcome::rejected()
            })
        }
    }
    async fn probe(source_id: u32, target_id: u32, expected_calls: u32) -> TestResult {
        let mut bytes = [1; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        let world = WorldId::decode(&bytes).map_err(|e| format!("{e:?}"))?;
        let channel = ChannelId::decode(&bytes).map_err(|e| format!("{e:?}"))?;
        let session = GameSessionId::decode(&bytes).map_err(|e| format!("{e:?}"))?;
        let actor = ExactActorRef::transport_fixture(world, channel);
        let definition = |id| TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "test-active-revision".into(),
        };
        let authority = Authority {
            candidate: Candidate {
                decision_root: [1; 32],
                command: HouseDestroyCommand {
                    house: HouseId::new(world, "oteryn:content.house.destroy_test")
                        .ok_or("house")?,
                    command: 1,
                    game_session_id: session,
                    source: [2; 16],
                    source_definition: definition(source_id),
                    target: [3; 16],
                    target_definition: definition(target_id),
                    expected_revision: 1,
                    expected_acl_revision: 1,
                    roll: 1,
                    attack: 40,
                    expected_success: false,
                    remains: definition(4996),
                    decay_target: definition(4995),
                    duration_ms: 300000,
                },
            },
            calls: Cell::new(0),
            next_calls: Cell::new(0),
        };
        let mut view = SessionItemView::resume(Default::default()).with_map_view();
        let (source, target) = view
            .map_view(
                &[ItemKey::Instance([2; 16]), ItemKey::Instance([3; 16])],
                |table| {
                    Ok((
                        table.handle(&ItemKey::Instance([2; 16])),
                        table.handle(&ItemKey::Instance([3; 16])),
                    ))
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        let intent = UseIntent {
            target: UseTarget::Item(source.ok_or("source handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(target.ok_or("target handle")?)),
            use_with_position: None,
        };
        let command = UseCommand {
            game_session_id: session,
            command_id: 1,
            item_fence: None,
        };
        let result =
            dispatch_map_use(&authority, actor, command, intent.clone(), Some(&view)).await;
        assert_eq!(authority.calls.get(), expected_calls);
        if expected_calls == 0 {
            assert!(result.is_none());
            let next = super::super::mining::dispatch_map_use(
                &authority,
                actor,
                command,
                intent,
                Some(&view),
            )
            .await
            .ok_or("mining outcome")?;
            assert_eq!(next.disposition, UseDisposition::Full);
            assert_eq!(authority.next_calls.get(), 1);
        } else {
            assert_eq!(
                result.ok_or("destroy outcome")?.disposition,
                UseDisposition::Committed
            );
            assert_eq!(authority.next_calls.get(), 0);
        }
        // Unresolved handles do not call even the candidate reader.
        let unresolved = UseIntent {
            target: UseTarget::Item(ItemHandle::new(99).ok_or("handle")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Item(ItemHandle::new(100).ok_or("handle")?)),
            use_with_position: None,
        };
        assert!(
            dispatch_map_use(&authority, actor, command, unresolved, Some(&view))
                .await
                .is_none()
        );
        assert_eq!(authority.calls.get(), expected_calls);
        Ok(())
    }
    fn run_probe(source: u32, target: u32, calls: u32) -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                runtime
                    .block_on(probe(source, target, calls))
                    .map_err(|e| e.to_string())
            })?
            .join()
            .map_err(|_| "destroy dispatch thread failed")??;
        Ok(())
    }
    #[test]
    fn non_weapon_use_with_falls_through_to_next_handler() -> TestResult {
        run_probe(100, 4995, 0)
    }
    #[test]
    fn destroy_weapon_on_non_destroyable_target_falls_through_without_permit() -> TestResult {
        run_probe(3264, 100, 0)
    }
    #[test]
    fn qualified_destroy_calls_durable_owner_once() -> TestResult {
        run_probe(3264, 4995, 1)
    }
}
