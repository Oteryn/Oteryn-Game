//! Actual familiar source casts in the Channel's existing player/physical/timer owners.
//! Prepared intent survives an uncertain database outcome; history never grants fresh authority.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::super::ComposedFreshAdmission;
use super::{ChannelSpellStates, SpellCastIntent, SpellCastOutcome};
use crate::durability::character_familiar::FamiliarStateRequest;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
use crate::spell::combat_batch::SpellAnchor;
use crate::spell::companion_lifecycle::PreparedFamiliar;
use crate::spell::owned_cast_facts::{
    AccessProjections, CastFactsBinding, CurrentProjection, OwnedCastFacts,
};
use std::num::NonZeroU32;
#[path = "familiar_cast_dispatch.rs"]
mod dispatch;
// Observability only. This duration grants no gameplay/DB/timer authority.
static FAMILIAR_LAST_OWNER_LOCK_MICROS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static FAMILIAR_MAX_OWNER_LOCK_MICROS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
struct FamiliarOwnerTurnMeasurement(std::time::Instant);
impl Drop for FamiliarOwnerTurnMeasurement {
    fn drop(&mut self) {
        let elapsed = u64::try_from(self.0.elapsed().as_micros()).unwrap_or(u64::MAX);
        FAMILIAR_LAST_OWNER_LOCK_MICROS.store(elapsed, std::sync::atomic::Ordering::Relaxed);
        FAMILIAR_MAX_OWNER_LOCK_MICROS.fetch_max(elapsed, std::sync::atomic::Ordering::Relaxed);
    }
}
pub(crate) fn familiar_owner_lock_measurement() -> (u64, u64) {
    (
        FAMILIAR_LAST_OWNER_LOCK_MICROS.load(std::sync::atomic::Ordering::Relaxed),
        FAMILIAR_MAX_OWNER_LOCK_MICROS.load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// Source config/account projection. A registered owner supplies actual authenticated VIP/God
/// and server configuration with its independent revision and expiry; missing is unavailable.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FamiliarSourceSettings {
    benefit_use: FamiliarBenefitUse,
    pub(crate) account_at_least_god: bool,
    pub(crate) premium: bool,
    pub(crate) premium_revision: u64,
    pub(crate) familiar_minutes: i64,
    pub(crate) vip: bool,
    pub(crate) vip_reduction_minutes: i64,
    pub(crate) cooldown_rate: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamiliarBenefitUse {
    CurrentEligibility,
    NotApplicableDeath,
}
#[derive(Debug, Clone)]
pub(crate) struct PreparedFamiliarCast {
    actor: ExactActorRef,
    session: GameSessionId,
    command_id: u64,
    intent: SpellCastIntent,
    spell_index: NonZeroU32,
    fence: CurrentCharacterGameplayFence,
    request: FamiliarStateRequest,
    familiar: PreparedFamiliar,
    before: PlayerSpellState,
    paid: PlayerSpellState,
    anchor: SpellAnchor,
    facts_binding: CastFactsBinding,
    settings: CurrentProjection<FamiliarSourceSettings>,
    training: PreparedPlayerTraining,
    touches: Option<crate::foundation::QualifiedCompanionTouches>,
    physical: Option<crate::foundation::StagedSpellBatch>,
}
/// Exact genuine actor-end intent retained before a possibly ambiguous durable write.
#[derive(Debug, Clone)]
pub(crate) struct PreparedFamiliarLogout {
    actor: ExactActorRef,
    session: GameSessionId,
    loss_epoch: u64,
    fence: CurrentCharacterGameplayFence,
    request: FamiliarStateRequest,
    before: PlayerSpellState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gameplay_transport) enum FamiliarLogoutSave {
    Saved,
    NotApplicable,
    FencedOut,
    Unknown,
}
impl ChannelSpellStates {
    pub(crate) fn has_pending_familiar(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        self.pending_familiars
            .iter()
            .any(|v| v.actor == actor && v.session == session)
            || self
                .pending_familiar_lifecycle
                .iter()
                .any(|v| v.actor == actor && v.session == session)
            || self
                .pending_familiar_logouts
                .iter()
                .any(|v| v.actor == actor && v.session == session)
    }
}

// Only a matched private original uses this absence-preserving adapter. An access
// failure supplies no Premium/learning/Wheel facts; the source writer then resolves
// history only. Independently current SQL/actor/content checks are never bypassed.
struct FamiliarRetryAccess<'a, A> {
    owner: &'a A,
    original_retained: bool,
}
impl<A> super::super::spell_access_facts::owner_registration::Registered
    for FamiliarRetryAccess<'_, A>
{
}
impl<A: super::super::spell_access_facts::CurrentSpellAccessOwner>
    super::super::spell_access_facts::CurrentSpellAccessOwner for FamiliarRetryAccess<'_, A>
{
    fn account_id(&self) -> Option<[u8; 16]> {
        self.owner.account_id()
    }
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Result<AccessProjections, super::super::spell_access_facts::AccessFactsError> {
        match self.owner.read_current(binding, now) {
            Err(_) if self.original_retained => Ok(AccessProjections::default()),
            result => result,
        }
    }
}

pub(crate) mod source_registration {
    pub(crate) trait Registered {}
}
pub(crate) trait CurrentFamiliarSourceOwner: source_registration::Registered {
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
    ) -> Option<CurrentProjection<FamiliarSourceSettings>>;
}
use crate::content::native_gameplay::NativeGameplayState;
use crate::domain::CharacterId;
use crate::durability::character_build::BuildOccurrence;
use crate::durability::character_familiar::FamiliarStateOccurrence;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::spell_items_abi::SpellItemTransactionRequest;
use crate::foundation::{CommandId, CommandRef, GameSessionState, RuntimeScopeRefV1};
use crate::spell::companion_lifecycle::{
    FamiliarOwnerFacts, commit_familiar, familiar_cost_binding, finish_familiar_payment,
    prepare_familiar,
};
use crate::spell::delayed_execution::{
    CastBinding, ScheduleRequest, SpellTimerOccurrence, TimerPayload,
};
use crate::spell::mana_training::PreparedPlayerTraining;
use crate::spell::native_companions::{CompanionFacts, FamiliarEvent};
use crate::spell::{Execution, OperationalCastFacts};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use sha2::{Digest, Sha256};
fn nonce(tag: &[u8], actor: ExactActorRef, session: GameSessionId, command: u64) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(tag)
        .chain_update(actor.placement_identity())
        .chain_update(session.as_bytes())
        .chain_update(command.to_be_bytes())
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}
fn current_owner(
    runtime: &crate::foundation::ChannelRuntimeV1,
    fence: &CurrentCharacterGameplayFence,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    let b = runtime.binding();
    fence.game_session_id == session
        && fence.runtime_scope == RuntimeScopeRefV1::channel(b.world_id(), b.channel_id())
        && fence.scope_ownership_generation == b.scope_generation()
        && runtime
            .player_control_facts(actor, session)
            .is_ok_and(|v| v.control_loss.is_none())
}
fn definite_familiar_rejection(
    error: &crate::durability::character_progression::CharacterProgressionError,
) -> bool {
    // These typed semantic refusals return before the sole COMMIT boundary.
    // Database/deadline/commit errors carry Unavailable and remain ambiguous.
    !matches!(
        error,
        crate::durability::character_progression::CharacterProgressionError::Unavailable(_)
    )
}
fn logout_owner(
    runtime: &crate::foundation::ChannelRuntimeV1,
    fence: &CurrentCharacterGameplayFence,
    actor: ExactActorRef,
    session: GameSessionId,
    epoch: u64,
) -> bool {
    let binding = runtime.binding();
    fence.game_session_id == session
        && fence.runtime_scope
            == RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        && fence.scope_ownership_generation == binding.scope_generation()
        && runtime
            .player_control_facts(actor, session)
            .is_ok_and(|v| v.control_loss.is_some_and(|mark| mark.epoch == epoch))
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Source Login composition after the real first-entry owners have been installed.
    /// Reads the actual noncast admission/lifecycle owners with no manufactured cast
    /// command. Unknown commercial access stays unavailable.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn initialize_familiar_login(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        admission: &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
        content: &NativeGameplayState,
        access: &impl super::super::spell_access_facts::CurrentSpellAccessOwner,
        config: &crate::content::spell_familiar_config::CompiledFamiliarConfig,
    ) -> Result<Option<crate::spell::companion_lifecycle::LifecycleFamiliarApplyReceipt>, ()> {
        if config.source_digest() != content.source_digest() {
            return Err(());
        }
        let owned = self
            .read_familiar_lifecycle_inputs(actor, session, content, access)
            .await?;
        let vocation =
            crate::spell::Vocation::from_key(owned.durable_build().vocation()).ok_or(())?;
        let Some(index) = self.active_familiar_index(base_familiar_vocation(vocation)) else {
            return Ok(None);
        };
        let fence = self.familiar_fence(session).await.ok_or(())?;
        let group = {
            let runtime = self.runtime.lock().await;
            if !current_owner(&runtime, &fence, actor, session)
                || runtime.content_pin().server_artifact_digest() != content.source_digest()
            {
                return Err(());
            }
            self.root
                .initialize_familiar_group(self.character, self.holder, fence)
                .await
                .map_err(|_| ())?
        };
        let administrative = PersistedFamiliarGroupOwner::from_observed(group);
        let source = FamiliarSourceAdapter {
            config,
            premium: access,
            administrative: &administrative,
        };
        self.apply_familiar_owner_event(
            actor,
            session,
            index,
            FamiliarOwnerEvent::Login(admission),
            content,
            &owned,
            &source,
        )
        .await
        .map(Some)
    }
    /// One genuine full-fenced SQL producer followed by actual Channel/Player comparison.
    /// The returned data grants no current authority; the mutation independently checks it.
    pub(in crate::gameplay_transport) async fn read_familiar_lifecycle_inputs(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        content: &NativeGameplayState,
        access: &impl super::super::spell_access_facts::CurrentSpellAccessOwner,
    ) -> Result<OwnedCastFacts, ()> {
        let fence = self.familiar_fence(session).await.ok_or(())?;
        let raw = self
            .root
            .read_lifecycle_durable_facts(
                self.character,
                self.holder,
                &fence,
                content.source_digest(),
            )
            .await
            .map_err(|_| ())?;
        let runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        let state = states.get(&runtime, actor, session).ok_or(())?;
        if !current_owner(&runtime, &fence, actor, session)
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
            || raw.equipment.character_revision != fence.expected_character_revision.get()
            || access.account_id().is_some_and(|v| v != raw.account)
        {
            return Err(());
        }
        let binding = CastFactsBinding {
            actor,
            session,
            character: *fence.character_id.as_bytes(),
            character_revision: fence.expected_character_revision.get(),
            lease_generation: fence.character_lease_generation,
            connection_generation: fence.connection_generation.get(),
            player_revision: state.revision(),
            content_digest: content.source_digest(),
            equipment_revision: raw.equipment.revision,
        };
        let retry_access = FamiliarRetryAccess {
            owner: access,
            original_retained: states
                .pending_familiar_lifecycle
                .iter()
                .any(|p| p.actor == actor && p.session == session),
        };
        let projections = super::super::spell_access_facts::CurrentSpellAccessOwner::read_current(
            &retry_access,
            &binding,
            self.owner_now().get(),
        )
        .map_err(|_| ())?;
        OwnedCastFacts::from_owner_reads(binding, raw.build, raw.level, raw.equipment, projections)
            .map_err(|_| ())
    }
    pub(in crate::gameplay_transport) fn active_familiar_index(
        &self,
        vocation: &str,
    ) -> Option<NonZeroU32> {
        let mut raw = 1;
        loop {
            let index = NonZeroU32::new(raw)?;
            let (spell, active) = self.spells.source_indexed(index)?;
            if active
                && let Execution::NativeProfile(profile) = &spell.execution
                && profile.spell()["execution"]["native_behavior"]["key"] == "familiar_summon"
                && profile.spell()["execution"]["native_behavior"]["parameters"]["vocation"]
                    .as_str()
                    == Some(vocation)
            {
                return Some(index);
            }
            raw = raw.checked_add(1)?;
        }
    }
    /// Save paused reference-spell cooldowns at the real final actor-end boundary.
    /// The caller supplies the actual control-loss epoch; both live Channel mark
    /// and independently reread durable session must prove that same epoch.
    /// This method performs no periodic save, caster payment or training mutation.
    pub(in crate::gameplay_transport) async fn save_familiar_logout(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        loss_epoch: crate::foundation::ControlLossEpochRefV1,
    ) -> FamiliarLogoutSave {
        use crate::durability::character_familiar::FamiliarStateOutcome;
        let Some(content) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return FamiliarLogoutSave::NotApplicable;
        };
        let Some(profile) = self.first_active_familiar_profile() else {
            return FamiliarLogoutSave::NotApplicable;
        };
        let Some(fence) = self.familiar_logout_fence(session, loss_epoch.get()).await else {
            return FamiliarLogoutSave::FencedOut;
        };
        let Ok(Some(progression)) = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
        else {
            return FamiliarLogoutSave::Unknown;
        };
        let Ok(loaded) = self
            .root
            .read_character_familiar_state(self.character, fence.character_id)
            .await
        else {
            return FamiliarLogoutSave::Unknown;
        };
        let Ok(unix) = self.root.read_familiar_unix_time(self.character).await else {
            return FamiliarLogoutSave::Unknown;
        };
        let runtime = self.runtime.lock().await;
        let _measurement = FamiliarOwnerTurnMeasurement(std::time::Instant::now());
        let mut states = self.spell_states.lock().await;
        if !logout_owner(&runtime, &fence, actor, session, loss_epoch.get())
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
        {
            return FamiliarLogoutSave::FencedOut;
        }
        let prepared = if let Some(retained) = states
            .pending_familiar_logouts
            .iter()
            .find(|p| p.actor == actor && p.session == session)
            .cloned()
        {
            if retained.loss_epoch != loss_epoch.get()
                || states.get(&runtime, actor, session) != Some(&retained.before)
            {
                return FamiliarLogoutSave::Unknown;
            }
            retained
        } else {
            // A pending paid/lifecycle cast must resolve before actor-end can snapshot it.
            if states.has_pending_familiar(actor, session) {
                return FamiliarLogoutSave::Unknown;
            }
            let Some(before) = states.get(&runtime, actor, session) else {
                return FamiliarLogoutSave::NotApplicable;
            };
            let Ok(after) = crate::spell::companion_lifecycle::familiar_logout_snapshot(
                loaded.state(),
                before,
                profile,
                self.spells,
                self.owner_now(),
                unix,
            ) else {
                return FamiliarLogoutSave::Unknown;
            };
            if &after == loaded.state() {
                return FamiliarLogoutSave::NotApplicable;
            }
            let Ok(occurrence) = FamiliarStateOccurrence::from_bytes(nonce(
                b"oteryn:familiar-logout:v1",
                actor,
                session,
                loss_epoch.get(),
            )) else {
                return FamiliarLogoutSave::Unknown;
            };
            let prepared = PreparedFamiliarLogout {
                actor,
                session,
                loss_epoch: loss_epoch.get(),
                fence,
                request: FamiliarStateRequest {
                    occurrence,
                    before: loaded.state().clone(),
                    after,
                    content_revision: progression.context.content,
                    policy_revision: progression.policy_revision,
                    policy_digest: content.source_digest(),
                },
                before: before.clone(),
            };
            if states.pending_familiar_logouts.try_reserve(1).is_err() {
                return FamiliarLogoutSave::Unknown;
            }
            states.pending_familiar_logouts.push(prepared.clone());
            prepared
        };
        let committed = match self
            .root
            .commit_character_familiar_state(
                self.character,
                self.holder,
                prepared.fence,
                prepared.request.clone(),
            )
            .await
        {
            Ok(
                FamiliarStateOutcome::Committed(value)
                | FamiliarStateOutcome::AlreadyCommitted(value),
            ) => value,
            Err(_) => return FamiliarLogoutSave::Unknown,
        };
        if !committed.matches_request(&prepared.fence, &prepared.request) {
            return FamiliarLogoutSave::Unknown;
        }
        let Some(fresh) = self.familiar_logout_fence(session, loss_epoch.get()).await else {
            return FamiliarLogoutSave::FencedOut;
        };
        let mut expected = prepared.fence;
        expected.expected_character_revision = committed.committed_character_revision();
        if fresh != expected
            || !logout_owner(&runtime, &fresh, actor, session, loss_epoch.get())
            || states.get(&runtime, actor, session) != Some(&prepared.before)
        {
            return FamiliarLogoutSave::Unknown;
        }
        states.pending_familiar_logouts.retain(|v| {
            v.actor != actor || v.session != session || v.loss_epoch != loss_epoch.get()
        });
        FamiliarLogoutSave::Saved
    }
    fn first_active_familiar_profile(&self) -> Option<&crate::spell::native::CompiledNativeSpell> {
        let mut index = 1;
        loop {
            let (spell, active) = self.spells.source_indexed(NonZeroU32::new(index)?)?;
            if active
                && let Execution::NativeProfile(profile) = &spell.execution
                && profile.spell()["execution"]["native_behavior"]["key"] == "familiar_summon"
            {
                return Some(profile);
            }
            index = index.checked_add(1)?;
        }
    }
    async fn familiar_logout_fence(
        &self,
        session: GameSessionId,
        epoch: u64,
    ) -> Option<CurrentCharacterGameplayFence> {
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
            .ok()?;
        if current.session_state() == GameSessionState::Terminal
            || current.current_control_loss_epoch().map(|v| v.get()) != Some(epoch)
        {
            return None;
        }
        let character =
            CharacterId::from_bytes(*current.commit().character_id().as_bytes()).ok()?;
        let root = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        Some(CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session,
            connection_generation: current.current_connection_generation(),
            character_lease_generation: current.current_character_lease().generation(),
            runtime_scope: current.current_runtime_scope(),
            scope_ownership_generation: current.current_scope_generation(),
            expected_character_revision: root.revision,
        })
    }
    async fn familiar_fence(
        &self,
        session_id: GameSessionId,
    ) -> Option<CurrentCharacterGameplayFence> {
        let (session, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session_id)
            .await
            .ok()?;
        if session.session_state() != GameSessionState::Active {
            return None;
        }
        let character =
            CharacterId::from_bytes(*session.commit().character_id().as_bytes()).ok()?;
        let record = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        Some(CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: session_id,
            connection_generation: session.current_connection_generation(),
            character_lease_generation: session.current_character_lease().generation(),
            runtime_scope: session.current_runtime_scope(),
            scope_ownership_generation: session.current_scope_generation(),
            expected_character_revision: record.revision,
        })
    }
    /// Real Cast path. Its facts/config ports must be registered current owner reads. External
    /// Premium/VIP/God input cannot be invented by this adapter. Locks stay runtime→states across
    /// only bounded SQL passes, never an external API call; existing DB fences are rechecked in TX.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn cast_durable_familiar(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        content: &NativeGameplayState,
        owned: &OwnedCastFacts,
        source: &impl CurrentFamiliarSourceOwner,
    ) -> Option<SpellCastOutcome> {
        let (spell, active) = self.spells.source_indexed(intent.spell)?;
        let Execution::NativeProfile(profile) = &spell.execution else {
            return None;
        };
        if profile.spell()["execution"]["native_behavior"]["key"] != "familiar_summon" {
            return None;
        }
        if !active || intent.target != SpellTarget::None || intent.aim_at_target || command_id == 0
        {
            return Some(SpellCastOutcome::rejected());
        }
        let Some(fence) = self.familiar_fence(session).await else {
            return Some(SpellCastOutcome::rejected());
        };
        let Ok(Some(progression)) = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
        else {
            return Some(SpellCastOutcome::rejected());
        };
        let Ok(loaded) = self
            .root
            .read_character_familiar_state(self.character, fence.character_id)
            .await
        else {
            return Some(SpellCastOutcome::rejected());
        };
        let Ok(unix) = self.root.read_familiar_unix_time(self.character).await else {
            return Some(SpellCastOutcome::rejected());
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let _measurement = FamiliarOwnerTurnMeasurement(std::time::Instant::now());
        let mut states = self.spell_states.lock().await;
        if !current_owner(&runtime, &fence, actor, session)
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
            || owned.binding().actor != actor
            || owned.binding().session != session
            || owned.binding().character != *fence.character_id.as_bytes()
            || owned.binding().character_revision != fence.expected_character_revision.get()
            || owned.binding().lease_generation != fence.character_lease_generation
            || owned.binding().connection_generation != fence.connection_generation.get()
        {
            return Some(SpellCastOutcome::rejected());
        }
        let retained = states
            .pending_familiars
            .iter()
            .find(|p| p.actor == actor && p.session == session)
            .cloned();
        let (prepared, training) = if let Some(prepared) = retained {
            if prepared.command_id != command_id
                || prepared.intent != *intent
                || states.get(&runtime, actor, session) != Some(&prepared.before)
            {
                return Some(SpellCastOutcome::rejected());
            }
            let training = prepared.training.clone();
            (prepared, training)
        } else {
            let Some(state) = states.get(&runtime, actor, session) else {
                return Some(SpellCastOutcome::rejected());
            };
            let Ok(caster) = owned.caster(
                state,
                spell,
                content,
                crate::spell::harmony::HarmonyMultiplier::ONE,
                now.get(),
            ) else {
                return Some(SpellCastOutcome::rejected());
            };
            let Some(settings) = source
                .read_current(owned.binding(), now.get())
                .filter(|p| p.current(owned.binding(), now.get()))
            else {
                return Some(SpellCastOutcome::rejected());
            };
            if settings.value.benefit_use != FamiliarBenefitUse::CurrentEligibility {
                return Some(SpellCastOutcome::rejected());
            }
            let Ok(position) = runtime.read_actor_position(actor) else {
                return Some(SpellCastOutcome::rejected());
            };
            let pos = position.position();
            let cell = crate::spell::chain::TilePosition {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            };
            let raw = nonce(b"oteryn:familiar-cast:v1", actor, session, command_id);
            let revisions = crate::ability::RevisionSet::new(
                &progression.context.ruleset,
                &progression.context.content,
                &progression.policy_revision,
                "source:familiar-r20",
                &progression.context.simulation,
            )
            .ok()?;
            let occurrence_id: String = raw.iter().map(|byte| format!("{byte:02x}")).collect();
            let occurrence = crate::ability::AbilityOccurrence::new(
                &format!("familiar:{occurrence_id}"),
                revisions,
            )
            .ok()?;
            let binding = CastBinding {
                spell: profile.clone(),
                caster: actor,
                attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .ok()?,
                command: CommandRef::new(session, CommandId::new(command_id).ok()?),
                occurrence,
                parent_binding: serde_json::to_vec(profile.spell()).ok()?,
                cast_at: crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                cast_position: cell,
                cast_snapshot: None,
            };
            let value = &settings.value;
            if value.premium != caster.premium {
                return Some(SpellCastOutcome::rejected());
            }
            let Ok(equipment_speed) = owned.equipment_speed_bonus(content) else {
                return Some(SpellCastOutcome::rejected());
            };
            let facts = FamiliarOwnerFacts {
                vocation: base_familiar_vocation(caster.vocation).to_owned(),
                premium: caster.premium,
                level: caster.level,
                account_at_least_god: value.account_at_least_god,
                current_speed: i32::from(crate::spell::actor_conditions::movement_speed(
                    state,
                    now.get(),
                    equipment_speed,
                )),
                familiar_minutes: value.familiar_minutes,
                vip: value.vip,
                vip_reduction_minutes: value.vip_reduction_minutes,
                cooldown_rate: value.cooldown_rate,
                state: loaded.state().clone(),
            };
            let Ok(mut familiar) = prepare_familiar(
                &runtime,
                self.movement_cells,
                binding,
                FamiliarEvent::Cast,
                &facts,
                unix,
                None,
            ) else {
                return Some(SpellCastOutcome::rejected());
            };
            let operational = OperationalCastFacts {
                caster_position: cell,
                target_position: None,
                target: None,
                line_of_sight_clear: None,
                direction_available: false,
                wheel_unlocked: None,
                in_protection_zone: false,
                target_tile_solid: None,
                target_tile_creature: None,
            };
            let Ok(mut paid) = crate::spell::cast::prepare_native_owner_cast_with_caster(
                state,
                spell,
                &operational,
                crate::spell::native::Facts::Companion(&CompanionFacts::Familiar(
                    familiar.facts().clone(),
                )),
                now,
                &mut |_, _| 0,
                &caster,
            ) else {
                return Some(SpellCastOutcome::rejected());
            };
            let Ok(anchor) =
                finish_familiar_payment(state, &mut paid.next, &mut familiar, self.spells, now)
            else {
                return Some(SpellCastOutcome::rejected());
            };
            let digest: [u8; 32] = Sha256::digest(serde_json::to_vec(profile.spell()).ok()?).into();
            let request = familiar.state_request(
                FamiliarStateOccurrence::from_bytes(raw).ok()?,
                progression.context.content.clone(),
                progression.policy_revision.clone(),
                digest,
            )?;
            let Some(formula) = content.training_formula() else {
                return Some(SpellCastOutcome::rejected());
            };
            let Ok(training) = state.prepare_paid_training(
                &paid.next,
                &anchor,
                formula,
                BuildOccurrence::from_bytes(nonce(
                    b"oteryn:familiar-training:v1",
                    actor,
                    session,
                    command_id,
                ))
                .ok()?,
                now.get(),
            ) else {
                return Some(SpellCastOutcome::rejected());
            };
            let prepared = PreparedFamiliarCast {
                actor,
                session,
                command_id,
                intent: *intent,
                spell_index: intent.spell,
                fence,
                request,
                familiar,
                before: state.clone(),
                paid: paid.next,
                anchor,
                facts_binding: owned.binding().clone(),
                settings,
                training: training.clone(),
                touches: None,
                physical: None,
            };
            if states.pending_familiars.try_reserve(1).is_err() {
                return Some(SpellCastOutcome::rejected());
            }
            states.pending_familiars.push(prepared.clone());
            (prepared, training)
        };
        Some(
            self.finish_familiar_cast(
                &mut runtime,
                &mut states,
                prepared,
                training,
                content,
                source,
                owned.binding(),
            )
            .await,
        )
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    #[allow(
        clippy::too_many_arguments,
        reason = "the owner turn binds every independently resolved fact explicitly"
    )]
    async fn finish_familiar_cast(
        &self,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        mut prepared: PreparedFamiliarCast,
        mut training: PreparedPlayerTraining,
        content: &NativeGameplayState,
        source: &impl CurrentFamiliarSourceOwner,
        actual_binding: &CastFactsBinding,
    ) -> SpellCastOutcome {
        let now = self.owner_now();
        if states.get(runtime, prepared.actor, prepared.session) != Some(&prepared.before)
            || !current_owner(runtime, &prepared.fence, prepared.actor, prepared.session)
            || prepared
                .familiar
                .validate_current(runtime, prepared.familiar.owner_facts())
                .is_err()
        {
            return SpellCastOutcome::rejected();
        }
        let mut expected_binding = prepared.facts_binding.clone();
        expected_binding.character_revision = actual_binding.character_revision;
        if actual_binding != &expected_binding {
            return SpellCastOutcome::rejected();
        }
        let settings = source
            .read_current(actual_binding, now.get())
            .filter(|v| v.current(actual_binding, now.get()));
        // Eligibility is required for a NEW mutation. A genuine already-paid original
        // completes under current actor/session/lease/scope/Content without a second grant.
        let allow_new_mutation = settings.as_ref().is_some_and(|settings| {
            settings.value == prepared.settings.value
                && settings.authority_revision == prepared.settings.authority_revision
        });
        let binding = prepared.familiar.binding();
        let mut batch = crate::spell::combat_batch::OwnerCombatBatch {
            caster: prepared.actor,
            attacker: binding.attacker,
            current_lease_generation: prepared.fence.character_lease_generation,
            command: binding.command,
            occurrence: binding.occurrence.clone().into(),
            binding: binding.parent_binding.clone(),
            anchor: Some(prepared.anchor.clone()),
            now_ms: binding.cast_at.get() / 1000,
            effects: Vec::new(),
            deferred: None,
        };
        let touches = match prepared.touches.as_ref() {
            Some(touches) => {
                let Some(physical) = prepared.physical.as_ref() else {
                    return SpellCastOutcome::rejected();
                };
                batch = physical.batch().clone();
                touches.clone()
            }
            None => match crate::foundation::QualifiedCompanionTouches::bind(
                &mut batch,
                prepared.familiar.reservation_snapshots(),
            ) {
                Ok(touches) => touches,
                Err(_) => return SpellCastOutcome::rejected(),
            },
        };
        // Reserve all physical batch/timer capacity while these same owners remain held.
        // The genuine training receipt can update only the paid successor after COMMIT.
        let Ok(mut base_preflight) =
            super::stage_player_batch(runtime, states, &batch, Some(prepared.paid.clone()))
        else {
            return SpellCastOutcome::rejected();
        };
        if base_preflight.validate_current(runtime, states).is_err() {
            return SpellCastOutcome::rejected();
        }
        let Ok(mut staged) = runtime.stage_spell_batch_with_companion_touches(&batch, &touches)
        else {
            return SpellCastOutcome::rejected();
        };
        if !staged.will_apply() {
            return SpellCastOutcome::rejected();
        }
        let Ok(stamp) = runtime.issue_owner_work() else {
            return SpellCastOutcome::rejected();
        };
        let Ok(schedules) = prepared.familiar.schedules(binding.cast_at, 0) else {
            return SpellCastOutcome::rejected();
        };
        let requests = schedules
            .into_iter()
            .map(|schedule| ScheduleRequest {
                occurrence: SpellTimerOccurrence {
                    command: binding.command,
                    phase: schedule.phase,
                },
                due: schedule.due,
                payload: TimerPayload::Familiar(schedule.saved),
            })
            .collect();
        let Some(timers) = states.spell_timers.as_ref() else {
            return SpellCastOutcome::rejected();
        };
        let Ok(owner_fence) = runtime.owner_fence() else {
            return SpellCastOutcome::rejected();
        };
        let Ok(reservation) = timers.schedule_reservation(owner_fence, stamp, requests) else {
            return SpellCastOutcome::rejected();
        };
        let Ok(timer_preflight) =
            timers.preflight_familiar_install(owner_fence, stamp, reservation, &prepared.familiar)
        else {
            return SpellCastOutcome::rejected();
        };
        let profile = binding.spell.spell();
        let (Some(key), Some(revision)) = (
            profile["identity"]["key"].as_str(),
            profile["identity"]["revision"].as_str(),
        ) else {
            return SpellCastOutcome::rejected();
        };
        let Ok(cost_binding) =
            familiar_cost_binding(&prepared.before, &prepared.paid, &prepared.anchor)
        else {
            return SpellCastOutcome::rejected();
        };
        let cost = SpellItemTransactionRequest {
            command: binding.command,
            spell: TypedDefinitionRef {
                family: "Spell".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            },
            catalog_digest: content.source_digest(),
            transaction_id: nonce(
                b"oteryn:familiar-cost-tx:v1",
                prepared.actor,
                prepared.session,
                prepared.command_id,
            ),
            event_id: nonce(
                b"oteryn:familiar-cost-event:v1",
                prepared.actor,
                prepared.session,
                prepared.command_id,
            ),
            cost: cost_binding,
            caster_origin: None,
            operations: Vec::new(),
            companion: None,
            direct_companion: None,
        };
        let Some(formula) = content.training_formula() else {
            return SpellCastOutcome::rejected();
        };
        let training_request = training
            .request()
            .cloned()
            .map(|request| (request, formula.clone()));
        // Allocate the successor and immutable comparison data before the durable write.
        let mut paid = prepared.paid.clone();
        let facts = prepared.familiar.owner_facts().clone();
        if let Some(original) = prepared.physical.as_ref() {
            if runtime.validate_staged_spell_batch(original).is_err() {
                return SpellCastOutcome::rejected();
            }
        } else if prepared.familiar.reserve_physical(runtime).is_err() {
            return SpellCastOutcome::rejected();
        }
        if runtime.reserve_spell_batch(&mut staged).is_err() {
            // No SQL has started. A matching new fixed-slot reservation may be returned.
            let _ = prepared.familiar.rollback_physical(runtime);
            return SpellCastOutcome::rejected();
        }
        // Retain the exact real touched-slot seal BEFORE the first committing await.
        // These clones allocate only before SQL; the physical pending_owner data lives
        // in the actual caster and companion slots, and dropping this data releases none.
        prepared.touches = Some(touches);
        prepared.physical = Some(staged.clone());
        if let Some(original) = states.pending_familiars.iter_mut().find(|value| {
            value.actor == prepared.actor
                && value.session == prepared.session
                && value.command_id == prepared.command_id
        }) {
            *original = prepared.clone();
        } else {
            return SpellCastOutcome::rejected();
        }
        // This method captures only owned recovery/node/request/formula data in its SQL pass.
        // No Channel guard is captured by that callback; locks keep the real fixed slots stable.
        let commit_result = if allow_new_mutation {
            self.root
                .commit_familiar_spell(
                    self.character,
                    self.holder,
                    prepared.fence,
                    prepared.request.clone(),
                    cost,
                    training_request,
                )
                .await
        } else {
            self.root
                .reconcile_familiar_spell(
                    self.character,
                    self.holder,
                    prepared.fence,
                    prepared.request.clone(),
                    cost,
                    training_request,
                )
                .await
        };
        let committed = match commit_result {
            Ok(value) => value,
            Err(error) => {
                if definite_familiar_rejection(&error)
                    && prepared.familiar.rollback_physical(runtime).is_ok()
                    && runtime
                        .release_definitely_uncommitted_spell_batch(&staged)
                        .is_ok()
                {
                    states.pending_familiars.retain(|v| {
                        v.actor != prepared.actor
                            || v.session != prepared.session
                            || v.command_id != prepared.command_id
                    });
                }
                return SpellCastOutcome::rejected();
            }
        };
        let Some(receipt) = committed.familiar() else {
            return SpellCastOutcome::rejected();
        };
        if committed.cost().is_none()
            || !receipt.matches_request(&prepared.fence, &prepared.request)
        {
            return SpellCastOutcome::rejected();
        }
        let expected_revision = committed
            .training()
            .map(|value| value.committed_character_revision)
            .unwrap_or_else(|| receipt.committed_character_revision());
        // History never renews authority. Re-read the independently current admission/root fence.
        let Some(fresh_fence) = self.familiar_fence(prepared.session).await else {
            return SpellCastOutcome::rejected();
        };
        let mut expected_fence = prepared.fence;
        expected_fence.expected_character_revision = expected_revision;
        if fresh_fence != expected_fence
            || !current_owner(runtime, &fresh_fence, prepared.actor, prepared.session)
            || states.get(runtime, prepared.actor, prepared.session) != Some(&prepared.before)
        {
            return SpellCastOutcome::rejected();
        }
        // The genuine source COMMIT already accepted this exact original occurrence.
        // Commercial expiry now cannot retract its paid successor. Fresh full durable
        // and physical fences above still gate installation independently from history.
        if training
            .prepare_install(
                &prepared.before,
                &mut paid,
                &prepared.anchor,
                committed.training(),
            )
            .is_err()
        {
            return SpellCastOutcome::rejected();
        }
        if base_preflight
            .rebind_training(runtime, states, paid)
            .is_err()
            || base_preflight.validate_current(runtime, states).is_err()
            || prepared
                .familiar
                .validate_current(runtime, prepared.familiar.owner_facts())
                .is_err()
        {
            return SpellCastOutcome::rejected();
        }
        if runtime
            .release_companion_touches_for_source_commit(&mut staged)
            .is_err()
        {
            return SpellCastOutcome::rejected();
        }
        let Ok(applied) = commit_familiar(
            runtime,
            prepared.familiar,
            &facts,
            &prepared.fence,
            Some(&prepared.request),
            Some(receipt),
        ) else {
            return SpellCastOutcome::rejected();
        };
        // All timer payloads were source-matched to this exact preparation before SQL.
        let validated_timer = timer_preflight.finalize(&applied).expect(
            "unchanged source-qualified familiar preparation must match its actual apply receipt",
        );
        super::commit_owner_batch(runtime, states, staged, Some(base_preflight))
            .expect("exclusive familiar owner turn preserves the preflighted caster batch");
        states
            .spell_timers
            .as_mut()
            .expect("held timer owner exists")
            .install_familiar_preflighted(validated_timer);
        states.pending_familiars.retain(|value| {
            value.actor != prepared.actor
                || value.session != prepared.session
                || value.command_id != prepared.command_id
        });
        let next = states
            .get(runtime, prepared.actor, prepared.session)
            .expect("same owner contains committed familiar caster");

        SpellCastOutcome {
            disposition: SpellCastDisposition::Cast,
            vitals: Some((next.revision(), next.vitals())),
        }
    }
}

