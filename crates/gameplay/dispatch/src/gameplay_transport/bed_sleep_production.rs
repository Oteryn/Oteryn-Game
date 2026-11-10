//! Production House-runtime qualification for bed sleep.
use super::*;
use crate::content::LogicalCell;
use crate::durability::bed_sleep::{BedSleepOwner, BedSleepRequest};
use crate::durability::character_last_position::{HouseLastPosition, LastPositionCell};
use crate::durability::house_scope_handoff::HouseId;
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::bed::BedSleepIntent;

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
        expected_revision: Option<u64>,
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
        let ItemKey::MapBase {
            reset_epoch,
            placement_key,
            ..
        } = target
        else {
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
        let changes = if expected_revision.is_some() {
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
            if let Some(revision) = expected_revision {
                use crate::map::view::MapFacts as _;
                let Some(map) = self.bundle_map() else {
                    return Ok(UseOutcome::rejected());
                };
                let base = map.facts.base_only();
                let facts = super::world_map::DecoratedFacts {
                    base: &base,
                    changes: changes.as_deref().unwrap_or_default(),
                };
                if facts.object_revision(placement_key) != revision {
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
        )
        .await
    }

    /// Read-only selection admission. The ordinary sleep owner repeats this proof
    /// and independently checks Premium, combat, ACL, occupancy and durable fences.
    pub(super) fn bed_modal_target(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        admitted: connection::AdmittedSession,
        target: ItemKey,
        changes: &[super::world_map::HouseBaseMapEntry],
    ) -> bool {
        let Some(actor) = admitted.runtime_actor else {
            return false;
        };
        let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
            return false;
        };
        self.bed_head_for_item(runtime, &state.house, state.position, target, Some(changes))
            .is_some()
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
        if position.floor != -i16::from(pos.floor)
            || position.x.abs_diff(i32::from(pos.x)) > 1
            || position.y.abs_diff(i32::from(pos.y)) > 1
        {
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
                return bundle_bed_head(map, &facts, native, index, house, pos, ordinal);
            }
            return bundle_bed_head(map, &map.facts, native, index, house, pos, ordinal);
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

fn bundle_bed_head(
    map: &crate::map::boot::BundleMap,
    facts: &impl crate::map::view::MapFacts,
    native: &crate::content::native_gameplay::NativeGameplayState,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    house: &str,
    pos: crate::map::overlay::TilePos,
    ordinal: u8,
) -> Option<LogicalCell> {
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
            let other = bundle_bed_part(
                map,
                facts,
                native,
                index,
                partner,
                u8::try_from(ordinal).ok()?,
            )?;
            (other.part != part.part && other.partner_direction == opposite).then_some(other)
        });
    matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let head = if part.part == Part::Head {
        pos
    } else {
        partner
    };
    Some(LogicalCell {
        x: i32::from(head.x),
        y: i32::from(head.y),
        z: -i32::from(head.floor),
    })
}
