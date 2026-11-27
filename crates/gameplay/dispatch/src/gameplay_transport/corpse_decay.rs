//! D113/D135 and NIGHT-W1 decision 11: corpse deadlines are independent of ordinary Ground.
use super::ComposedFreshAdmission;
use crate::durability::DurabilityError;
use crate::durability::item_decay_retire::{CorpseDecayFence, DecayRetireError};
use std::time::Duration;

pub(super) const DECAY_HOLDER_RETRIES: usize = 2;
const DECAY_HOLDER_RETRY_WINDOW: Duration = Duration::from_millis(50);

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called by the server's one-second item clock, including with no connected players.
    /// The complete bounded read also recovers missed/partial drains after a restart. Each
    /// retirement re-proves the deadline and current scope authority inside the existing writer.
    pub(super) async fn drain_corpse_deadlines(&self) {
        self.root
            .with_background_operation(self.reconcile_corpse_schedule(true))
            .await;
    }

    pub(super) async fn reconcile_corpse_schedule(&self, retire_due: bool) {
        let deadline = tokio::time::Instant::now() + crate::durability::ROOT_RECOVERY_WINDOW;
        for retry in 0..=DECAY_HOLDER_RETRIES {
            let Err(error) = self.try_reconcile_corpse_schedule(retire_due).await else {
                return;
            };
            self.log_corpse_read_deferral("schedule", retirement_deferral_reason(&error), retry)
                .await;
            if !matches!(
                error,
                DecayRetireError::Unavailable(DurabilityError::RootUnavailable)
            ) || retry == DECAY_HOLDER_RETRIES
            {
                return;
            }
            if !matches!(
                tokio::time::timeout_at(deadline, self.root.await_ready()).await,
                Ok(Ok(()))
            ) {
                return;
            }
        }
    }

    async fn try_reconcile_corpse_schedule(
        &self,
        retire_due: bool,
    ) -> Result<(), DecayRetireError> {
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(());
        };
        let fence = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().world_id() != self.world_id {
                return Ok(());
            }
            let Ok(owner) = runtime.owner_fence() else {
                return Ok(());
            };
            CorpseDecayFence {
                runtime_scope: runtime.binding().runtime_scope(),
                scope_ownership_generation: owner.generation(),
            }
        };
        let schedule = self
            .root
            .read_scoped_corpse_decay_schedule(self.character, fence.runtime_scope)
            .await?;
        let mut live: Vec<_> = schedule
            .corpses
            .iter()
            .map(|corpse| corpse.corpse_item_instance_id)
            .collect();
        let mut retired = None;
        if retire_due {
            let due: Vec<_> = schedule
                .corpses
                .iter()
                .filter(|corpse| {
                    corpse
                        .decay_at_unix_ms
                        .is_some_and(|deadline| deadline <= schedule.database_now_unix_ms)
                })
                .collect();
            let start = self.background_start("corpse", due.len());
            if let Some(corpse) = due.get(start) {
                match self
                    .root
                    .retire_decayed_corpse(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        corpse.corpse_item_instance_id,
                    )
                    .await
                {
                    Ok(_) => {
                        live.retain(|item| *item != corpse.corpse_item_instance_id);
                        retired = Some(corpse.corpse_item_instance_id);
                        if let Ok(mut cursors) = self.states.background_cursors.lock() {
                            cursors.insert("corpse", start + 1);
                        }
                    }
                    Err(error) => {
                        log_decay_deferral(
                            "retire",
                            Some((
                                corpse.corpse_item_instance_id,
                                &corpse.definition.production_key,
                            )),
                            retirement_deferral_reason(&error),
                            0,
                        );
                    }
                }
            }
        }
        // A prior ambiguous commit may already have removed a corpse from the schedule. Only
        // a successful complete read may hide it, and no runtime guard crosses a durable call.
        let mut runtime = self.runtime.lock().await;
        if runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
            && runtime.binding().runtime_scope() == fence.runtime_scope
        {
            self.states
                .loop_ground
                .reconcile_corpses(fence.runtime_scope, &live);
            if let Some(item) = retired {
                let _ = runtime.retire_committed_corpse_item(item);
            }
            let _ = runtime.retain_live_corpse_items(&live);
            for corpse in &schedule.corpses {
                let _ = runtime.refresh_corpse_item(
                    corpse.corpse_item_instance_id,
                    &corpse.definition.family,
                    &corpse.definition.production_key,
                    &corpse.definition.revision_ref,
                );
            }
        }
        Ok(())
    }

    pub(super) async fn wait_for_decay_holder(&self) {
        let now = tokio::time::Instant::now();
        self.root
            .wait_holder_retry(now, now + DECAY_HOLDER_RETRY_WINDOW)
            .await;
    }

    pub(super) async fn log_corpse_read_deferral(&self, phase: &str, reason: &str, retry: usize) {
        let corpses = self.runtime.lock().await.visible_corpses();
        if corpses.is_empty() {
            log_decay_deferral(phase, None, reason, retry);
        }
        for corpse in corpses {
            log_decay_deferral(
                phase,
                Some((corpse.item.item_instance_id, &corpse.item.production_key)),
                reason,
                retry,
            );
        }
    }
}

fn log_decay_deferral(phase: &str, corpse: Option<([u8; 16], &str)>, reason: &str, retry: usize) {
    let (corpse_id, stage) = corpse
        .map(|(id, stage)| (super::format_connection_trace(id), stage))
        .unwrap_or_else(|| ("unknown".into(), "unknown"));
    super::operator_event(&format!(
        "event=corpse_decay_deferred corpse_id={corpse_id} stage={stage} phase={phase} reason={reason} retry={retry}"
    ));
}

fn retirement_deferral_reason(error: &DecayRetireError) -> &'static str {
    match error {
        DecayRetireError::Unavailable(error) => durability_deferral_reason(error),
        DecayRetireError::AuthorityRejected => "authority",
        DecayRetireError::Refused(_) => "retirement_refused",
        _ => "retirement_rejected",
    }
}

pub(super) fn durability_deferral_reason(error: &DurabilityError) -> &'static str {
    match error {
        DurabilityError::RootUnavailable => "holder_unavailable",
        DurabilityError::Database(_) => "database",
        DurabilityError::RootPassDeadlineExceeded | DurabilityError::CommitTransactionTimeout => {
            "deadline"
        }
        DurabilityError::CommitOutcomeUnknown => "commit_unknown",
        DurabilityError::InvalidStoredState => "invalid_state",
        _ => "durability",
    }
}
