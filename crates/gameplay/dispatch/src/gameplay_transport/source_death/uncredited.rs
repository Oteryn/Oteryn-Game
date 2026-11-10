//! Retain the canonical completion through the existing fenced empty-corpse writer.
use super::*;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CommittedItemMint, GroundPlacement, ItemMintCandidate,
    ItemMintCause, ItemMintError, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_mint_audit::RL08_RETRY_WORK_UNITS_MAX;
use oteryn_gameplay_entities::carrier::native_source_death::{
    SourceCreatureDeathCompletion, SourceCreatureDeathSeal,
};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingSourceCorpse {
    completed: SourceCreatureDeathCompletion,
    request: ItemMintRequest,
    candidate: Option<ItemMintCandidate>,
    mint: SourceCorpseMint,
}

#[derive(Debug)]
enum SourceCorpseMint {
    Ready { absence_proven: bool },
    Reconcile { terminal_refusal: bool },
    Committed(CommittedItemMint),
    Absent,
    Conflict,
}

impl SourceCorpseMint {
    fn begin_attempt(&mut self) -> Option<bool> {
        let Self::Ready { absence_proven } = *self else {
            return None;
        };
        // Publish Unknown before awaiting the writer: dropping its future must
        // not leave the previous absence proof usable for another attempt.
        *self = Self::Reconcile {
            terminal_refusal: false,
        };
        Some(absence_proven)
    }

    fn attempted(
        &mut self,
        result: Result<ItemMintOutcome, ItemMintError>,
        absence_before_attempt: bool,
    ) {
        *self = match result {
            Ok(
                ItemMintOutcome::Committed(receipt) | ItemMintOutcome::AlreadyCommitted(receipt),
            ) => Self::Committed(receipt),
            Err(
                ItemMintError::InvalidInput
                | ItemMintError::CapacityExceeded
                | ItemMintError::AuthorityRejected,
            ) => {
                // This attempt certainly did not commit, but Capacity also
                // names RL08 exhaustion after an earlier Unknown. Only a
                // previous cause-locked None proves that no earlier MINT exists.
                if absence_before_attempt {
                    Self::Absent
                } else {
                    Self::Reconcile {
                        terminal_refusal: true,
                    }
                }
            }
            Err(ItemMintError::ConflictingCause | ItemMintError::ConflictingCandidate) => {
                Self::Conflict
            }
            Err(ItemMintError::Unavailable(_)) => Self::Reconcile {
                terminal_refusal: false,
            },
        };
    }

    fn reconciled(
        &mut self,
        result: Result<Option<CommittedItemMint>, ItemMintError>,
        work_units_used: u8,
    ) {
        let Self::Reconcile { terminal_refusal } = *self else {
            return;
        };
        match result {
            Ok(Some(receipt)) => *self = Self::Committed(receipt),
            Ok(None) => {
                // Keep this certain absence even when the read consumed the
                // last unit. A later budget refusal cannot turn it into Unknown.
                *self = if terminal_refusal || work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
                    Self::Absent
                } else {
                    Self::Ready {
                        absence_proven: true,
                    }
                };
            }
            Err(ItemMintError::ConflictingCause | ItemMintError::ConflictingCandidate) => {
                *self = Self::Conflict;
            }
            Err(
                ItemMintError::InvalidInput
                | ItemMintError::CapacityExceeded
                | ItemMintError::AuthorityRejected
                | ItemMintError::Unavailable(_),
            ) => {}
        }
    }
}