/// Administrative false is explicit current ordinary-group data. Missing group/God ownership
/// remains Unknown; no inferred player type or unauthenticated client flag supplies this port.
pub(crate) trait CurrentFamiliarAdministrativeOwner:
    source_registration::Registered
{
    fn read_current_group(
        &self,
        binding: &CastFactsBinding,
        now_micros: u64,
    ) -> Option<CurrentProjection<bool>>;
}
/// Uses the same actual Premium producer as cast eligibility. Canary Player::isVip is the
/// qualified config flag AND current Premium, so this adapter creates no independent VIP grant.
pub(crate) struct FamiliarSourceAdapter<'a, P, A> {
    pub(crate) config: &'a crate::content::spell_familiar_config::CompiledFamiliarConfig,
    pub(crate) premium: &'a P,
    pub(crate) administrative: &'a A,
}
impl<P, A> source_registration::Registered for FamiliarSourceAdapter<'_, P, A> {}
impl<
    P: super::super::spell_access_facts::CurrentSpellAccessOwner,
    A: CurrentFamiliarAdministrativeOwner,
> CurrentFamiliarSourceOwner for FamiliarSourceAdapter<'_, P, A>
{
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        now: u64,
    ) -> Option<CurrentProjection<FamiliarSourceSettings>> {
        if self.config.source_digest() != binding.content_digest {
            return None;
        }
        let premium = self.premium.read_current(binding, now).ok()?.premium?;
        let group = self.administrative.read_current_group(binding, now)?;
        if !premium.current(binding, now) || !group.current(binding, now) {
            return None;
        }
        Some(CurrentProjection {
            binding: binding.clone(),
            authority_revision: group.authority_revision,
            valid_until_micros: premium.valid_until_micros.min(group.valid_until_micros),
            value: FamiliarSourceSettings {
                benefit_use: FamiliarBenefitUse::CurrentEligibility,
                account_at_least_god: group.value,
                premium: premium.value,
                premium_revision: premium.authority_revision,
                familiar_minutes: self.config.familiar_minutes(),
                vip: self.config.vip_enabled() && premium.value,
                vip_reduction_minutes: self.config.vip_reduction_minutes(),
                cooldown_rate: self.config.cooldown_rate(),
            },
        })
    }
}

