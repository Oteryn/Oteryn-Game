//! Source-qualified transport dispatch and retries of the original familiar owner intent.
//! This child uses the existing Channel owners and registered access producer.
use super::super::super::spell_access_facts::{
    CurrentSpellAccessOwner, qualify_raw_owned_cast_facts,
};
use super::super::NativeCastDispatch;
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;

fn familiar_spell(spell: &crate::spell::SpellDefinition) -> bool {
    matches!(&spell.execution, Execution::NativeProfile(profile)
        if profile.spell()["execution"]["native_behavior"]["key"] == "familiar_summon")
}
/// Upstream FamiliarDeath has no commercial/config benefit reads. This private producer
/// is usable only with the actual Death receipt; its explicit N/A mode is refused by casts
/// and other lifecycle kinds. It neither grants nor denies an unknown Premium capability.
struct DeathCleanupSource {
    actor: ExactActorRef,
    session: GameSessionId,
    digest: [u8; 32],
}
impl source_registration::Registered for DeathCleanupSource {}
impl CurrentFamiliarSourceOwner for DeathCleanupSource {
    fn read_current(
        &self,
        binding: &CastFactsBinding,
        _: u64,
    ) -> Option<CurrentProjection<FamiliarSourceSettings>> {
        if binding.actor != self.actor
            || binding.session != self.session
            || binding.content_digest != self.digest
        {
            return None;
        }
        Some(CurrentProjection {
            binding: binding.clone(),
            authority_revision: binding.character_revision,
            valid_until_micros: u64::MAX,
            value: FamiliarSourceSettings {
                benefit_use: FamiliarBenefitUse::NotApplicableDeath,
                premium: false,
                premium_revision: 0,
                account_at_least_god: false,
                familiar_minutes: 0,
                vip: false,
                vip_reduction_minutes: 0,
                cooldown_rate: 0.0,
            },
        })
    }
}