impl PendingSourceCorpse {
    pub(super) fn request(
        native: &crate::content::native_gameplay::NativeGameplayState,
        begin: &SourceCreatureDeathSeal,
    ) -> Result<ItemMintRequest, ()> {
        if begin.definition_key() != "oteryn:creature.outburst"
            || begin.has_reward_damage_credit()
            || native.source_digest() != begin.content_digest()
            || !begin
                .events()
                .iter()
                .any(|event| event.name() == "HeartBossDeath")
        {
            return Err(());
        }
        let row = native
            .reward_table()
            .row(begin.definition_key())
            .map_err(|_| ())?;
        let corpse = row.corpse_item.as_ref().ok_or(())?;
        if corpse.production_key != "oteryn:item.tibia.i23564" {
            return Err(());
        }
        let item = TypedDefinitionRef {
            family: corpse.family.clone(),
            production_key: corpse.production_key.clone(),
            revision_ref: corpse.revision_ref.clone(),
        };
        let cause = ItemMintCause::from_creature_death(
            begin.death_key(),
            item.clone(),
            CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
            0,
        )
        .map_err(|_| ())?;
        let digest_text = |digest: [u8; 32]| {
            format!(
                "sha256:{}",
                digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            )
        };
        let at = begin.position();
        let mut spatial_position = Vec::with_capacity(10);
        spatial_position.extend_from_slice(&at.x.to_be_bytes());
        spatial_position.extend_from_slice(&at.y.to_be_bytes());
        spatial_position.extend_from_slice(&at.floor.to_be_bytes());
        Ok(ItemMintRequest {
            cause,
            item,
            quantity: 1,
            ground: GroundPlacement {
                spatial_position,
                corpse_ref: crate::durability::party_kill_xp::encode_death_key(begin.death_key()),
                map_revision: digest_text(begin.map_digest()),
                content_revision: digest_text(begin.content_digest()),
                native_room_placement_context: begin.frame_digest().to_vec(),
            },
            content_revision: digest_text(begin.content_digest()),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        })
    }

