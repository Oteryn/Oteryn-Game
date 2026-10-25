//! Existing physical realization and occurrence memo, shared by source and installed owners.
use super::*;
impl NativeSummonOwner {
    pub fn execute(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        catalog: &NativeSummonCatalog,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        self.execute_bound(runtime, Some(fence), stamp, catalog, proposal, policy)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_bound(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        catalog: &NativeSummonCatalog,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        let b = runtime.binding();
        let current = fence
            .or_else(|| runtime.owner_fence().ok())
            .ok_or(SummonError::StaleOwner)?;
        if !current.is_current_for_scope(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        ) || !current.accepts_stamp(stamp)
        {
            return Err(SummonError::StaleOwner);
        }
        if catalog.content != runtime.content_pin().server_artifact_digest() {
            return Err(SummonError::InvalidSource);
        }
        if !runtime.matches_live_creature_identity(
            proposal.occurrence.actor,
            catalog.parent.key.as_bytes(),
        ) || !current_source_creature_role(runtime, proposal.occurrence.actor)
            .map(CurrentSourceCreatureRole::is_ordinary)
            .unwrap_or(false)
        {
            return Err(SummonError::StaleParent);
        }
        if runtime
            .read_actor_position(proposal.target_with_path)
            .is_err()
        {
            return Err(SummonError::StaleTarget);
        }
        let source = catalog
            .entries
            .get(proposal.entry_index)
            .ok_or(SummonError::InvalidSource)?
            .as_ref()
            .map_err(|e| *e)?;
        if proposal.creature != source.child
            || proposal.maximum_total != source.total
            || proposal.maximum_of_creature != source.count
        {
            return Err(SummonError::InvalidSource);
        }
        let binding = Sha256::digest(format!("{:?}:{proposal:?}", source).as_bytes()).into();
        let parent = proposal.occurrence.actor;
        // The live parent's slot drops its former occupant's memos; the memos of other dead
        // parents go only when the table is full (Codex 4243039220).
        self.memos.evict_stale_at(parent);
        if let Some(m) = self.memos.get(parent, &source.index) {
            if proposal.occurrence.sequence < m.sequence {
                return Err(SummonError::Superseded);
            }
            if proposal.occurrence.sequence == m.sequence {
                return if binding == m.binding {
                    m.result.clone().map(|r| SummonReceipt {
                        newly_created: false,
                        ..r
                    })
                } else {
                    Err(SummonError::ReplayConflict)
                };
            }
        } else {
            // `CREATUREAI0-RL-04` live summoners times `CREATUREAI0-RL-08` entries (Codex
            // 4242954089): when full, the memos of dead parents are dropped first.
            if self.memos.len() >= NATIVE_SUMMON_MEMOS_PER_CHANNEL_MAX {
                self.memos.prune(|a| runtime.contains_live_creature(a));
            }
            if self.memos.len() >= NATIVE_SUMMON_MEMOS_PER_CHANNEL_MAX {
                return Err(SummonError::Capacity);
            }
        }
        let result = self.place(runtime, fence, stamp, source, proposal, policy);
        let memo = SummonMemo {
            sequence: proposal.occurrence.sequence,
            binding,
            result: result.clone(),
        };
        self.memos.insert(parent, source.index, memo);
        result
    }
    fn place(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: Option<&ScopeRuntimeFence>,
        stamp: RuntimeWorkStamp,
        source: &NativeSummonSpec,
        proposal: &ProfileSummonProposal,
        policy: &mut dyn SummonLocationPolicy,
    ) -> Result<SummonReceipt, SummonError> {
        if policy.current_target_reachable(
            runtime,
            proposal.occurrence.actor,
            proposal.target_with_path,
            stamp,
        ) != Some(true)
        {
            return Err(SummonError::MissingMap);
        }
        let center = runtime
            .read_actor_position(proposal.occurrence.actor)
            .map_err(SummonError::Carrier)?
            .position();
        let mut free = Vec::new();
        for (dx, dy) in [
            (0, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let position = MovementLocalPosition {
                x: center.x.checked_add(dx).ok_or(SummonError::InvalidSource)?,
                y: center.y.checked_add(dy).ok_or(SummonError::InvalidSource)?,
                floor: center.floor,
            };
            let allowed = policy
                .current_cell_admits(
                    runtime,
                    proposal.occurrence.actor,
                    &source.child,
                    position,
                    stamp,
                )
                .ok_or(SummonError::MissingMap)?;
            if allowed && runtime.native_summon_cell_free(position) {
                if dx == 0 && dy == 0 {
                    free.clear();
                    free.push(position);
                    break;
                }
                free.push(position)
            }
        }
        if free.is_empty() {
            return Err(SummonError::NoFreeTile);
        }
        let digest = Sha256::digest(format!("summon-place:{proposal:?}").as_bytes());
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let root = GameplayDecisionRoot::from_bytes(source.content);
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(id),
            "SUMMON_PLACE",
            source.index as u64,
        )
        .map_err(|_| SummonError::InvalidSource)?;
        let position = free[(draw % (free.len() as u64)) as usize];
        let child = match fence {
            Some(fence) => runtime.admit_native_summon(
                fence,
                stamp,
                proposal.occurrence.actor,
                &source.admission,
                position,
            ),
            None => runtime.admit_current_native_summon(
                stamp,
                proposal.occurrence.actor,
                &source.admission,
                position,
            ),
        }
        .map_err(SummonError::Carrier)?;
        Ok(SummonReceipt {
            child,
            newly_created: true,
            position,
        })
    }
}
