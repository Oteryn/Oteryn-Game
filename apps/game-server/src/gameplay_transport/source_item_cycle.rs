//! Bounded source-owned world deadlines in the actual Channel owner cycle.
use super::ComposedFreshAdmission;
use crate::foundation::RuntimeScopeRefV1;
use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn run_source_owner_cycles(
        &self,
        shutdown: &oteryn_foundation::CancellationToken,
    ) {
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_none()
        {
            return;
        }
        let wake = self.spell_states.lock().await.owner_wake.clone();
        loop {
            let mut notified = pin!(wake.notified());
            notified.as_mut().enable();
            let delay_us = {
                let runtime = self.runtime.lock().await;
                let states = self.spell_states.lock().await;
                if runtime.owner_fence().is_err() {
                    return;
                }
                let now = self.owner_now().get();
                states
                    .spell_timers
                    .as_ref()
                    .and_then(|timer| timer.next_due_time())
                    .map_or(1_000_000, |due| {
                        // An unavailable due producer cannot cause a busy loop.
                        // A newly committed schedule wakes this pass immediately.
                        let remaining = due.get().saturating_sub(now);
                        if remaining == 0 {
                            50_000
                        } else {
                            remaining.min(1_000_000)
                        }
                    })
            };
            let mut stopping = pin!(shutdown.cancelled());
            let mut delay = pin!(tokio::time::sleep(std::time::Duration::from_micros(
                delay_us
            )));
            let stopped = poll_fn(|context| {
                if stopping.as_mut().poll(context).is_ready() {
                    return Poll::Ready(true);
                }
                if notified.as_mut().poll(context).is_ready()
                    || delay.as_mut().poll(context).is_ready()
                {
                    Poll::Ready(false)
                } else {
                    Poll::Pending
                }
            })
            .await;
            if stopped {
                return;
            }
            if !self.ensure_source_map_initialized().await {
                continue;
            }
            self.drain_source_item_deadlines().await;
            self.drain_source_party_deadlines_bounded().await;
            let _ = self.drain_native_ai_timers().await;
        }
    }

    pub(super) async fn drain_source_party_deadlines_bounded(&self) {
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_none()
        {
            return;
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            let now = self.owner_now().get();
            if runtime.owner_fence().is_err() || now < states.next_party_deadline_pass_us {
                return;
            }
            states.next_party_deadline_pass_us = now.saturating_add(1_000_000);
        }
        let _ = self.drain_source_party_expiry().await;
        let _ = self.drain_source_party_offline_presence().await;
    }

    pub(super) async fn ensure_source_map_initialized(&self) -> bool {
        #[cfg(test)]
        let stage = |name: &str| {
            if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
                eprintln!("SEAM_EVIDENCE source_map_initialize stage={name}");
            }
        };
        #[cfg(test)]
        stage("entered");
        let Some(room) = self
            .qualified_room
            .filter(|room| room.source_world().is_some())
        else {
            return true;
        };
        let Some(source) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || source.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return false;
        }
        let binding = match crate::spell::source_map_initialization::CurrentNativeMapInitialization::qualify_current_binding(
            &runtime, room, source,
        ) {
            Ok(binding) => binding,
            Err(_reason) => {
                #[cfg(test)]
                stage(&format!("binding_refused:{_reason}"));
                return false;
            }
        };
        if states.source_map_initialized.as_ref() == Some(&binding) {
            return true;
        }
        let now = self.owner_now().get();
        if now < states.next_map_initialization_pass_us {
            return false;
        }
        states.next_map_initialization_pass_us = now.saturating_add(1_000_000);
        let proof = match crate::spell::source_map_initialization::CurrentNativeMapInitialization::from_current_owner(
            &runtime, room, source,
        ) {
            Ok(proof) => proof,
            Err(_reason) => {
                #[cfg(test)]
                stage(&format!("placements_refused:{_reason}"));
                return false;
            }
        };
        #[cfg(test)]
        stage("owner_sql_begin");
        if let Err(_error) = self
            .root
            .initialize_native_map_current_owner(self.character, self.holder, &proof)
            .await
        {
            #[cfg(test)]
            match _error {
                crate::durability::spell_item_transaction::SpellItemError::Database(
                    sqlx::Error::Database(error),
                ) => stage(&format!(
                    "owner_sql_refused:sqlstate={}",
                    error.code().as_deref().unwrap_or("unknown")
                )),
                crate::durability::spell_item_transaction::SpellItemError::Rejected(reason) => {
                    stage(&format!("owner_sql_refused:rejected={reason}"))
                }
                _ => stage("owner_sql_refused:other_error"),
            }
            return false;
        }
        #[cfg(test)]
        stage("owner_sql_committed");
        // Only a real SQL COMMIT sets this physical owner's readiness. Tile
        // reads still inspect current custody; this never restores moved items.
        states.source_map_initialized = Some(binding);
        true
    }

    pub(super) async fn drain_source_item_deadlines(&self) {
        let Some(source) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        let now = self.owner_now().get();
        if now < states.next_item_deadline_pass_us
            || runtime.owner_fence().is_err()
            || source.source_digest() != runtime.content_pin().server_artifact_digest()
        {
            return;
        }
        // Failed or uncertain commits also retry on the next bounded owner
        // pass. Immutable SQL deadlines reconcile them without another cause.
        states.next_item_deadline_pass_us = now.saturating_add(1_000_000);
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        let _ = self
            .root
            .drain_spell_item_deadlines(
                self.character,
                self.holder,
                scope,
                runtime.binding().scope_generation().get(),
            )
            .await;
    }
}
