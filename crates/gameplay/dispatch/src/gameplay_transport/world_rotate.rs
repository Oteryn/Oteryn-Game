//! W18: Rotate is independent of USE and retains the existing opaque target model.
use super::capabilities::OfferedCapability;
use super::connection::{UseCommand, UseOutcome};
use super::item_view::ItemKey;
use super::{ComposedFreshAdmission, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result: Vec<_> = base.iter().copied().filter(|c| c.id != 388).collect();
    if available {
        let at = result.partition_point(|c| c.id < 388);
        result.insert(
            at,
            OfferedCapability {
                id: 388,
                requires: &[11],
            },
        );
    }
    result
}

macro_rules! authority_methods {
    () => {
        fn world_rotate_available(&self) -> bool {
            false
        }
        fn can_rotate_item(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _source: Option<$crate::gameplay_transport::item_view::ItemKey>,
            _observed: Option<std::num::NonZeroU32>,
        ) -> impl std::future::Future<Output = Result<bool, ()>> {
            async { Ok(false) }
        }
        fn rotate_world_item(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _intent: oteryn_protocol_oteryn::world_object::UseIntent,
            _source: Option<$crate::gameplay_transport::item_view::ItemKey>,
            _observed: Option<std::num::NonZeroU32>,
        ) -> impl std::future::Future<
            Output = Result<Option<$crate::gameplay_transport::connection::UseOutcome>, ()>,
        > {
            async { Ok(None) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn world_rotate_available(&self) -> bool {
            self.bundle_map().is_some()
                && self
                    .active_generation
                    .and_then(|g| g.native_gameplay())
                    .is_some()
        }
        async fn can_rotate_item(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            source: Option<$crate::gameplay_transport::item_view::ItemKey>,
            observed: Option<std::num::NonZeroU32>,
        ) -> Result<bool, ()> {
            self.can_rotate_carried_item_impl(actor, command, source, observed)
                .await
        }
        async fn rotate_world_item(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            intent: oteryn_protocol_oteryn::world_object::UseIntent,
            source: Option<$crate::gameplay_transport::item_view::ItemKey>,
            observed: Option<std::num::NonZeroU32>,
        ) -> Result<Option<$crate::gameplay_transport::connection::UseOutcome>, ()> {
            self.rotate_world_item_impl(actor, command, intent, source, observed)
                .await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Advisory carried-item affordance. The subsequent Rotate independently
    /// rechecks observed definition, custody, fences and the source rule.
    pub(super) async fn can_rotate_carried_item_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: Option<ItemKey>,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<bool, ()> {
        use oteryn_gameplay_content::content::native_gameplay::{
            NativeItemDestination, NativeItemStackClass,
        };
        let (Some(ItemKey::Instance(instance)), Some(observed), Some(fence)) =
            (source, observed, command.item_fence)
        else {
            return Ok(false);
        };
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return Ok(false);
        };
        let Some(expected) = native
            .item_index()
            .and_then(|index| index.definition_at(observed))
        else {
            return Ok(false);
        };
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Ok(false);
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let found = inventory
            .equipment
            .iter()
            .chain(
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| &bag.entries)
                    .map(|entry| &entry.item),
            )
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == instance);
        let Some(item) = found else { return Ok(false) };
        if item.quantity != 1
            || expected
                != (
                    item.definition.production_key.as_str(),
                    item.definition.revision_ref.as_str(),
                )
        {
            return Ok(false);
        }
        let Some(plan) = oteryn_gameplay_items::furniture::plan_rotate(native, &item.definition)
        else {
            return Ok(false);
        };
        let [(after, 1)] = plan.outputs.as_slice() else {
            return Ok(false);
        };
        Ok([&item.definition, after].iter().all(|definition| {
            native
                .item_policy(&definition.production_key, &definition.revision_ref)
                .is_some_and(|policy| {
                    let record = policy.record();
                    record.admission.materializable
                        && record.admission.stack_class == NativeItemStackClass::NonStackable
                        && record
                            .admission
                            .legal_destinations
                            .contains(&NativeItemDestination::CharacterInventory)
                })
        }))
    }

    pub(super) async fn rotate_world_item_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<Option<UseOutcome>, ()> {
        if command.item_fence.is_some_and(|fence| {
            matches!(
                fence.runtime_scope,
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            )
        }) {
            return self
                .rotate_house_event_item(actor, command, &intent, source, observed)
                .await;
        }
        if matches!(intent.target, UseTarget::Item(_)) {
            if let Some(result) = self
                .rotate_ground_world_item(actor, command, &intent, source, observed)
                .await?
            {
                return Ok(Some(result));
            }
            return self
                .rotate_carried_world_item(actor, command, &intent, source, observed)
                .await;
        }
        let UseTarget::WorldObject(target) = &intent.target else {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            }));
        };
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::Exhausted,
                ..UseOutcome::rejected()
            }));
        }
        let position = runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position();
        let at = crate::map::overlay::TilePos {
            x: position.x.try_into().map_err(|_| ())?,
            y: position.y.try_into().map_err(|_| ())?,
            floor: i8::try_from(position.floor)
                .map_err(|_| ())?
                .checked_neg()
                .ok_or(())?,
        };
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return Ok(None);
        };
        if super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none() {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::StaleState,
                ..UseOutcome::rejected()
            }));
        }
        Ok(Some(UseOutcome {
            disposition: super::item_transform::rotate_explicit(
                map,
                native,
                runtime.content_pin().client_artifact_digest(),
                at,
                target,
            )
            .unwrap_or(UseDisposition::NothingToUse),
            ..UseOutcome::rejected()
        }))
    }
    async fn rotate_carried_world_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<Option<UseOutcome>, ()> {
        use super::item_ref_admission::NativeItemWeights;
        use crate::durability::consume_transform::ConsumeTransformRequest;
        use crate::foundation::{CommandId, CommandRef};
        let refuse = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(fence) = command.item_fence else {
            return Ok(refuse(UseDisposition::StaleState));
        };
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return Ok(None);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(None);
        };
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: match source {
                Some(ItemKey::Instance(id)) => id,
                _ => [0; 16],
            },
            target: None,
            intent: bound_intent(intent, observed).map_err(|_| ())?,
        };
        // Immutable replay precedes the current handle, actor or mutable source checks.
        if let Some(previous) = self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await
            .map_err(|_| ())?
        {
            return Ok(refuse(previous.disposition));
        }
        if !matches!(source, Some(ItemKey::Instance(_))) {
            return Ok(refuse(UseDisposition::StaleState));
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Ok(refuse(UseDisposition::Rejected));
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return Ok(refuse(UseDisposition::Exhausted));
            }
        }
        Ok(self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &content,
                false,
            )
            .await
            .map_err(|_| ())?
            .map(|result| UseOutcome {
                disposition: result.disposition,
                ..UseOutcome::rejected()
            }))
    }
}

/// Commit the observed type as part of the semantic command, without publishing instance ids.
pub(super) fn bound_intent(
    intent: &UseIntent,
    observed: Option<std::num::NonZeroU32>,
) -> Result<Vec<u8>, oteryn_protocol_oteryn::world_object::WorldObjectError> {
    let reference =
        observed.ok_or(oteryn_protocol_oteryn::world_object::WorldObjectError::Malformed)?;
    let body = oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)?;
    let mut bytes = Vec::with_capacity(body.len() + 5);
    bytes.push(0xff);
    bytes.extend_from_slice(&reference.get().to_be_bytes());
    bytes.extend(body);
    Ok(bytes)
}