fn matches_replayed_familiar_profile(
    binding: &[u8],
    profile: &crate::spell::native::CompiledNativeSpell,
) -> bool {
    let Ok(mut saved) = serde_json::from_slice::<serde_json::Value>(binding) else {
        return false;
    };
    let Some(object) = saved.as_object_mut() else {
        return false;
    };
    if object
        .get("source_companion_touches")
        .is_some_and(|v| !v.is_string())
    {
        return false;
    }
    object.remove("source_companion_touches");
    &saved == profile.spell()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Periodic real owner death issuer. The current physical health-zero slot yields
    /// an opaque Combat lethal receipt, then the actual Combat projection; immutable
    /// source history is still independently current-qualified by the lifecycle writer.
    pub(in crate::gameplay_transport) async fn drain_familiar_deaths<
        A: CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &A,
    ) -> Result<usize, ()> {
        let deaths = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            if states.has_pending_familiar(actor, session) {
                return Err(());
            }
            runtime
                .dead_owned_familiars(actor, session)
                .map_err(|_| ())?
        };
        let mut completed = 0;
        for dead in deaths {
            let projection = {
                let mut runtime = self.runtime.lock().await;
                let snapshot = runtime
                    .companion_snapshot_including_dead(dead)
                    .map_err(|_| ())?;
                if snapshot.health != 0
                    || snapshot.state.master
                        != Some(crate::foundation::CompanionMaster { actor, session })
                    || !snapshot.state.policy.is_familiar
                {
                    return Err(());
                }
                runtime
                    .assert_actor_spell_unreserved(dead)
                    .map_err(|_| ())?;
                let mut combat = runtime.borrow_combat_death();
                let lethal = combat.committed_lethal_receipt(dead).map_err(|_| ())?;
                combat
                    .project_committed_lethal(lethal)
                    .map_err(|_| ())?
                    .clone()
            };
            let Some(_) = self
                .dispatch_familiar_owner_event(
                    actor,
                    session,
                    FamiliarOwnerEvent::Death(&projection),
                    access,
                )
                .await?
            else {
                return Err(());
            };
            completed += 1;
        }
        Ok(completed)
    }
    /// The selected source family owns its terminal disposition. Unknown durable outcomes
    /// never produce a terminal wire rejection while the original intent remains retained.
    pub(in crate::gameplay_transport) async fn cast_native_familiar<
        A: CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        access: &A,
    ) -> NativeCastDispatch {
        let Some((spell, active_spell)) = self.spells.source_indexed(intent.spell) else {
            return NativeCastDispatch::NotApplicable;
        };
        if !familiar_spell(spell) {
            return NativeCastDispatch::NotApplicable;
        }
        let result = self
            .dispatch_familiar_inner(actor, session, command_id, intent, access, active_spell)
            .await;
        let states = self.spell_states.lock().await;
        if states
            .pending_familiars
            .iter()
            .any(|p| p.actor == actor && p.session == session)
        {
            NativeCastDispatch::Pending
        } else {
            NativeCastDispatch::Outcome(result.unwrap_or_else(SpellCastOutcome::rejected))
        }
    }

    /// Must run before control-loss commits or detaches the current player owner. Retrying
    /// uses the private original command, target and aim; neither a new wire intent nor a
    /// regenerated occurrence can replace an uncertain paid cast.
    pub(in crate::gameplay_transport) async fn reconcile_pending_familiar_for_control_loss<
        A: CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        access: &A,
    ) -> Option<NativeCastDispatch> {
        let original = {
            let states = self.spell_states.lock().await;
            states
                .pending_familiars
                .iter()
                .find(|p| p.actor == actor && p.session == session)
                .map(|p| (p.command_id, p.intent))
        };
        if let Some((command, intent)) = original {
            return Some(
                self.cast_native_familiar(actor, session, command, &intent, access)
                    .await,
            );
        }
        let original = {
            let states = self.spell_states.lock().await;
            states
                .pending_familiar_lifecycle
                .iter()
                .find(|p| p.actor == actor && p.session == session)
                .cloned()
        }?;
        match self
            .reconcile_original_familiar_lifecycle(original, access)
            .await
        {
            Ok(_) => {
                let runtime = self.runtime.lock().await;
                let states = self.spell_states.lock().await;
                Some(NativeCastDispatch::Outcome(SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: super::super::observe_vitals(&runtime, &states, actor, session),
                }))
            }
            Err(()) => {
                let states = self.spell_states.lock().await;
                if states.has_pending_familiar(actor, session) {
                    Some(NativeCastDispatch::Pending)
                } else {
                    Some(NativeCastDispatch::Outcome(SpellCastOutcome::rejected()))
                }
            }
        }
    }

    /// Resume the already sealed owner event without reconstructing an admission, XP
    /// or lethal receipt from historical IDs. Its frozen source origin remains original.
    async fn reconcile_original_familiar_lifecycle<A: CurrentSpellAccessOwner + Sync>(
        &self,
        original: PreparedFamiliarLifecycle,
        access: &A,
    ) -> Result<crate::spell::companion_lifecycle::LifecycleFamiliarApplyReceipt, ()> {
        let actor = original.actor;
        let session = original.session;
        let content = self
            .active_generation
            .and_then(|a| a.native_gameplay())
            .ok_or(())?;
        if original.settings.value.benefit_use == FamiliarBenefitUse::NotApplicableDeath {
            let owned = self
                .read_familiar_lifecycle_inputs(
                    actor,
                    session,
                    content,
                    &super::super::super::spell_access_facts::UnavailableAccessOwner,
                )
                .await?;
            let source = DeathCleanupSource {
                actor,
                session,
                digest: content.source_digest(),
            };
            let mut runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            return self
                .finish_familiar_lifecycle(
                    &mut runtime,
                    &mut states,
                    original,
                    owned.binding(),
                    &source,
                )
                .await;
        }
        let config = content.familiar_config().ok_or(())?;
        let (spell, true) = self.spells.source_indexed(original.spell_index).ok_or(())? else {
            return Err(());
        };
        if !familiar_spell(spell) {
            return Err(());
        }
        let owned = self
            .read_familiar_lifecycle_inputs(actor, session, content, access)
            .await?;
        let fence = self.familiar_fence(session).await.ok_or(())?;
        let group = {
            let runtime = self.runtime.lock().await;
            if !current_owner(&runtime, &fence, actor, session) {
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
        let mut runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        let retained = states
            .pending_familiar_lifecycle
            .iter()
            .find(|p| p.actor == actor && p.session == session)
            .ok_or(())?;
        if retained.familiar.binding() != original.familiar.binding()
            || retained.before != original.before
            || runtime.content_pin().server_artifact_digest() != content.source_digest()
        {
            return Err(());
        }
        self.finish_familiar_lifecycle(
            &mut runtime,
            &mut states,
            original,
            owned.binding(),
            &source,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn dispatch_familiar_inner<A: CurrentSpellAccessOwner + Sync>(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &SpellCastIntent,
        access: &A,
        active_spell: bool,
    ) -> Option<SpellCastOutcome> {
        if !active_spell
            || command_id == 0
            || intent.target != SpellTarget::None
            || intent.aim_at_target
        {
            return Some(SpellCastOutcome::rejected());
        }
        {
            let states = self.spell_states.lock().await;
            if states.pending_familiars.iter().any(|p| {
                p.actor == actor
                    && p.session == session
                    && (p.command_id != command_id || p.intent != *intent)
            }) {
                return None;
            }
            // An unrelated uncertain source transaction may not be overtaken by a new cast.
            if states.has_pending_familiar(actor, session)
                && !states
                    .pending_familiars
                    .iter()
                    .any(|p| p.actor == actor && p.session == session)
            {
                return None;
            }
        }
        let active = self.active_generation?;
        let content = active.native_gameplay()?;
        let config = content.familiar_config()?;
        if config.source_digest() != content.source_digest() {
            return None;
        }
        let fence = self.familiar_fence(session).await?;
        let command = CommandRef::new(session, CommandId::new(command_id).ok()?);
        // Complete already-installed replay using the real physical owner ledger. No new
        // cost, familiar allocation or eligibility evaluation follows an exact replay.
        {
            let mut runtime = self.runtime.lock().await;
            let mut states = self.spell_states.lock().await;
            if !current_owner(&runtime, &fence, actor, session)
                || runtime.content_pin().server_artifact_digest() != content.source_digest()
            {
                return None;
            }
            let attacker =
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).ok()?;
            if let Some(batch) = runtime
                .retained_spell_batch(actor, attacker, fence.character_lease_generation, command)
                .ok()?
            {
                let (spell, _) = self.spells.source_indexed(intent.spell)?;
                let Execution::NativeProfile(profile) = &spell.execution else {
                    return None;
                };
                // Familiar casts accept only target=None/aim=false above. Their original
                // ledger binds the complete exact profile rather than legacy spell IDs.
                // The genuine original ledger retains the full qualified source touch
                // list. Replay compares source identity and accepts no new touch request.
                if !matches_replayed_familiar_profile(&batch.binding, profile) {
                    return Some(SpellCastOutcome::rejected());
                }
                let staged = runtime.stage_spell_batch(&batch).ok()?;
                if staged.will_apply() {
                    return None;
                }
                super::super::commit_owner_batch(&mut runtime, &mut states, staged, None).ok()?;
                return Some(SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: super::super::observe_vitals(&runtime, &states, actor, session),
                });
            }
        }
        let item_fence = CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: session,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let raw = self
            .root
            .read_cast_durable_facts(
                self.character,
                self.holder,
                &item_fence,
                command,
                content.source_digest(),
            )
            .await
            .ok()?;
        let owned = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            if !current_owner(&runtime, &fence, actor, session) {
                return None;
            }
            let state = states.get(&runtime, actor, session)?;
            let retry_access = FamiliarRetryAccess {
                owner: access,
                original_retained: states.pending_familiars.iter().any(|p| {
                    p.actor == actor
                        && p.session == session
                        && p.command_id == command_id
                        && p.intent == *intent
                }),
            };
            qualify_raw_owned_cast_facts(
                &raw,
                command,
                &item_fence,
                &runtime,
                actor,
                state,
                active,
                &retry_access,
                self.owner_now().get(),
            )
            .ok()?
        };
        // Administrative state has an explicit source bootstrap and current SQL fence.
        // Missing Premium/learning evidence remains unavailable in the registered adapter.
        let group = {
            let runtime = self.runtime.lock().await;
            if !current_owner(&runtime, &fence, actor, session) {
                return None;
            }
            self.root
                .initialize_familiar_group(self.character, self.holder, fence)
                .await
                .ok()?
        };
        let administrative = PersistedFamiliarGroupOwner::from_observed(group);
        let source = FamiliarSourceAdapter {
            config,
            premium: access,
            administrative: &administrative,
        };
        self.cast_durable_familiar(actor, session, command_id, intent, content, &owned, &source)
            .await
    }

    /// Advance and Death callers supply the real committed XP or lethal owner receipt.
    /// No client event, synthetic cast command or guessed commercial permission is accepted.
    pub(in crate::gameplay_transport) async fn dispatch_familiar_owner_event<
        A: CurrentSpellAccessOwner + Sync,
    >(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        event: FamiliarOwnerEvent<'_>,
        access: &A,
    ) -> Result<Option<crate::spell::companion_lifecycle::LifecycleFamiliarApplyReceipt>, ()> {
        let Some(content) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return Ok(None);
        };
        if matches!(event, FamiliarOwnerEvent::Death(_)) {
            let owned = self
                .read_familiar_lifecycle_inputs(
                    actor,
                    session,
                    content,
                    &super::super::super::spell_access_facts::UnavailableAccessOwner,
                )
                .await?;
            let vocation =
                crate::spell::Vocation::from_key(owned.durable_build().vocation()).ok_or(())?;
            let Some(index) = self.active_familiar_index(base_familiar_vocation(vocation)) else {
                return Ok(None);
            };
            let source = DeathCleanupSource {
                actor,
                session,
                digest: content.source_digest(),
            };
            return self
                .apply_familiar_owner_event(actor, session, index, event, content, &owned, &source)
                .await
                .map(Some);
        }
        let config = content.familiar_config().ok_or(())?;
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
            if !current_owner(&runtime, &fence, actor, session) {
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
        self.apply_familiar_owner_event(actor, session, index, event, content, &owned, &source)
            .await
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> crate::spell::native::CompiledNativeSpell {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
        ))
        .expect("canonical source-qualified catalog");
        let row = catalog["profiles"]
            .as_array()
            .expect("profiles")
            .iter()
            .find(|v| v["name"] == name)
            .expect("source profile");
        crate::spell::native::spell_from_bundle(
            &serde_json::json!({"spell":row["spell"]}),
            &row["dependencies"],
        )
        .expect("canonical profile qualifies")
    }

    #[test]
    fn exact_installed_familiar_replay_cannot_substitute_source_profile_or_extra_binding() {
        let knight = profile("Knight familiar");
        let druid = profile("Druid familiar");
        let mut value = knight.spell().clone();
        value.as_object_mut().expect("spell object").insert(
            "source_companion_touches".into(),
            serde_json::json!("actual-owner-touch-seal-fixture"),
        );
        let bytes = serde_json::to_vec(&value).expect("ledger binding");
        assert!(matches_replayed_familiar_profile(&bytes, &knight));
        assert!(!matches_replayed_familiar_profile(&bytes, &druid));
        value["source_companion_touches"] = serde_json::json!([]);
        assert!(!matches_replayed_familiar_profile(
            &serde_json::to_vec(&value).unwrap(),
            &knight
        ));
        value["source_companion_touches"] = serde_json::json!("actual-owner-touch-seal-fixture");
        value["unexpected_source"] = serde_json::json!(true);
        assert!(!matches_replayed_familiar_profile(
            &serde_json::to_vec(&value).unwrap(),
            &knight
        ));
        assert!(!matches_replayed_familiar_profile(b"[]", &knight));
        assert!(!matches_replayed_familiar_profile(b"invalid json", &knight));
    }

    #[test]
    fn source_death_cleanup_does_not_read_commercial_benefits_but_requires_matching_owner_and_creature()
     {
        use crate::spell::native_companions::{
            CompanionFacts, CompanionPlan, FamiliarAction, FamiliarFacts, plan,
        };
        let source = profile("Knight familiar");
        let params = &source.spell()["execution"]["native_behavior"]["parameters"];
        // Explicit N/A representations for source inputs the Death branch never reads;
        // this pure case grants no Premium or live actor capability.
        let mut facts = FamiliarFacts {
            event: FamiliarEvent::FamiliarDeath,
            vocation: "knight".into(),
            premium: false,
            level: 0,
            account_at_least_god: false,
            owned_summons: 0,
            spawn_room: false,
            chosen_look: 0,
            has_vocation_look: false,
            owner_current_speed: 0,
            familiar_base_speed: 0,
            familiar_minutes: 0,
            vip: false,
            vip_reduction_minutes: 0,
            cooldown_rate: 0.0,
            now_unix: 1000,
            saved_expiry_unix: 2000,
            last_logout_unix: 0,
            owner_present: true,
            creature_present: true,
            creature_name: "Knight familiar".into(),
            matching_summon_ids: Vec::new(),
            dx: 0,
            dy: 0,
            dz: 0,
            owner_tile_is_teleport: false,
        };
        let expected = CompanionPlan::Familiar(vec![
            FamiliarAction::StoreExpiry(1000),
            FamiliarAction::CancelAndResetWarnings,
        ]);
        assert_eq!(
            plan(params, &CompanionFacts::Familiar(facts.clone())).unwrap(),
            expected
        );
        facts.premium = true;
        facts.level = 1000;
        facts.account_at_least_god = true;
        facts.vip = true;
        facts.familiar_minutes = 30;
        facts.cooldown_rate = 1.0;
        assert_eq!(
            plan(params, &CompanionFacts::Familiar(facts.clone())).unwrap(),
            expected
        );
        facts.owner_present = false;
        assert_eq!(
            plan(params, &CompanionFacts::Familiar(facts.clone())).unwrap(),
            CompanionPlan::Familiar(Vec::new())
        );
        facts.owner_present = true;
        facts.creature_name = "Unrelated creature".into();
        assert_eq!(
            plan(params, &CompanionFacts::Familiar(facts)).unwrap(),
            CompanionPlan::Familiar(Vec::new())
        );
    }
}