/// Retained actual owner-event intent. It contains immutable preparation/snapshots only,
/// never admission authority or a second creature owner; a retry revalidates current owners.
#[derive(Debug, Clone)]
pub(crate) struct PreparedFamiliarLifecycle {
    actor: ExactActorRef,
    session: GameSessionId,
    spell_index: NonZeroU32,
    fence: CurrentCharacterGameplayFence,
    request: Option<FamiliarStateRequest>,
    familiar: crate::spell::companion_lifecycle::PreparedLifecycleFamiliar,
    before: PlayerSpellState,
    login_successor: Option<PlayerSpellState>,
    facts_binding: CastFactsBinding,
    settings: CurrentProjection<FamiliarSourceSettings>,
    affected_actors: Vec<ExactActorRef>,
    source_event_id: [u8; 16],
    source_binding_digest: [u8; 32],
    reservation: Option<crate::foundation::SourceActorReservation>,
}
impl crate::foundation::source_reservation_seal::Sealed for PreparedFamiliarLifecycle {}
impl crate::foundation::SourceActorReservationProof for PreparedFamiliarLifecycle {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.actor, self.session)
    }
    fn event_identity(&self) -> [u8; 16] {
        self.source_event_id
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.source_binding_digest
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &self.affected_actors
    }
    fn current_for(&self, runtime: &crate::foundation::ChannelRuntimeV1) -> bool {
        current_owner(runtime, &self.fence, self.actor, self.session)
            && runtime.content_pin().server_artifact_digest() == self.facts_binding.content_digest
            && runtime
                .owner_fence()
                .is_ok_and(|f| f.accepts_stamp(self.familiar.binding().stamp()))
            && self
                .familiar
                .validate_current(runtime, self.familiar.owner_facts())
                .is_ok()
    }
}

