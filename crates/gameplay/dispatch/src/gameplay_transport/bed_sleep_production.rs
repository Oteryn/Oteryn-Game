//! Production House-runtime qualification for bed sleep.
use super::*;
use crate::content::LogicalCell;
use crate::durability::bed_sleep::{
    BedReservation, BedSleepOwner, BedSleepRequest, BedUseOutcome, BedUsePart, BedUsePlacement,
    BedUseReceipt, BedUseRequest,
};
use crate::durability::character_last_position::{HouseLastPosition, LastPositionCell};
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::house_scope_handoff::HouseId;
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::{bed::BedSleepIntent, world_object::UseDisposition};

#[cfg(test)]
tokio::task_local! {
    pub(super) static BED_USE_PLACEMENT_BARRIER: std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn production_bed_sleep(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: BedSleepIntent,
        target: ItemKey,
    ) -> Result<UseOutcome, crate::durability::DurabilityError> {
        self.production_bed_skill(actor, command, intent.skill, target, None)
            .await
    }

    pub(super) async fn production_bed_skill(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        skill: u8,
        target: ItemKey,
        reservation: Option<BedReservation>,
    ) -> Result<UseOutcome, crate::durability::DurabilityError> {
        if !self
            .session_premium_gate(command.game_session_id)
            .await
            .premium_current()
        {
            return Ok(UseOutcome::rejected());
        }
        let Some(_lane) = self.spell_lane_permit().await else {
            return Ok(UseOutcome::rejected());
        };
        {
            let states = self.states.spell_states.lock().await;
            if pvp_live::logout_blocked(&states, actor, command.game_session_id, self.owner_now()) {
                return Ok(UseOutcome::rejected());
            }
        }
        let ItemKey::MapBase { reset_epoch, .. } = target else {
            return Ok(UseOutcome::rejected());
        };
        let Some(current) =
            character_gameplay_fence::current(self, command.game_session_id, false).await
        else {
            return Ok(UseOutcome::rejected());
        };
        if command.item_fence != Some(current.item_fence()) || reset_epoch != self.reset_epoch {
            return Ok(UseOutcome::rejected());
        }
        let changes = if reservation.is_some() {
            let context = {
                let runtime = self.runtime.lock().await;
                let Ok(context) =
                    crate::durability::consume_transform::HouseActionContext::from_native_owner(
                        &runtime,
                        actor,
                        command.game_session_id,
                        current.item_fence(),
                    )
                else {
                    return Ok(UseOutcome::rejected());
                };
                context
            };
            let Some(map) = self.bundle_map() else {
                return Ok(UseOutcome::rejected());
            };
            Some(
                self.house_base_decoration_facts(&context, map.overlay.base().digest())
                    .await
                    .ok_or(crate::durability::DurabilityError::InvalidStoredState)?,
            )
        } else {
            None
        };
        let (session, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(command.game_session_id)
                .await?;
        let Some(transport) = session.current_transport() else {
            return Ok(UseOutcome::rejected());
        };
        let sex = self
            .root
            .read_character_sex(self.character, current.character_id)
            .await?;
        let Some(final_position) = self
            .final_last_position(command.game_session_id, actor)
            .await
            .filter(|p| p.record.rest.is_some())
        else {
            return Ok(UseOutcome::rejected());
        };
        let (presence, head, saved_house) = {
            let runtime = self.runtime.lock().await;
            // House custody is a distinct native actor owner; Channel position APIs
            // deliberately refuse it. Re-prove every field of the current fence.
            let Ok(state) = runtime.house_actor_state(actor, command.game_session_id) else {
                return Ok(UseOutcome::rejected());
            };
            if self.movement_cells().scope().world_id != self.world_id
                || self.movement_cells().scope().generation_digest
                    != runtime.content_pin().server_artifact_digest()
                || state.scope != current.runtime_scope
                || state.generation != current.scope_ownership_generation
                || state.lease.generation() != current.character_lease_generation
                || state.lease.character_id().as_bytes() != current.character_id.as_bytes()
            {
                return Ok(UseOutcome::rejected());
            }
            let Some(house) = HouseId::new(self.world_id, &state.house) else {
                return Ok(UseOutcome::rejected());
            };
            let Some(head) = self.bed_head_for_item(
                &runtime,
                &state.house,
                state.position,
                target,
                changes.as_deref(),
            ) else {
                return Ok(UseOutcome::rejected());
            };
            if let Some(reservation) = &reservation {
                let placement = reservation.placement;
                let changes = changes.as_deref().unwrap_or_default();
                let unchanged = [placement.head, placement.foot]
                    .into_iter()
                    .all(|part| bed_placement_part(changes, part.placement_key) == part);
                if !unchanged {
                    return Ok(UseOutcome {
                        disposition: UseDisposition::StaleState,
                        ..UseOutcome::rejected()
                    });
                }
            } else {
                let (Ok(x), Ok(y), Ok(floor)) = (
                    u16::try_from(head.x),
                    u16::try_from(head.y),
                    i8::try_from(-head.z),
                ) else {
                    return Ok(UseOutcome::rejected());
                };
                if self.states.server_modals.bed_reserved(
                    current.runtime_scope,
                    crate::map::overlay::TilePos { x, y, floor },
                ) {
                    return Ok(UseOutcome::rejected());
                }
            }
            let presence = NativeBedPresence {
                world: self.world_id,
                session: command.game_session_id,
                generation: state.generation.get(),
                digest: runtime.content_pin().server_artifact_digest(),
                house: state.house.clone(),
            };
            let saved_house = HouseLastPosition {
                house,
                origin_channel: self.channel_id,
                origin: LastPositionCell {
                    x: state.origin_position.x,
                    y: state.origin_position.y,
                    floor: state.origin_position.floor,
                },
            };
            (presence, head, saved_house)
        };
        // The fixed map proof is owned data. No runtime guard crosses durable IO.
        let Some(account) = self.session_account(command.game_session_id) else {
            return Ok(UseOutcome::rejected());
        };
        let owner = BedSleepOwner {
            root: self.root,
            recovery: self.character,
            node: self.holder,
            presence: &presence,
            request: BedSleepRequest {
                fence: current.item_fence(),
                transport,
                command_id: command.command_id,
                house: saved_house.house.clone(),
                head,
                sex,
                skill,
                final_position: Some(final_position),
            },
        };
        super::bed_sleep::finish_skill_with_house_position(
            command,
            skill,
            owner,
            Some((saved_house, super::canonical_uuid(&account))),
            reservation,
        )
        .await
    }

    pub(super) async fn production_bed_use(
        &self,
        admitted: connection::AdmittedSession,
        command_id: u64,
        context: HouseActionContext,
        placement: BedUsePlacement,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<BedUseOutcome, crate::durability::DurabilityError> {
        let Some(controller) = admitted.controller else {
            return Ok(BedUseOutcome::Denied);
        };
        let Ok(command) = crate::foundation::CommandId::new(command_id) else {
            return Ok(BedUseOutcome::Denied);
        };
        self.states
            .command_ingress
            .mark_bed_use(crate::foundation::CommandRef::new(
                admitted.game_session_id,
                command,
            ));
        #[cfg(test)]
        if let Ok(barrier) = BED_USE_PLACEMENT_BARRIER.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        let request = BedUseRequest {
            context,
            origin_channel: admitted.channel_id,
            transport: controller.transport,
            command_id,
            placement,
        };
        loop {
            let decision = self
                .root
                .use_house_bed(
                    self.character,
                    self.holder,
                    request.clone(),
                    budget.deadline.into_std(),
                )
                .await?;
            if !matches!(decision, BedUseOutcome::CheckoutUnavailable)
                || !budget
                    .retries_unavailable(connection::AdmissionRefusal::Unavailable)
                    .await
            {
                return Ok(decision);
            }
        }
    }

    pub(super) async fn reconcile_bed_use(
        &self,
        lost: &connection::AdmittedSession,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<(), connection::AdmissionRefusal> {
        use connection::AdmissionRefusal;
        let Some(command) = self.states.command_ingress.pending_bed_use(lost) else {
            return Ok(());
        };
        let (Some(fence), Some(controller)) = (lost.item_fence, lost.controller) else {
            return Err(AdmissionRefusal::Rejected);
        };
        let receipt = budget
            .retry_unavailable(async || {
                self.root
                    .read_house_bed_use_receipt(
                        fence,
                        controller.transport,
                        command.command_id(),
                        budget.deadline.into_std(),
                    )
                    .await
                    .map_err(|_| AdmissionRefusal::Unavailable)
            })
            .await?;
        let disposition = match receipt {
            BedUseReceipt::Woken => UseDisposition::BedWoken,
            BedUseReceipt::Occupied => UseDisposition::BedOccupied,
            // A vacant modal belongs to the lost connection. An absent
            // receipt after the durable loss fence proves no wake committed.
            BedUseReceipt::Vacant | BedUseReceipt::Absent => UseDisposition::Rejected,
            BedUseReceipt::BindingMismatch | BedUseReceipt::Unfenced => {
                return Err(AdmissionRefusal::Rejected);
            }
        };
        self.states
            .command_ingress
            .complete_item_use(command, disposition)
            .ok_or(AdmissionRefusal::Unavailable)
    }

    /// Prove both parts before applying reach, so an out-of-reach bed is
    /// claimed by this owner and cannot fall through to another item action.
    pub(super) fn bed_use_placement(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        admitted: connection::AdmittedSession,
        target: ItemKey,
        changes: &[super::world_map::HouseBaseMapEntry],
    ) -> Result<Option<BedUsePlacement>, UseDisposition> {
        let scope = self.movement_cells().scope();
        if scope.world_id != self.world_id
            || scope.generation_digest != runtime.content_pin().server_artifact_digest()
        {
            return Ok(None);
        }
        let ItemKey::MapBase {
            bundle_digest,
            placement_key,
            ..
        } = target
        else {
            return Ok(None);
        };
        let (Some(actor), Some(map), Some((pos, ordinal)), Some(native), Some(index)) = (
            admitted.runtime_actor,
            self.bundle_map(),
            crate::map::view::placement_of(placement_key),
            self.active_generation
                .and_then(|generation| generation.native_gameplay()),
            super::item_ref_admission::pinned_index(runtime, self.active_generation),
        ) else {
            return Ok(None);
        };
        let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
            return Ok(None);
        };
        let base = map.facts.base_only();
        let facts = super::world_map::DecoratedFacts {
            base: &base,
            changes,
        };
        let Some(pair) = bundle_bed_pair(map, &facts, native, index, &state.house, pos, ordinal)
        else {
            return Ok(None);
        };
        if !bed_part_reachable(state.position, pos) {
            return Err(UseDisposition::TooFar);
        }
        Ok(Some(BedUsePlacement {
            bundle_digest,
            head: bed_placement_part(changes, pair.head_placement),
            foot: bed_placement_part(changes, pair.foot_placement),
        }))
    }

    fn bed_head_for_item(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        house: &str,
        position: crate::foundation::LocalPosition,
        target: ItemKey,
        changes: Option<&[super::world_map::HouseBaseMapEntry]>,
    ) -> Option<LogicalCell> {
        let ItemKey::MapBase {
            placement_key,
            bundle_digest,
            reset_epoch,
        } = target
        else {
            return None;
        };
        if reset_epoch != self.reset_epoch
            || self.movement_cells().scope().world_id != self.world_id
            || self.movement_cells().scope().generation_digest
                != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let (pos, ordinal) = crate::map::view::placement_of(placement_key)?;
        if !bed_part_reachable(position, pos) {
            return None;
        }
        let index = super::item_ref_admission::pinned_index(runtime, self.active_generation)?;
        if let Some(map) = self.bundle_map() {
            // A map handle carries the bundle digest, not the gameplay artifact digest.
            if bundle_digest != map.overlay.base().digest() {
                return None;
            }
            let native = self.active_generation?.native_gameplay()?;
            if let Some(changes) = changes {
                let base = map.facts.base_only();
                let facts = super::world_map::DecoratedFacts {
                    base: &base,
                    changes,
                };
                return bundle_bed_pair(map, &facts, native, index, house, pos, ordinal)
                    .map(|pair| pair.head);
            }
            return bundle_bed_pair(map, &map.facts, native, index, house, pos, ordinal)
                .map(|pair| pair.head);
        }
        if bundle_digest != runtime.content_pin().server_artifact_digest() {
            return None;
        }
        let cell = LogicalCell {
            x: i32::from(pos.x),
            y: i32::from(pos.y),
            z: -i32::from(pos.floor),
        };
        self.movement_cells()
            .house_tiles()
            .house_at(self.movement_cells().scope(), cell)
            .ok()
            .flatten()
            .filter(|placement| placement.house().key == house)?
            .bed_head_at(cell)
    }
}

/// A proof copied only after the native House owner has checked the exact
/// actor, session, lease, content and reciprocal map parts under its lock.
struct NativeBedPresence {
    world: WorldId,
    session: GameSessionId,
    generation: u64,
    digest: [u8; 32],
    house: String,
}
impl crate::durability::spell_house_abi::house_presence_seal::Sealed for NativeBedPresence {}
impl crate::durability::spell_house_abi::HousePresenceProof for NativeBedPresence {
    fn world(&self) -> WorldId {
        self.world
    }
    fn actor_session(&self) -> GameSessionId {
        self.session
    }
    fn scope_generation(&self) -> u64 {
        self.generation
    }
    fn content_digest(&self) -> [u8; 32] {
        self.digest
    }
    fn house_key(&self) -> &str {
        &self.house
    }
    fn has_door(&self, _: u32) -> bool {
        false
    }
}

fn bundle_bed_part<'a>(
    map: &crate::map::boot::BundleMap,
    facts: &impl crate::map::view::MapFacts,
    native: &'a crate::content::native_gameplay::NativeGameplayState,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    pos: crate::map::overlay::TilePos,
    ordinal: u8,
) -> Option<&'a crate::content::ReferenceItemBed> {
    let base = map.overlay.base();
    let tile = base.tile(pos.x, pos.y, pos.floor)?;
    let id = *tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))?
        .0;
    let key = crate::map::view::placement_key(pos, ordinal)?;
    if map
        .overlay
        .tile(pos)
        .is_some_and(|tile| tile.is_hidden(ordinal))
        || facts.base_hidden(key)
    {
        return None;
    }
    let facts = facts.base_entry(pos, ordinal, id)?;
    if facts.pickupable {
        return None;
    }
    let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) = facts.definition else {
        return None;
    };
    let (key, revision) = index.definition_at(reference)?;
    let record = native.item_policy(key, revision)?.record();
    let crate::content::ReferenceItemField::Known(bed) = &record.semantics.bed else {
        return None;
    };
    Some(bed)
}

