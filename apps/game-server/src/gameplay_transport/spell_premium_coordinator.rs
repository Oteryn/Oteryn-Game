//! Candidate bounded account coordinator. Only fresh authenticated Platform pulls committed by
//! the real durable consumer issue a benefit view; restart never reconstructs cached grants.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::{
    ComposedFreshAdmission,
    spell_access_facts::{AccessFactsError, CurrentSpellAccessOwner, owner_registration},
    spell_entitlements::{PremiumAccessOwner, PremiumSource, TrustedTime},
};
use crate::durability::{
    fresh_admission::FreshAdmissionStore, item_transfer::CurrentCharacterItemFence,
};
use crate::foundation::{ExactActorRef, GameSessionId, GameSessionState, RuntimeScopeRefV1};
use crate::spell::owned_cast_facts::{AccessProjections, CastFactsBinding};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};
const MAX_ACCOUNTS: usize = 64;
/// Explicit registration from a genuine independent trusted clock owner. SystemTime, database
/// receipt time and a readiness field with guessed zero uncertainty cannot implement this port.
pub(crate) mod clock_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait RegisteredPremiumClock:
    clock_registration::Registered + Send + Sync
{
    fn read_current_window(&self) -> Option<TrustedTime>;
}
pub(crate) enum RegisteredSpellAccessOwner {
    Unavailable {
        account: Option<[u8; 16]>,
    },
    Premium {
        owner: Box<PremiumAccessOwner>,
        gate: AccountViewGate,
    },
}
impl RegisteredSpellAccessOwner {
    pub(crate) fn unavailable() -> Self {
        Self::Unavailable { account: None }
    }
}
impl owner_registration::Registered for RegisteredSpellAccessOwner {}
impl CurrentSpellAccessOwner for RegisteredSpellAccessOwner {
    fn account_id(&self) -> Option<[u8; 16]> {
        match self {
            Self::Unavailable { account } => *account,
            Self::Premium { owner, .. } => Some(owner.account()),
        }
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        match self {
            Self::Unavailable { .. } => Ok(AccessProjections::default()),
            Self::Premium { owner, gate } => match gate.current_time() {
                Some(time) => owner.read_current_at_trusted_time(binding, now, time),
                None => Ok(AccessProjections::default()),
            },
        }
    }
}
impl owner_registration::Registered for Arc<RegisteredSpellAccessOwner> {}
impl CurrentSpellAccessOwner for Arc<RegisteredSpellAccessOwner> {
    fn account_id(&self) -> Option<[u8; 16]> {
        self.as_ref().account_id()
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, AccessFactsError> {
        self.as_ref().read_current(binding, now)
    }
}
struct Entry {
    epoch: u64,
    in_flight: bool,
    view: Arc<RegisteredSpellAccessOwner>,
}
pub(crate) struct AccountViewGate {
    views: Weak<Mutex<BTreeMap<[u8; 16], Entry>>>,
    account: [u8; 16],
    epoch: u64,
    clock: Arc<dyn RegisteredPremiumClock>,
}
impl AccountViewGate {
    fn current_time(&self) -> Option<TrustedTime> {
        let views = self.views.upgrade()?;
        let time = self.clock.read_current_window();
        let mut views = views.lock().ok()?;
        let entry = views.get_mut(&self.account)?;
        if entry.epoch != self.epoch {
            return None;
        }
        if time.is_none() {
            entry.epoch = entry.epoch.saturating_add(1);
            entry.view = Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: Some(self.account),
            });
        }
        time
    }
}
pub(crate) struct SpellPremiumCoordinator {
    source: Option<Arc<PremiumSource>>,
    clock: Option<Arc<dyn RegisteredPremiumClock>>,
    views: Arc<Mutex<BTreeMap<[u8; 16], Entry>>>,
}
impl SpellPremiumCoordinator {
    pub(crate) fn new(
        source: Option<Arc<PremiumSource>>,
        clock: Option<Arc<dyn RegisteredPremiumClock>>,
    ) -> Self {
        Self {
            source,
            clock,
            views: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    pub(crate) fn current(&self, account: [u8; 16]) -> Arc<RegisteredSpellAccessOwner> {
        self.views
            .lock()
            .ok()
            .and_then(|views| views.get(&account).map(|entry| Arc::clone(&entry.view)))
            .unwrap_or_else(|| {
                Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some(account),
                })
            })
    }
    fn deny(&self, account: [u8; 16]) -> Arc<RegisteredSpellAccessOwner> {
        let denied = Arc::new(RegisteredSpellAccessOwner::Unavailable {
            account: Some(account),
        });
        if let Ok(mut views) = self.views.lock()
            && let Some(entry) = views.get_mut(&account)
        {
            entry.epoch = entry.epoch.saturating_add(1);
            entry.view = Arc::clone(&denied);
        }
        denied
    }
    fn finish(
        &self,
        account: [u8; 16],
        owner: PremiumAccessOwner,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let Some(clock) = &self.clock else {
            return self.deny(account);
        };
        let Ok(mut views) = self.views.lock() else {
            return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: Some(account),
            });
        };
        let Some(entry) = views.get_mut(&account) else {
            return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: Some(account),
            });
        };
        let Some(epoch) = entry.epoch.checked_add(1).filter(|v| *v < u64::MAX) else {
            entry.epoch = u64::MAX;
            entry.view = Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: Some(account),
            });
            return Arc::clone(&entry.view);
        };
        entry.epoch = epoch;
        let view = Arc::new(RegisteredSpellAccessOwner::Premium {
            owner: Box::new(owner),
            gate: AccountViewGate {
                views: Arc::downgrade(&self.views),
                account,
                epoch,
                clock: Arc::clone(clock),
            },
        });
        entry.view = Arc::clone(&view);
        view
    }
    async fn refresh(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        before: PremiumOwnerBinding,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let account = before.account;
        {
            let Ok(mut views) = self.views.lock() else {
                return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some(account),
                });
            };
            if !views.contains_key(&account) && views.len() >= MAX_ACCOUNTS {
                return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some(account),
                });
            }
            let entry = views.entry(account).or_insert_with(|| Entry {
                epoch: 0,
                in_flight: false,
                view: Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some(account),
                }),
            });
            if entry.in_flight {
                return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some(account),
                });
            }
            entry.in_flight = true;
        }
        struct Guard<'a> {
            coordinator: &'a SpellPremiumCoordinator,
            account: [u8; 16],
            completed: bool,
        }
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                if let Ok(mut views) = self.coordinator.views.lock()
                    && let Some(entry) = views.get_mut(&self.account)
                {
                    entry.in_flight = false;
                    if !self.completed {
                        entry.epoch = entry.epoch.saturating_add(1);
                        entry.view = Arc::new(RegisteredSpellAccessOwner::Unavailable {
                            account: Some(self.account),
                        });
                    }
                }
            }
        }
        let mut guard = Guard {
            coordinator: self,
            account,
            completed: false,
        };
        let Some(source) = &self.source else {
            return self.deny(account);
        };
        let Some(clock) = &self.clock else {
            return self.deny(account);
        };
        if clock.read_current_window().is_none() {
            return self.deny(account);
        }
        let snapshot = match source.pull(account).await {
            Ok(snapshot) => snapshot,
            Err(_) => {
                let _ = owner.root.premium_failed_pull(account).await;
                return self.deny(account);
            }
        };
        if snapshot.account() != account {
            return self.deny(account);
        }
        let Some(after) = owner
            .current_premium_binding(before.actor, before.session)
            .await
        else {
            return self.deny(account);
        };
        if after != before {
            return self.deny(account);
        }
        let committed = match owner.root.consume_premium_snapshot(snapshot).await {
            Ok(read) => read,
            Err(_) => return self.deny(account),
        };
        // A lease/root/actor handover during durable consumer commit never installs a live view.
        let Some(final_binding) = owner
            .current_premium_binding(before.actor, before.session)
            .await
        else {
            return self.deny(account);
        };
        if final_binding != before {
            return self.deny(account);
        }
        let observed = owner.owner_now().get();
        let Some(time) = clock.read_current_window() else {
            return self.deny(account);
        };
        let access = PremiumAccessOwner::from_committed(
            committed,
            time,
            observed,
            AccessProjections::default(),
        );
        let result = self.finish(account, access);
        guard.completed = true;
        result
    }
}
/// Private binding produced only by actual session/Character/recovery/node/owner reads below.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PremiumOwnerBinding {
    account: [u8; 16],
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    character_revision: u64,
    content_digest: [u8; 32],
    placement_identity: [u8; 16],
    placement_revision: u64,
    player_revision: u64,
    equipment: crate::durability::character_equipment::EquipmentSnapshot,
    build: crate::durability::character_build::DurableBuildState,
    level: u32,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    async fn current_premium_binding(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<PremiumOwnerBinding> {
        let active = self.active_generation?;
        let current = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?
            .0;
        if current.session_state() != GameSessionState::Active {
            return None;
        }
        let character =
            crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                .ok()?;
        let record = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        if record.world_id.as_bytes() != self.world_id.as_bytes() {
            return None;
        }
        let gameplay = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: record.revision,
        };
        let digest = active.identity().server_artifact_digest();
        let raw = self
            .root
            .read_lifecycle_durable_facts(self.character, self.holder, &gameplay, digest)
            .await
            .ok()?;
        if raw.account != *record.account_id.as_bytes() {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        if raw.fence.runtime_scope != scope
            || raw.fence.scope_ownership_generation != runtime.binding().scope_generation()
            || runtime.content_pin().server_artifact_digest() != digest
            || raw.equipment.character_revision != record.revision.get()
        {
            return None;
        }
        let physical = runtime.player_control_facts(actor, session).ok()?;
        if physical.control_loss.is_some() {
            return None;
        }
        let mut states = self.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session)?;
        let player_revision = state.revision();
        Some(PremiumOwnerBinding {
            account: raw.account,
            actor,
            session,
            fence: raw.fence,
            character_revision: record.revision.get(),
            content_digest: digest,
            placement_identity: physical.placement_identity,
            placement_revision: physical.placement_revision,
            player_revision,
            equipment: raw.equipment,
            build: raw.build,
            level: raw.level,
        })
    }
    /// Apply the account transition only to the genuine current actor, after all current
    /// SQL reads and before source caster facts are sampled. Pending accepted snapshots freeze.
    pub(super) async fn apply_refreshed_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &impl CurrentSpellAccessOwner,
    ) -> Result<bool, AccessFactsError> {
        let current = self.current_premium_binding(actor, session).await.ok_or(
            AccessFactsError::Unavailable("current Premium transition owners"),
        )?;
        if access
            .account_id()
            .is_some_and(|account| account != current.account)
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition account changed",
            ));
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return Ok(false);
        }
        if runtime.content_pin().server_artifact_digest() != current.content_digest
            || runtime.binding().scope_generation() != current.fence.scope_ownership_generation
            || current.fence.runtime_scope
                != RuntimeScopeRefV1::channel(
                    runtime.binding().world_id(),
                    runtime.binding().channel_id(),
                )
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition scope/Content changed",
            ));
        }
        let physical = runtime
            .player_control_facts(actor, session)
            .map_err(|_| AccessFactsError::Unavailable("Premium transition current actor"))?;
        if physical.control_loss.is_some()
            || physical.placement_identity != current.placement_identity
            || physical.placement_revision != current.placement_revision
        {
            return Err(AccessFactsError::Unavailable(
                "Premium transition placement changed",
            ));
        }
        let state =
            states
                .get_mut(&runtime, actor, session)
                .ok_or(AccessFactsError::Unavailable(
                    "Premium transition current player",
                ))?;
        if state.revision() != current.player_revision {
            return Err(AccessFactsError::Unavailable(
                "Premium transition player changed",
            ));
        }
        let now = self.owner_now().get();
        let binding = CastFactsBinding {
            actor,
            session,
            character: *current.fence.character_id.as_bytes(),
            character_revision: current.character_revision,
            lease_generation: current.fence.character_lease_generation,
            connection_generation: current.fence.connection_generation.get(),
            player_revision: current.player_revision,
            content_digest: current.content_digest,
            equipment_revision: current.equipment.revision,
        };
        let projections = access.read_current(&binding, now)?;
        let facts = crate::spell::owned_cast_facts::OwnedCastFacts::from_owner_reads(
            binding,
            current.build,
            current.level,
            current.equipment,
            projections,
        )?;
        states
            .apply_current_premium(&runtime, actor, session, &facts, now)
            .map_err(|_| AccessFactsError::Unavailable("Premium transition refused"))
    }
    pub(super) async fn refresh_and_apply_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let view = self.refresh_spell_access(actor, session).await;
        if self
            .apply_refreshed_spell_access(actor, session, &view)
            .await
            .is_err()
        {
            return Arc::new(RegisteredSpellAccessOwner::Unavailable {
                account: view.account_id(),
            });
        }
        view
    }
    /// The account selector is derived from current Game owners, never the client or caller.
    pub(super) async fn refresh_spell_access(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Arc<RegisteredSpellAccessOwner> {
        let Some(coordinator) = self.premium_coordinator else {
            return Arc::new(RegisteredSpellAccessOwner::unavailable());
        };
        let Some(before) = self.current_premium_binding(actor, session).await else {
            return Arc::new(RegisteredSpellAccessOwner::unavailable());
        };
        coordinator.refresh(self, before).await
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn restart_missing_source_and_clock_never_reconstruct_cached_allow() {
        let first = SpellPremiumCoordinator::new(None, None);
        let account = [1; 16];
        assert!(
            matches!(first.current(account).as_ref(),RegisteredSpellAccessOwner::Unavailable{account:Some(a)} if *a==account)
        );
        let second = SpellPremiumCoordinator::new(None, None);
        assert!(matches!(
            second.current(account).as_ref(),
            RegisteredSpellAccessOwner::Unavailable { .. }
        ));
    }
    #[test]
    fn failure_replaces_owned_view_and_never_changes_another_account() {
        let owner = SpellPremiumCoordinator::new(None, None);
        let a = [1; 16];
        let b = [2; 16];
        owner.views.lock().unwrap().insert(
            a,
            Entry {
                epoch: 0,
                in_flight: false,
                view: Arc::new(RegisteredSpellAccessOwner::Unavailable { account: Some(a) }),
            },
        );
        owner.views.lock().unwrap().insert(
            b,
            Entry {
                epoch: 0,
                in_flight: false,
                view: Arc::new(RegisteredSpellAccessOwner::Unavailable { account: Some(b) }),
            },
        );
        let retained_b = owner.current(b);
        let denied = owner.deny(a);
        assert!(Arc::ptr_eq(&denied, &owner.current(a)));
        assert!(Arc::ptr_eq(&retained_b, &owner.current(b)));
    }
    struct TestClock(Mutex<Option<TrustedTime>>);
    impl clock_registration::Registered for TestClock {}
    impl RegisteredPremiumClock for TestClock {
        fn read_current_window(&self) -> Option<TrustedTime> {
            *self.0.lock().ok()?
        }
    }
    #[test]
    fn failed_clock_invalidates_retained_epoch_even_if_clock_recovers() {
        // This fixture tests revocation of the private epoch gate only. It constructs
        // no Premium snapshot, committed receipt, benefit or production clock registration.
        let clock = Arc::new(TestClock(Mutex::new(
            TrustedTime::from_current_clock_owner(100, 101).ok(),
        )));
        let views = Arc::new(Mutex::new(BTreeMap::from([(
            [1; 16],
            Entry {
                epoch: 1,
                in_flight: false,
                view: Arc::new(RegisteredSpellAccessOwner::Unavailable {
                    account: Some([1; 16]),
                }),
            },
        )])));
        let gate = AccountViewGate {
            views: Arc::downgrade(&views),
            account: [1; 16],
            epoch: 1,
            clock: clock.clone(),
        };
        assert!(gate.current_time().is_some());
        *clock.0.lock().unwrap() = None;
        assert!(gate.current_time().is_none());
        *clock.0.lock().unwrap() = TrustedTime::from_current_clock_owner(102, 103).ok();
        assert!(gate.current_time().is_none());
        assert_eq!(views.lock().unwrap().get(&[1; 16]).unwrap().epoch, 2);
    }
    #[test]
    fn denied_account_revokes_its_retained_gate_without_revoking_other_accounts() {
        // Private epoch gate qualification only: no fake grant or Platform source.
        let clock = Arc::new(TestClock(Mutex::new(
            TrustedTime::from_current_clock_owner(100, 101).ok(),
        )));
        let coordinator = SpellPremiumCoordinator::new(None, Some(clock.clone()));
        for account in [[1; 16], [2; 16]] {
            coordinator.views.lock().unwrap().insert(
                account,
                Entry {
                    epoch: 1,
                    in_flight: false,
                    view: Arc::new(RegisteredSpellAccessOwner::Unavailable {
                        account: Some(account),
                    }),
                },
            );
        }
        let gate = |account| AccountViewGate {
            views: Arc::downgrade(&coordinator.views),
            account,
            epoch: 1,
            clock: clock.clone(),
        };
        let first = gate([1; 16]);
        let second = gate([2; 16]);
        assert!(first.current_time().is_some());
        assert!(second.current_time().is_some());
        coordinator.deny([1; 16]);
        assert!(first.current_time().is_none());
        assert!(second.current_time().is_some());
        assert_eq!(
            coordinator
                .views
                .lock()
                .unwrap()
                .get(&[1; 16])
                .unwrap()
                .epoch,
            2
        );
        coordinator.views.lock().unwrap().remove(&[2; 16]);
        assert!(second.current_time().is_none());
    }
}