/// Only server owner callbacks pass these actual source results. None is a wire command.
pub(crate) enum FamiliarOwnerEvent<'a> {
    Login(&'a crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1),
    Advance(&'a crate::durability::character_progression::CommittedExperienceAward),
    Death(&'a crate::foundation::RuntimeCorpseProjection),
}
fn matches_owner_event(
    binding: &crate::spell::companion_lifecycle::FamiliarLifecycleBinding,
    event: &FamiliarOwnerEvent<'_>,
) -> bool {
    use crate::spell::companion_lifecycle::FamiliarLifecycleOrigin as Origin;
    match (binding.origin(), event) {
        (Origin::Login { session }, FamiliarOwnerEvent::Login(value)) => {
            *session == value.binding().candidate_session
        }
        (
            Origin::Advance {
                occurrence,
                committed_character_revision,
                ..
            },
            FamiliarOwnerEvent::Advance(value),
        ) => {
            *occurrence == value.occurrence
                && *committed_character_revision == value.committed_character_revision.get()
        }
        (Origin::Death { occurrence }, FamiliarOwnerEvent::Death(value)) => {
            occurrence == value.occurrence()
        }
        _ => false,
    }
}
fn lifecycle_nonce(
    binding: &crate::spell::companion_lifecycle::FamiliarLifecycleBinding,
) -> [u8; 16] {
    use crate::spell::companion_lifecycle::FamiliarLifecycleOrigin as Origin;
    let mut hash = Sha256::new()
        .chain_update(b"oteryn:familiar-owner-event:v1")
        .chain_update(binding.owner().placement_identity())
        .chain_update(binding.session().as_bytes());
    match binding.origin() {
        Origin::Login { session } => {
            hash.update([0]);
            hash.update(session.as_bytes());
        }
        Origin::Advance {
            occurrence,
            committed_character_revision,
            ..
        } => {
            hash.update([1]);
            hash.update(occurrence.as_bytes());
            hash.update(committed_character_revision.to_be_bytes());
        }
        Origin::Death { occurrence } => {
            hash.update([2]);
            hash.update(occurrence.actor().placement_identity());
            hash.update(occurrence.commit_binding());
        }
    }
    hash.update(
        binding.spell().spell()["identity"]["key"]
            .as_str()
            .unwrap_or("")
            .as_bytes(),
    );
    let value = hash.finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&value[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Actual Login/XP/physical lethal callback composition. The root invokes this at those
    /// committed source sites, with its current account/config/equipment producers. Historical
    /// source identity and durable receipts never replace independently current owner checks.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn apply_familiar_owner_event(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        index: NonZeroU32,
        event: FamiliarOwnerEvent<'_>,
        content: &NativeGameplayState,
        owned: &OwnedCastFacts,
        source: &impl CurrentFamiliarSourceOwner,
    ) -> Result<crate::spell::companion_lifecycle::LifecycleFamiliarApplyReceipt, ()> {
        use crate::spell::companion_lifecycle::{
            FamiliarLifecycleBinding, prepare_lifecycle_familiar, restore_familiar_cooldown,
        };
        let (spell, true) = self.spells.source_indexed(index).ok_or(())? else {
            return Err(());
        };
        let Execution::NativeProfile(profile) = &spell.execution else {
            return Err(());
        };
        if profile.spell()["execution"]["native_behavior"]["key"] != "familiar_summon" {
            return Err(());
        }
        let fence = self.familiar_fence(session).await.ok_or(())?;
        let progression = self
            .root
            .read_character_progression(self.character, fence.character_id)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        let loaded = self
            .root
            .read_character_familiar_state(self.character, fence.character_id)
            .await
            .map_err(|_| ())?;
        let unix = self
            .root
            .read_familiar_unix_time(self.character)
            .await
            .map_err(|_| ())?;
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let _measurement = FamiliarOwnerTurnMeasurement(std::time::Instant::now());
        let mut states = self.spell_states.lock().await;
        if !current_owner(&runtime, &fence, actor, session)
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
            || owned.binding().actor != actor
            || owned.binding().session != session
            || owned.binding().character != *fence.character_id.as_bytes()
            || owned.binding().character_revision != fence.expected_character_revision.get()
            || owned.binding().lease_generation != fence.character_lease_generation
            || owned.binding().connection_generation != fence.connection_generation.get()
        {
            return Err(());
        }
        let retained = states
            .pending_familiar_lifecycle
            .iter()
            .find(|p| p.actor == actor && p.session == session)
            .cloned();
        let prepared = if let Some(prepared) = retained {
            if prepared.spell_index != index
                || !matches_owner_event(prepared.familiar.binding(), &event)
                || states.get(&runtime, actor, session) != Some(&prepared.before)
            {
                return Err(());
            }
            prepared
        } else {
            let state = states.get(&runtime, actor, session).ok_or(())?;
            let is_death = matches!(event, FamiliarOwnerEvent::Death(_));
            let caster = if is_death {
                None
            } else {
                Some(
                    owned
                        .caster(
                            state,
                            spell,
                            content,
                            crate::spell::harmony::HarmonyMultiplier::ONE,
                            now.get(),
                        )
                        .map_err(|_| ())?,
                )
            };
            let settings = source
                .read_current(owned.binding(), now.get())
                .filter(|v| v.current(owned.binding(), now.get()))
                .ok_or(())?;
            if (is_death && settings.value.benefit_use != FamiliarBenefitUse::NotApplicableDeath)
                || (!is_death
                    && settings.value.benefit_use != FamiliarBenefitUse::CurrentEligibility)
                || caster
                    .as_ref()
                    .is_some_and(|c| settings.value.premium != c.premium)
            {
                return Err(());
            }
            let equipment_speed = if is_death {
                0
            } else {
                owned.equipment_speed_bonus(content).map_err(|_| ())?
            };
            let vocation =
                crate::spell::Vocation::from_key(owned.durable_build().vocation()).ok_or(())?;
            let facts = FamiliarOwnerFacts {
                vocation: base_familiar_vocation(vocation).to_owned(),
                // The explicit NotApplicableDeath mode reaches only the source's Death
                // branch, which never reads benefit/level/speed/cooldown inputs. These
                // representation values grant no benefit; Cast/Login/Advance reject it.
                premium: caster.as_ref().is_some_and(|c| c.premium),
                level: caster.as_ref().map_or(0, |c| c.level),
                account_at_least_god: settings.value.account_at_least_god,
                current_speed: i32::from(crate::spell::actor_conditions::movement_speed(
                    state,
                    now.get(),
                    equipment_speed,
                )),
                familiar_minutes: settings.value.familiar_minutes,
                vip: settings.value.vip,
                vip_reduction_minutes: settings.value.vip_reduction_minutes,
                cooldown_rate: settings.value.cooldown_rate,
                state: loaded.state().clone(),
            };
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            let at = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get());
            let binding = match &event {
                FamiliarOwnerEvent::Login(value) => FamiliarLifecycleBinding::from_admission(
                    &runtime,
                    profile.clone(),
                    actor,
                    fence,
                    stamp,
                    at,
                    value,
                ),
                FamiliarOwnerEvent::Advance(value) => FamiliarLifecycleBinding::from_level_advance(
                    &runtime,
                    profile.clone(),
                    actor,
                    fence,
                    stamp,
                    at,
                    value,
                ),
                FamiliarOwnerEvent::Death(value) => {
                    FamiliarLifecycleBinding::from_committed_lethal(
                        &runtime,
                        profile.clone(),
                        actor,
                        fence,
                        stamp,
                        at,
                        value,
                    )
                }
            }
            .map_err(|_| ())?;
            let familiar = prepare_lifecycle_familiar(
                &runtime,
                Some(self.movement_cells),
                binding,
                &facts,
                unix,
            )
            .map_err(|_| ())?;
            let digest: [u8; 32] =
                Sha256::digest(serde_json::to_vec(profile.spell()).map_err(|_| ())?).into();
            let request = familiar.state_request(
                FamiliarStateOccurrence::from_bytes(lifecycle_nonce(familiar.binding()))
                    .map_err(|_| ())?,
                progression.context.content,
                progression.policy_revision,
                digest,
            );
            let login_successor = if matches!(event, FamiliarOwnerEvent::Login(_)) {
                let mut next = state.clone();
                restore_familiar_cooldown(&mut next, &familiar, self.spells, now)
                    .map_err(|_| ())?;
                if &next != state {
                    next.advance_batch_revision().map_err(|_| ())?;
                    Some(next)
                } else {
                    None
                }
            } else {
                None
            };
            let source_event_id = lifecycle_nonce(familiar.binding());
            // Private source event was issued from an actual admission/XP/lethal receipt.
            // Freeze its complete immutable binding before SQL, never rehash after COMMIT.
            let source_binding_digest: [u8; 32] = Sha256::new()
                .chain_update(b"oteryn:familiar-lifecycle-physical-binding:v1")
                .chain_update(format!("{:?}", familiar.binding()).as_bytes())
                .chain_update(format!("{facts:?}").as_bytes())
                .finalize()
                .into();
            let mut affected_actors: Vec<_> = familiar.reservation_actors().collect();
            affected_actors.sort_unstable_by_key(|actor| actor.placement_identity());
            affected_actors.dedup();
            let prepared = PreparedFamiliarLifecycle {
                actor,
                session,
                spell_index: index,
                fence,
                request,
                familiar,
                before: state.clone(),
                login_successor,
                facts_binding: owned.binding().clone(),
                settings,
                affected_actors,
                source_event_id,
                source_binding_digest,
                reservation: None,
            };
            states
                .pending_familiar_lifecycle
                .try_reserve(1)
                .map_err(|_| ())?;
            states.pending_familiar_lifecycle.push(prepared.clone());
            prepared
        };
        self.finish_familiar_lifecycle(&mut runtime, &mut states, prepared, owned.binding(), source)
            .await
    }

    #[allow(
        clippy::expect_used,
        reason = "post-validation commit invariant; a fallible exit here would leave a partial owner write"
    )]
    async fn finish_familiar_lifecycle(
        &self,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        mut prepared: PreparedFamiliarLifecycle,
        actual_binding: &CastFactsBinding,
        source: &impl CurrentFamiliarSourceOwner,
    ) -> Result<crate::spell::companion_lifecycle::LifecycleFamiliarApplyReceipt, ()> {
        use crate::durability::character_familiar::FamiliarStateOutcome;
        use crate::spell::companion_lifecycle::commit_lifecycle_familiar;
        use crate::spell::delayed_execution::LifecycleScheduleRequest;
        let actor = prepared.actor;
        let session = prepared.session;
        let index = prepared.spell_index;
        let now = self.owner_now();
        if states.get(runtime, actor, session) != Some(&prepared.before)
            || !current_owner(runtime, &prepared.fence, actor, session)
        {
            return Err(());
        }
        let mut expected_binding = prepared.facts_binding.clone();
        expected_binding.character_revision = actual_binding.character_revision;
        if actual_binding != &expected_binding {
            return Err(());
        }
        let current = source
            .read_current(actual_binding, now.get())
            .filter(|v| v.current(actual_binding, now.get()));
        let allow_new_mutation = current.as_ref().is_some_and(|v| {
            v.value == prepared.settings.value
                && v.authority_revision == prepared.settings.authority_revision
        });
        if prepared.request.is_none() && !allow_new_mutation {
            return Err(());
        }
        prepared
            .familiar
            .validate_current(runtime, prepared.familiar.owner_facts())
            .map_err(|_| ())?;
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        let timers = states.spell_timers.as_ref().ok_or(())?;
        let requests = prepared
            .familiar
            .schedules(0)
            .map_err(|_| ())?
            .into_iter()
            .map(|v| LifecycleScheduleRequest {
                phase: v.phase,
                due: v.due,
                saved: v.saved,
            })
            .collect();
        let owner_fence = runtime.owner_fence().map_err(|_| ())?;
        let reservation = timers
            .schedule_lifecycle_reservation(owner_fence, stamp, requests)
            .map_err(|_| ())?;
        let timer_preflight = timers
            .preflight_lifecycle_install(owner_fence, reservation, &prepared.familiar)
            .map_err(|_| ())?;
        let facts = prepared.familiar.owner_facts().clone();
        let mut physical_reservation = if let Some(original) = prepared.reservation.as_ref() {
            runtime
                .validate_source_actor_reservation(original, &prepared)
                .map_err(|_| ())?;
            original.clone()
        } else {
            prepared
                .familiar
                .reserve_physical(runtime)
                .map_err(|_| ())?;
            runtime
                .prepare_source_actor_reservation(&prepared)
                .map_err(|_| ())?
        };
        runtime
            .reserve_source_actors(&mut physical_reservation, &prepared)
            .map_err(|_| ())?;
        prepared.reservation = Some(physical_reservation.clone());
        let original = states
            .pending_familiar_lifecycle
            .iter_mut()
            .find(|p| p.actor == actor && p.session == session)
            .ok_or(())?;
        *original = prepared.clone();
        let committed = if let Some(request) = &prepared.request {
            let result = if allow_new_mutation {
                self.root
                    .commit_character_familiar_state(
                        self.character,
                        self.holder,
                        prepared.fence,
                        request.clone(),
                    )
                    .await
            } else {
                self.root
                    .reconcile_familiar_owner_state(
                        self.character,
                        self.holder,
                        prepared.fence,
                        request.clone(),
                    )
                    .await
            };
            let outcome = match result {
                Ok(value) => value,
                Err(error) => {
                    if definite_familiar_rejection(&error)
                        && runtime
                            .release_source_actors_after_observed_outcome(
                                physical_reservation,
                                &prepared,
                            )
                            .is_ok()
                        && prepared.familiar.rollback_physical(runtime).is_ok()
                    {
                        states.pending_familiar_lifecycle.retain(|v| {
                            v.actor != actor || v.session != session || v.spell_index != index
                        });
                    }
                    return Err(());
                }
            };
            Some(match outcome {
                FamiliarStateOutcome::Committed(value)
                | FamiliarStateOutcome::AlreadyCommitted(value) => value,
            })
        } else {
            None
        };
        let fresh = self.familiar_fence(session).await.ok_or(())?;
        let mut expected = prepared.fence;
        if let Some(receipt) = &committed {
            expected.expected_character_revision = receipt.committed_character_revision();
        }
        if fresh != expected
            || !current_owner(runtime, &fresh, actor, session)
            || states.get(runtime, actor, session) != Some(&prepared.before)
        {
            return Err(());
        }
        prepared
            .familiar
            .validate_current(runtime, prepared.familiar.owner_facts())
            .map_err(|_| ())?;
        let timers = states.spell_timers.as_ref().ok_or(())?;
        timer_preflight
            .validate_current(timers, runtime.owner_fence().map_err(|_| ())?)
            .map_err(|_| ())?;
        // Genuine durable outcome/current successor is qualified above. The same held
        // owner turn now consumes exactly the original commandless reservations before
        // source party/despawn/player installation; no allocation or await follows.
        runtime
            .release_source_actors_after_observed_outcome(physical_reservation, &prepared)
            .map_err(|_| ())?;
        let applied = commit_lifecycle_familiar(
            runtime,
            prepared.familiar,
            &facts,
            &fresh,
            prepared.request.as_ref(),
            committed.as_ref(),
        )
        .map_err(|_| ())?;
        let timers = timer_preflight
            .finalize(&applied)
            .expect("same preflighted lifecycle must match actual physical receipt");
        if let Some(next) = prepared.login_successor {
            assert!(
                states.commit(runtime, actor, session, next),
                "held Login owner preserves exact preflighted successor"
            );
        }
        states
            .spell_timers
            .as_mut()
            .expect("held lifecycle timer owner exists")
            .install_lifecycle_preflighted(timers);
        states
            .pending_familiar_lifecycle
            .retain(|p| p.actor != actor || p.session != session || p.spell_index != index);
        Ok(applied)
    }
}