fn bed_placement_part(
    changes: &[super::world_map::HouseBaseMapEntry],
    placement_key: u64,
) -> BedUsePart {
    BedUsePart {
        placement_key,
        revision: changes
            .binary_search_by_key(&placement_key, |change| change.placement_key)
            .ok()
            .map_or(0, |at| changes[at].revision),
    }
}

pub(super) fn bed_part_reachable(
    position: crate::foundation::LocalPosition,
    part: crate::map::overlay::TilePos,
) -> bool {
    position.floor == -i16::from(part.floor)
        && position.x.abs_diff(i32::from(part.x)) <= 1
        && position.y.abs_diff(i32::from(part.y)) <= 1
}

struct BundleBedPair {
    head: LogicalCell,
    head_placement: u64,
    foot_placement: u64,
}

fn bundle_bed_pair(
    map: &crate::map::boot::BundleMap,
    facts: &impl crate::map::view::MapFacts,
    native: &crate::content::native_gameplay::NativeGameplayState,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    house: &str,
    pos: crate::map::overlay::TilePos,
    ordinal: u8,
) -> Option<BundleBedPair> {
    // Crystal 201189f `src/items/bed.cpp:91-100,112-123,145-160` requires the
    // reciprocal physical half and accepts USE only through the pillow/head.
    // Oteryn canonicalizes either visible half to that single reservation cell.
    use crate::content::{ReferenceBedDirection as Direction, ReferenceBedPart as Part};
    let source = super::house_query::source_house_id(house)?;
    if map.facts.house_id(pos) != Some(source) {
        return None;
    }
    let part = bundle_bed_part(map, facts, native, index, pos, ordinal)?;
    let (dx, dy, opposite) = match part.partner_direction {
        Direction::North => (0, -1, Direction::South),
        Direction::East => (1, 0, Direction::West),
        Direction::South => (0, 1, Direction::North),
        Direction::West => (-1, 0, Direction::East),
    };
    let partner = crate::map::overlay::TilePos {
        x: u16::try_from(i32::from(pos.x) + dx).ok()?,
        y: u16::try_from(i32::from(pos.y) + dy).ok()?,
        floor: pos.floor,
    };
    if map.facts.house_id(partner) != Some(source) {
        return None;
    }
    let tile = map
        .overlay
        .base()
        .tile(partner.x, partner.y, partner.floor)?;
    let mut matches = tile
        .depths()
        .iter()
        .filter(|depth| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, _)| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let other = bundle_bed_part(map, facts, native, index, partner, ordinal)?;
            (other.part != part.part && other.partner_direction == opposite).then_some(ordinal)
        });
    let partner_ordinal = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let target_placement = crate::map::view::placement_key(pos, ordinal)?;
    let partner_placement = crate::map::view::placement_key(partner, partner_ordinal)?;
    let (head, head_placement, foot_placement) = if part.part == Part::Head {
        (pos, target_placement, partner_placement)
    } else {
        (partner, partner_placement, target_placement)
    };
    Some(BundleBedPair {
        head: LogicalCell {
            x: i32::from(head.x),
            y: i32::from(head.y),
            z: -i32::from(head.floor),
        },
        head_placement,
        foot_placement,
    })
}
