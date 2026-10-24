use super::*;

/// MAP-ITEM-REF-1 Part B: the durable corpse Item a projected death materialized as. The Channel
/// owner binds it once the corpse's MINT has committed: KILL-REWARD part B calls
/// [`ChannelRuntimeV1::bind_corpse_item`] with the item instance id `commit_corpse_mint` returned
/// and the corpse's typed definition. Not durable, like the projection it belongs to: an unbound
/// corpse is neither shown nor usable, so nothing ever names a corpse Item by a made-up id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCorpseItem {
    pub item_instance_id: [u8; 16],
    pub family: String,
    pub production_key: String,
    pub revision_ref: String,
}

/// MAP-ITEM-REF-1 Part B: one bound corpse of [`ChannelRuntimeV1::visible_corpses`], at its
/// projected position. `identity` is [`ExactActorRef::corpse_identity`] of the dead actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRuntimeCorpse {
    pub identity: [u8; 16],
    pub position: MovementLocalPosition,
    pub revision: u64,
    pub item: RuntimeCorpseItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorpseItemBinding {
    actor: ExactActorRef,
    item: RuntimeCorpseItem,
}

impl ChannelActorCarrier {
    pub(super) fn bind_corpse_item_inner(
        &mut self,
        actor: ExactActorRef,
        item: RuntimeCorpseItem,
    ) -> Result<(), CarrierError> {
        if actor.world_id() != self.world_id
            || actor.channel_id() != self.channel_id
            || actor.scope_generation() != self.scope_generation
            || !self
                .corpse_projections
                .iter()
                .any(|projection| projection.occurrence.actor == actor)
        {
            return Err(CarrierError::CommittedLethalUnavailable);
        }
        if let Some(bound) = self
            .corpse_item_bindings
            .iter()
            .find(|binding| binding.actor == actor)
        {
            return if bound.item == item {
                Ok(())
            } else {
                Err(CarrierError::CorpseProjectionConflict)
            };
        }
        if self
            .corpse_item_bindings
            .iter()
            .any(|binding| binding.item.item_instance_id == item.item_instance_id)
        {
            return Err(CarrierError::CorpseProjectionConflict);
        }
        let projections = &self.corpse_projections;
        self.corpse_item_bindings.retain(|binding| {
            projections
                .iter()
                .any(|projection| projection.occurrence.actor == binding.actor)
        });
        self.corpse_item_bindings
            .try_reserve(1)
            .map_err(|_| CarrierError::AllocationFailed)?;
        self.corpse_item_bindings
            .push(CorpseItemBinding { actor, item });
        Ok(())
    }

    pub(super) fn bound_corpses(
        &self,
        context: Option<PreProductionPositionContext>,
    ) -> Vec<VisibleRuntimeCorpse> {
        self.corpse_projections
            .iter()
            .filter(|projection| {
                context.is_none_or(|context| projection.position.context == context)
            })
            .filter_map(|projection| {
                let actor = projection.occurrence.actor;
                let binding = self
                    .corpse_item_bindings
                    .iter()
                    .find(|binding| binding.actor == actor)?;
                Some(VisibleRuntimeCorpse {
                    identity: actor.corpse_identity(),
                    position: projection.position(),
                    revision: projection.position_revision(),
                    item: binding.item.clone(),
                })
            })
            .collect()
    }
}

impl ChannelRuntimeV1 {
    /// MAP-ITEM-REF-1 Part B: bind `actor`'s projected corpse to the corpse Item its committed
    /// MINT created (`item_instance_id` and the typed definition `commit_corpse_mint` was given).
    /// The projection must be retained in this generation (`CommittedLethalUnavailable`
    /// otherwise); binding the same Item again is a no-op and another Item, or an Item bound to
    /// another corpse, is `CorpseProjectionConflict`. Bindings whose projection was evicted are
    /// dropped here, so there is at most one per retained projection.
    #[allow(
        dead_code,
        reason = "the interface KILL-REWARD part B calls after commit_corpse_mint"
    )]
    pub fn bind_corpse_item(
        &mut self,
        actor: ExactActorRef,
        item_instance_id: [u8; 16],
        family: &str,
        production_key: &str,
        revision_ref: &str,
    ) -> Result<(), CarrierError> {
        self.carrier.bind_corpse_item_inner(
            actor,
            RuntimeCorpseItem {
                item_instance_id,
                family: family.to_owned(),
                production_key: production_key.to_owned(),
                revision_ref: revision_ref.to_owned(),
            },
        )
    }

    /// Codex 4248192466 (LOOT-LIVE-1): keep `actor`'s retained corpse projection from eviction
    /// until [`Self::unpin_corpse_projection`], so the kill settlement the queue accepted for it
    /// can still bind its committed corpse Item. Pinning a pinned projection is a no-op. A
    /// projection that is not retained in this generation is `CommittedLethalUnavailable`; more
    /// than `MAX_PINNED_CORPSE_PROJECTIONS` pins are `CapacityExceeded`. Runtime memory only.
    pub fn pin_corpse_projection(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.carrier.pin_corpse_projection_inner(actor)
    }

    /// Codex 4248192466: release `actor`'s pin once its kill settlement is final. The projection
    /// stays retained until the bounded FIFO evicts it. Unpinning an unpinned actor is a no-op.
    pub fn unpin_corpse_projection(&mut self, actor: ExactActorRef) {
        self.carrier.unpin_corpse_projection_inner(actor);
    }

    /// Codex 4248192466: release every pin whose actor is not in `held`, the killed actors of the
    /// kill settlements still queued, parked or in flight. An entry freed without a settle (a
    /// `principal_gone` refusal) leaves its pin until this runs.
    pub fn retain_corpse_projection_pins(&mut self, held: &[ExactActorRef]) {
        self.carrier
            .corpse_projection_pins
            .retain(|pinned| held.contains(pinned));
    }

    /// Whether `actor`'s corpse projection is pinned against eviction.
    #[cfg(any(test, feature = "test-support"))]
    pub fn corpse_projection_pinned(&self, actor: ExactActorRef) -> bool {
        self.carrier.corpse_projection_pins.contains(&actor)
    }

    /// Whether `actor`'s committed death is still projected and retained in this generation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn retains_corpse_projection(&self, actor: ExactActorRef) -> bool {
        self.carrier.retains_corpse_projection(actor)
    }

    /// MAP-ITEM-REF-1 Part B: every bound corpse of this generation under the pinned Movement
    /// context, in projection order. A corpse without a binding, or whose projection is gone, is
    /// left out. Read-only.
    pub fn visible_corpses(&self) -> Vec<VisibleRuntimeCorpse> {
        self.carrier
            .bound_corpses(Some(self.pinned_position_context()))
    }

    /// MAP-ITEM-REF-1 Part B: the bound corpse whose [`ExactActorRef::corpse_identity`] is
    /// `identity`, as [`Self::visible_corpses`] shows it. Read-only.
    pub fn bound_corpse(&self, identity: [u8; 16]) -> Option<VisibleRuntimeCorpse> {
        self.visible_corpses()
            .into_iter()
            .find(|corpse| corpse.identity == identity)
    }
}
