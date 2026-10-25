use super::super::item_ref_admission::{NativeItemWeights, definition_ref, pinned_index};
use super::super::world_spatial::VisibleKind;
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
use crate::foundation::RuntimeScopeRefV1;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn trade_open(
        &self,
        admitted: AdmittedSession,
    ) -> Option<()> {
        let actor = admitted.runtime_actor?;
        let name = self.chat_speaker_name(admitted.game_session_id).await?.0;
        let runtime = self.runtime.lock().await;
        runtime
            .player_control_facts(actor, admitted.game_session_id)
            .ok()?;
        let mut book = self.states.player_trades.0.lock().ok()?;
        if let Some(old) = book.live.get(&admitted.game_session_id)
            && old.admitted.continuity.connection_generation
                != admitted.continuity.connection_generation
            && let Some(record) = book.record(admitted.game_session_id)
            && record.request.is_none()
        {
            book.close(&record, ResultCode::Cancelled)?;
        }
        let live = book.live.entry(admitted.game_session_id).or_insert(Live {
            admitted,
            actor,
            name: name.clone(),
            revision: 1,
            state: State::default(),
            inventory_changed: false,
            rate: Rate::default(),
        });
        live.admitted = admitted;
        live.actor = actor;
        live.name = name;
        Some(())
    }

    pub(super) async fn trade_fences(
        &self,
        lives: &[Live; 2],
    ) -> Result<[CurrentCharacterItemFence; 2], ResultCode> {
        let mut fences = Vec::new();
        for live in lives {
            let current = super::super::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(live.admitted.game_session_id)
                .await
                .map_err(|_| ResultCode::Rejected)?
                .0;
            if current.session_state() != super::super::GameSessionState::Active {
                return Err(ResultCode::Stale);
            }
            let fence = super::super::item_fence_of(current).ok_or(ResultCode::Rejected)?;
            if Some(fence) != live.admitted.item_fence {
                return Err(ResultCode::Stale);
            }
            fences.push(fence);
        }
        fences.try_into().map_err(|_| ResultCode::Rejected)
    }

    pub(super) async fn trade_pair(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        lives: &[Live; 2],
        command: CommandRef,
    ) -> Result<(), ResultCode> {
        let states = self.states.spell_states.lock().await;
        for live in lives {
            let facts = runtime
                .player_control_facts(live.actor, live.admitted.game_session_id)
                .map_err(|_| ResultCode::Stale)?;
            if facts.control_loss.is_some()
                || states.is_dead(live.actor)
                || states.has_pending_spell_commit(live.actor, live.admitted.game_session_id)
            {
                return Err(ResultCode::Stale);
            }
        }
        drop(states);
        let binding = runtime.binding();
        if !binding.matches_committed_assignment(
            binding.world_id(),
            binding.channel_id(),
            self.holder.fact().node_id(),
            self.holder.fact().registration_revision(),
            binding.scope_generation().get(),
            binding.source_revision(),
            &format!("runtime-scope-assignment:{}", binding.source_revision()),
        ) {
            return Err(ResultCode::Rejected);
        }
        runtime.owner_fence().map_err(|_| ResultCode::Rejected)?;
        let positions = lives
            .each_ref()
            .map(|l| runtime.read_actor_position(l.actor).map(|p| p.position()));
        let [Ok(a), Ok(b)] = positions else {
            return Err(ResultCode::Stale);
        };
        if a.floor != b.floor || a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) > 2 {
            return Err(ResultCode::TooFar);
        }
        if lives[0].actor == lives[1].actor
            || lives[0].admitted.controller.is_none()
            || lives[0].admitted.controller.map(|c| c.account_id)
                == lives[1].admitted.controller.map(|c| c.account_id)
        {
            return Err(ResultCode::Rejected);
        }
        // Use the same bounded interest selection as domain 1, including its 256-result cut.
        let visible = runtime.visible_entities();
        let entities = visible
            .players
            .iter()
            .map(|e| (VisibleKind::Player, e))
            .chain(visible.creatures.iter().map(|e| (VisibleKind::Creature, e)))
            .chain(visible.npcs.iter().map(|e| (VisibleKind::Npc, e)))
            .map(|(kind, e)| super::super::world_spatial::ChannelEntity {
                kind,
                identity: e.actor.placement_identity(),
                generation: e.generation,
                position: super::super::world_spatial::ActorPosition {
                    x: e.position.x,
                    y: e.position.y,
                    floor: e.position.floor,
                },
                revision: e.revision,
            })
            .chain(super::super::item_ref_admission::visible_corpses(
                runtime,
                self.active_generation,
            ))
            .collect::<Vec<_>>();
        for side in 0..2 {
            let channel = super::super::world_spatial::ChannelEntities {
                observer: lives[side].actor.placement_identity(),
                content_generation: runtime.content_pin().client_artifact_digest(),
                entities: entities.clone(),
                lights: Default::default(),
                observed_at_ms: self.owner_now().get() / 1000,
            };
            let mut view = super::super::world_spatial::SessionVisibility::default();
            view.snapshot(&channel, &mut |_| Ok(()))
                .map_err(|_| ResultCode::Rejected)?;
            if !view.shows(entity(lives[1 - side].actor)) {
                return Err(ResultCode::Rejected);
            }
        }
        let fences = self.trade_fences(lives).await?;
        for fence in fences {
            if fence.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
                || fence.scope_ownership_generation != runtime.binding().scope_generation()
            {
                return Err(ResultCode::Stale);
            }
        }
        let digest = runtime.content_pin().server_artifact_digest();
        pinned_index(runtime, self.active_generation).ok_or(ResultCode::Rejected)?;
        let room = self
            .rooms
            .gate_attack_target()
            .ok_or(ResultCode::Rejected)?;
        let objects = self.door.lock().await;
        let mut context = (self, permit, runtime, &*objects, room);
        let sight = self
            .root
            .try_issue_semantic_pass()
            .map_err(|_| ResultCode::Rejected)?
            .run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, permit, runtime, objects, room) = ctx;
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let side =
                        usize::from(command.game_session_id() == lives[1].admitted.game_session_id);
                    let authority = item_tx::assert_spell_item_authority_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fences[side],
                        command,
                        digest,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let from = crate::spell::chain::TilePosition {
                        x: a.x,
                        y: a.y,
                        floor: a.floor,
                    };
                    let to = crate::spell::chain::TilePosition {
                        x: b.x,
                        y: b.y,
                        floor: b.floor,
                    };
                    // Occupied endpoints still need a qualified current map read, including
                    // the same-cell case whose endpoint-exclusive sight path is empty.
                    for p in [from, to] {
                        let flags = super::super::actor_spell::native_combat_cast::read_tile(
                            &mut tx, &authority, room, runtime, objects, p,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        if !flags.present {
                            return Ok(false);
                        }
                    }
                    for (p, exempt) in
                        super::super::actor_spell::native_combat_cast::sight_steps(from, to)
                            .map_err(|_| DurabilityError::Unavailable)?
                    {
                        let flags = super::super::actor_spell::native_combat_cast::read_tile(
                            &mut tx, &authority, room, runtime, objects, p,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                        if !flags.present || (!exempt && flags.projectile) {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                })
            })
            .await
            .map_err(|_| ResultCode::Rejected)?;
        if !sight {
            return Err(ResultCode::TooFar);
        }
        Ok(())
    }

    pub(super) fn trade_lives(&self, record: &Record) -> Option<[Live; 2]> {
        let book = self.states.player_trades.0.lock().ok()?;
        Some([
            book.live.get(&record.sessions[0])?.clone(),
            book.live.get(&record.sessions[1])?.clone(),
        ])
    }
    pub(super) async fn trade_unchanged(
        &self,
        record: &Record,
        fences: [CurrentCharacterItemFence; 2],
    ) -> Result<(), ResultCode> {
        for (side, fence) in fences.iter().enumerate() {
            let current = self
                .root
                .read_character_backpack(self.character, fence.character_id)
                .await
                .map_err(|_| ResultCode::Rejected)?;
            if current != record.backpacks[side] {
                return Err(ResultCode::Stale);
            }
        }
        Ok(())
    }
    pub(super) fn trade_projection(
        &self,
        runtime: &ChannelRuntimeV1,
        leaf: &TradeLeaf,
    ) -> Result<wire::Offer, ResultCode> {
        let (definition, count) = leaf
            .definition_and_quantity()
            .map_err(|_| ResultCode::Rejected)?;
        let index = pinned_index(runtime, self.active_generation).ok_or(ResultCode::Rejected)?;
        let native = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(ResultCode::Rejected)?;
        let Some((tradeable, pickupable)) =
            NativeItemWeights(native).player_trade_flags(&definition)
        else {
            return Err(ResultCode::NotTradeable);
        };
        oteryn_gameplay_economy::player_trade::LeafFacts {
            direct_backpack_entry: true,
            is_main_backpack: false,
            has_contents: false,
            pickupable,
            tradeable,
            quantity: count,
        }
        .check()
        .map_err(result)?;
        Ok(wire::Offer {
            definition: definition_ref(index, &definition)
                .ok_or(ResultCode::NotTradeable)?
                .get(),
            count,
            sub_type: 0,
        })
    }
}