    pub(super) fn new(completed: SourceCreatureDeathCompletion, request: ItemMintRequest) -> Self {
        Self {
            completed,
            request,
            candidate: None,
            mint: SourceCorpseMint::Ready {
                absence_proven: false,
            },
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `true` means this owner turn belongs to retained corpse work, including an Unknown result.
    pub(in crate::gameplay_transport) async fn drain_uncredited_source_corpse(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        let retained = {
            self.states
                .spell_states
                .lock()
                .await
                .source_death_corpse
                .clone()
        };
        let Some(retained) = retained else {
            return false;
        };
        // The Arc stays in Channel state across cancellation; the non-Clone candidate keeps
        // its original durable identities and writer budget across every retry.
        let mut pending = retained.lock().await;
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return true;
        };
        {
            let runtime = self.runtime.lock().await;
            if !pending.completed.current_for(&runtime)
                || native.source_digest() != pending.completed.begin().content_digest()
            {
                return true;
            }
        }
        if pending.candidate.is_none() {
            let candidate = self
                .root
                .freeze_item_mint(self.character, self.holder, pending.request.clone())
                .await;
            let Ok(candidate) = candidate else {
                return true;
            };
            pending.candidate = Some(candidate);
        }
        if let Some(absence_before_attempt) = pending.mint.begin_attempt() {
            let PendingSourceCorpse {
                completed,
                candidate,
                ..
            } = &mut *pending;
            let Some(candidate) = candidate.as_mut() else {
                return true;
            };
            let outcome = self
                .root
                .commit_uncredited_source_corpse_mint(
                    permit,
                    self.character,
                    self.holder,
                    native,
                    completed,
                    candidate,
                )
                .await;
            pending.mint.attempted(outcome, absence_before_attempt);
        }
        if matches!(pending.mint, SourceCorpseMint::Reconcile { .. }) {
            let Some(candidate) = pending.candidate.as_mut() else {
                return true;
            };
            // Exhaustion without a successful cause-locked read remains Unknown.
            // Never re-freeze this cause to obtain new identities or work units.
            if candidate.work_units_used() >= RL08_RETRY_WORK_UNITS_MAX {
                return true;
            }
            let reconciled = self
                .root
                .reconcile_item_mint(self.character, candidate)
                .await;
            let used = candidate.work_units_used();
            pending.mint.reconciled(reconciled, used);
        }
        let mut runtime = self.runtime.lock().await;
        if !pending.completed.current_for(&runtime) {
            return true;
        }
        match &pending.mint {
            SourceCorpseMint::Committed(committed) => {
                if runtime
                    .bind_corpse_item(
                        pending.completed.actor(),
                        committed.item_instance_id,
                        &pending.request.item.family,
                        &pending.request.item.production_key,
                        &pending.request.item.revision_ref,
                    )
                    .is_err()
                {
                    return true;
                }
            }
            SourceCorpseMint::Absent => {}
            SourceCorpseMint::Ready { .. }
            | SourceCorpseMint::Reconcile { .. }
            | SourceCorpseMint::Conflict => return true,
        }
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return true;
        };
        if !states
            .source_death_corpse
            .as_ref()
            .is_some_and(|current| std::sync::Arc::ptr_eq(current, &retained))
        {
            return true;
        }
        if states
            .native_callbacks
            .acknowledge_outburst_null_death(&runtime, &pending.completed)
            .is_err()
        {
            return true;
        }
        let released = if matches!(pending.mint, SourceCorpseMint::Absent) {
            runtime.release_unminted_source_corpse_projection(&pending.completed)
        } else {
            runtime.release_uncredited_source_corpse_projection(&pending.completed)
        };
        if released.is_err() {
            return true;
        }
        self.states.loop_ground.invalidate();
        states.source_death_corpse = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unknown_attempt() -> SourceCorpseMint {
        let mut mint = SourceCorpseMint::Ready {
            absence_proven: false,
        };
        assert_eq!(mint.begin_attempt(), Some(false));
        mint.attempted(
            Err(ItemMintError::Unavailable(
                crate::durability::DurabilityError::CommitOutcomeUnknown,
            )),
            false,
        );
        mint
    }

    #[test]
    fn capacity_without_successful_reconciliation_cannot_release_the_source() {
        let mut mint = SourceCorpseMint::Ready {
            absence_proven: false,
        };
        assert_eq!(mint.begin_attempt(), Some(false));
        mint.attempted(Err(ItemMintError::CapacityExceeded), false);
        mint.reconciled(
            Err(ItemMintError::CapacityExceeded),
            RL08_RETRY_WORK_UNITS_MAX,
        );
        assert!(matches!(
            mint,
            SourceCorpseMint::Reconcile {
                terminal_refusal: true
            }
        ));
        assert_eq!(
            mint.begin_attempt(),
            None,
            "no repeated writer or replacement candidate"
        );
    }

    #[test]
    fn last_reconciliation_unit_keeps_certain_absence_terminal() {
        let mut mint = unknown_attempt();
        mint.reconciled(Ok(None), RL08_RETRY_WORK_UNITS_MAX);
        assert!(matches!(mint, SourceCorpseMint::Absent));
        assert_eq!(
            mint.begin_attempt(),
            None,
            "a proven exhausted cause needs no fourth write"
        );
    }

    #[test]
    fn cancelled_retry_invalidates_the_previous_absence_before_await() {
        let mut mint = unknown_attempt();
        mint.reconciled(Ok(None), 2);
        assert_eq!(mint.begin_attempt(), Some(true));
        // No result reaches `attempted`: the owner future was cancelled while
        // the last possible write might still commit. Exhaustion proves nothing.
        mint.reconciled(
            Err(ItemMintError::CapacityExceeded),
            RL08_RETRY_WORK_UNITS_MAX,
        );
        assert!(matches!(
            mint,
            SourceCorpseMint::Reconcile {
                terminal_refusal: false
            }
        ));
    }

    #[test]
    fn certain_retry_refusal_preserves_the_previous_cause_locked_absence() {
        let mut mint = unknown_attempt();
        mint.reconciled(Ok(None), 2);
        assert_eq!(mint.begin_attempt(), Some(true));
        mint.attempted(Err(ItemMintError::CapacityExceeded), true);
        assert!(matches!(mint, SourceCorpseMint::Absent));
    }

    #[test]
    fn integrity_conflict_cannot_be_acknowledged_as_an_absent_corpse() {
        let mut mint = unknown_attempt();
        mint.reconciled(Err(ItemMintError::ConflictingCause), 2);
        mint.reconciled(Ok(None), RL08_RETRY_WORK_UNITS_MAX);
        assert!(matches!(mint, SourceCorpseMint::Conflict));
        assert_eq!(mint.begin_attempt(), None);
    }
}