fn base_familiar_vocation(vocation: crate::spell::Vocation) -> &'static str {
    use crate::spell::Vocation::*;
    match vocation {
        Druid | ElderDruid => "druid",
        Sorcerer | MasterSorcerer => "sorcerer",
        Knight | EliteKnight => "knight",
        Paladin | RoyalPaladin => "paladin",
        Monk | ExaltedMonk => "monk",
    }
}

/// Immutable explicit ordinary Game-group observation, minted only by the fenced persisted
/// group's actual committed initializer/read. Root separately checks current actor/session/
/// scope/Character at every mutation; this projection grants no admission authority.
pub(crate) struct PersistedFamiliarGroupOwner {
    snapshot: crate::durability::spell_familiar_group::FamiliarGroupSnapshot,
}
impl PersistedFamiliarGroupOwner {
    pub(crate) fn from_observed(
        snapshot: crate::durability::spell_familiar_group::FamiliarGroupSnapshot,
    ) -> Self {
        Self { snapshot }
    }
}
impl source_registration::Registered for PersistedFamiliarGroupOwner {}
impl CurrentFamiliarAdministrativeOwner for PersistedFamiliarGroupOwner {
    fn read_current_group(
        &self,
        binding: &CastFactsBinding,
        _: u64,
    ) -> Option<CurrentProjection<bool>> {
        if binding.character != *self.snapshot.character().as_bytes() {
            return None;
        }
        Some(CurrentProjection {
            binding: binding.clone(),
            authority_revision: self.snapshot.revision(),
            valid_until_micros: u64::MAX,
            value: self.snapshot.account_at_least_god(),
        })
    }
}
