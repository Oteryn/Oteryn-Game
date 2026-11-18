//! Crystal 201189f8 movements_arena_trophy.lua.
use super::*;
use crate::durability::reward_claim_mint::ArenaTrophyStep;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_arena_trophy(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) {
        if let Err(error) = self
            .try_step_arena_trophy(permit, actor, session, entered)
            .await
        {
            super::super::operator_event(&format!("arena_trophy_step_failed reason={error:?}"));
        }
    }

    async fn try_step_arena_trophy(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Result<(), crate::durability::DurabilityError> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        let Some(at) = native(entered.position()) else {
            return Ok(());
        };
        let Some(tile) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return Ok(());
        };
        let mut matches = (0..tile.depths().iter().filter(|depth| **depth == 0).count())
            .filter_map(|ordinal| placement_key(at, u8::try_from(ordinal).ok()?))
            .filter(|key| !map.facts.base_hidden(*key))
            .flat_map(|key| {
                [(3264, 5807), (3265, 5806), (3266, 5805)]
                    .into_iter()
                    .filter(move |(uid, _)| {
                        map.facts
                            .quest_callback_matches(key, Some(*uid), None, None)
                    })
            });
        let Some((uid, source)) = matches.next() else {
            return Ok(());
        };
        if matches.next().is_some()
            || !at
                .y
                .checked_sub(1)
                .is_some_and(|north| map.overlay.base().tile(at.x, north, at.floor).is_some())
        {
            return Ok(());
        }
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return Ok(());
        };
        let Some(policy) = native.item_policy_for_source_id(source) else {
            return Ok(());
        };
        let definition = &policy.record().production_definition;
        let item = crate::durability::item_mint::TypedDefinitionRef {
            family: definition.family.clone(),
            production_key: definition.production_key.clone(),
            revision_ref: definition.revision_ref.clone(),
        };
        if !oteryn_gameplay_items::item_ref_admission::generation_ground_item_facts(native, &item)
            .is_ok_and(|facts| {
                facts.stack
                    == oteryn_gameplay_domain::domain::item_definition::ItemStackClass::NonStackable
                    && facts.container_capacity.is_none()
            })
        {
            return Ok(());
        }
        let Some(fence) = self.current_quest_fence(session).await.ok().flatten() else {
            return Ok(());
        };
        let Some(command) = step_command(actor, session, entered.revision(), self.reset_epoch)
        else {
            return Ok(());
        };
        let (frame, content_digest) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || runtime.read_actor_position(actor) != Ok(entered)
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return Ok(());
            }
            let pin = runtime.content_pin();
            (pin.frame_binding_digest(), pin.server_artifact_digest())
        };
        if self
            .root
            .claim_arena_trophy_on_step(
                permit,
                self.character,
                self.holder,
                fence,
                ArenaTrophyStep {
                    command,
                    uid,
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!(
                        "sha256:{}",
                        content_digest
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ),
                    frame,
                    item,
                },
            )
            .await?
        {
            map.facts.emit_movement_effect(at, 14);
        }
        Ok(())
    }
}
