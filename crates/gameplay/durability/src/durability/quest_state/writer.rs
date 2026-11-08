use super::super::db::begin_type2_transaction;
use super::super::reward_claim_mint::quest::QuestRewardAdmission;
use super::*;

impl DurabilityRoot {
    /// Isolated source fixtures exercise the production reward transaction without adding quest content.
    #[cfg(feature = "test-support")]
    #[allow(
        clippy::too_many_arguments,
        reason = "fixture preserves every production authority and reward admission input"
    )]
    pub async fn commit_fixture_quest_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        admission: QuestRewardAdmission<'_>,
        callback: oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_resolved::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            Some(admission),
            None,
            None,
            vec![callback],
            Vec::new(),
        )
        .await
    }

    /// The same immutable fixture resolution as NPC rewards, retaining every World fence.
    #[cfg(feature = "test-support")]
    #[allow(
        clippy::too_many_arguments,
        reason = "fixture retains the complete production World admission tuple"
    )]
    pub async fn commit_fixture_world_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        admission: QuestRewardAdmission<'_>,
        callback: oteryn_gameplay_quests::quest::world_callbacks::Callback,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_resolved::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, Some(admission.native))),
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            Some(admission),
            None,
            None,
            Vec::new(),
            vec![callback],
        )
        .await
    }

    /// The same immutable fixture resolution as NPC rewards, retaining every World fence.
    #[cfg(feature = "test-support")]
    #[allow(
        clippy::too_many_arguments,
        reason = "fixture retains the complete production World admission tuple"
    )]
    pub async fn commit_fixture_world_admission<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        gate: &G,
        callback: oteryn_gameplay_quests::quest::world_callbacks::Callback,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_resolved::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, None)),
            None,
            None,
            None,
            gate,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            vec![callback],
        )
        .await
    }

    /// Commit one quest transition (§4-§5). Exact replay of the receipt key returns the receipt
    /// under the current recovery fence without session authority; another cause kind on the
    /// same key conflicts. A new request is fenced like an XP award, refused while a death's
    /// respawn is pending, and validated against the locked values: a refusal writes nothing to
    /// the quest relations or the revision chain. `catalogue` must be the Character's current
    /// content revision; an unknown transition is `InvalidInput`.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_quest_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    }

    /// A content-bound world callback uses the same fenced quest writer and receipt.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_callback(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, native)),
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
        )
        .await
    }

    /// A content-bound world callback uses the same fenced quest writer and receipt.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_callback_with_items<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
        native: Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        admission: Option<QuestRewardAdmission<'_>>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, native)),
            None,
            None,
            None,
            gate,
            None,
            None,
            admission,
            None,
            None,
        )
        .await
    }

    /// Bloodshade's fixed access purchase uses the existing fee owner and then
    /// this same Quest writer in one physical transaction.
    pub async fn commit_rotten_blood_access_purchase(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        payment: super::rotten_blood_access::RottenBloodAccessPayment,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = payment.request();
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            Some(payment),
            None,
            None,
            None,
        )
        .await
    }

    /// NPC reward effects participate in the same fenced QuestState transaction.
    #[allow(
        clippy::too_many_arguments,
        reason = "binds the existing authority and Quest transition to admitted reward participants"
    )]
    pub(crate) async fn commit_character_quest_reward<G: grizzly::GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        admission: QuestRewardAdmission<'_>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            gate,
            None,
            None,
            Some(admission),
            None,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn commit_quest_inner<G: grizzly::GrizzlyPremiumGate, const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        context: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::quest_world_callback::QuestWorldContext,
            Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        )>,
        grizzly_action: Option<oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction>,
        grizzly_boss: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            grizzly_boss::GrizzlyBossEntryContext,
        )>,
        grizzly_party: Option<crate::foundation::PartySnapshot>,
        grizzly_gate: &G,
        grizzly_policy: Option<&FiniteProgressionPolicy<String, N>>,
        payment: Option<super::rotten_blood_access::RottenBloodAccessPayment>,
        reward_admission: Option<QuestRewardAdmission<'_>>,
        native: Option<super::native_kills::NativeQuestDeath>,
        benevola: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::world_event::changes::counters::benevola::BenevolaHandover,
        )>,
    ) -> Result<QuestTransitionOutcome> {
        let reward_callbacks =
            oteryn_gameplay_quests::quest::reward_callbacks::load_for(&catalogue)
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let world_callbacks = oteryn_gameplay_quests::quest::world_callbacks::load_for(&catalogue)
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        self.commit_quest_inner_resolved(
            authority,
            node,
            fence,
            request,
            catalogue,
            context,
            grizzly_action,
            grizzly_boss,
            grizzly_party,
            grizzly_gate,
            grizzly_policy,
            payment,
            reward_admission,
            native,
            benevola,
            reward_callbacks,
            world_callbacks,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "shared fenced transaction receives the already resolved immutable source catalogue"
    )]
    async fn commit_quest_inner_resolved<G: grizzly::GrizzlyPremiumGate, const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        context: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::quest_world_callback::QuestWorldContext,
            Option<&oteryn_gameplay_content::content::native_gameplay::NativeGameplayState>,
        )>,
        grizzly_action: Option<oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction>,
        grizzly_boss: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            grizzly_boss::GrizzlyBossEntryContext,
        )>,
        grizzly_party: Option<crate::foundation::PartySnapshot>,
        grizzly_gate: &G,
        grizzly_policy: Option<&FiniteProgressionPolicy<String, N>>,
        payment: Option<super::rotten_blood_access::RottenBloodAccessPayment>,
        reward_admission: Option<QuestRewardAdmission<'_>>,
        native: Option<super::native_kills::NativeQuestDeath>,
        benevola: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::world_event::changes::counters::benevola::BenevolaHandover,
        )>,
        reward_callbacks: Vec<oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback>,
        world_callbacks: Vec<oteryn_gameplay_quests::quest::world_callbacks::Callback>,
    ) -> Result<QuestTransitionOutcome> {
        let timer_recipe = oteryn_gameplay_quests::quest::timers::recipe(&request.transition_key);
        if timer_recipe.is_some() != matches!(request.cause, QuestCause::ServerTimer(_)) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let native_key = request
            .transition_key
            .starts_with(oteryn_gameplay_quests::quest::native_kills::PREFIX)
            || oteryn_gameplay_quests::quest::source_deaths::rule(&request.transition_key)
                .is_some();
        if native_key != native.is_some()
            || native
                .as_ref()
                .is_some_and(|death| !death.valid(&request, &catalogue))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if reward_callbacks.iter().any(|c| {
            c.participants.iter().enumerate().any(|(i, _)| {
                oteryn_gameplay_quests::quest::reward_callbacks::participant_key(&c.key, i)
                    == request.transition_key
            })
        }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if oteryn_gameplay_quests::quest::world_callbacks::load()
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .iter()
            .any(|callback| {
                // Ownership survives a caller renaming the synthetic transition.
                callback
                    .effects
                    .quest_completion
                    .as_ref()
                    .is_some_and(|quest| {
                        catalogue
                            .transition(&request.transition_key)
                            .is_some_and(|transition| {
                                transition.completes && transition.quest.as_str() == quest.as_str()
                            })
                    })
                    || oteryn_gameplay_quests::quest::world_callbacks::completion_transition(
                        callback,
                    )
                    .is_some_and(|transition| transition.key == request.transition_key)
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let collection = catalogue
            .authored_bindings()
            .and_then(|bindings| {
                bindings
                    .quests
                    .iter()
                    .flat_map(|quest| &quest.stages)
                    .find_map(|stage| match &stage.trigger {
                        oteryn_gameplay_quests::quest::authored_bindings::Trigger::Collect {
                            collection,
                        } if collection.key == request.transition_key => Some(collection.as_ref()),
                        _ => None,
                    })
            })
            .cloned();
        let reward_callback = reward_callbacks
            .into_iter()
            .find(|c| c.key == request.transition_key);
        if reward_callback
            .as_ref()
            .and_then(|c| c.map_mark.as_ref())
            .is_some_and(|mark| !mark.valid())
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // Ownership survives a caller removing or renaming a bound effect.
        if let Some(transition) = catalogue.transition(&request.transition_key) {
            oteryn_gameplay_quests::quest::admission_binding::validate_for_catalogue(
                &catalogue,
                transition,
                context.is_some(),
            )
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        }
        if reward_callback.is_none()
            && benevola.is_none()
            && catalogue
                .transition(&request.transition_key)
                .is_some_and(|t| t.effects.is_empty() && !t.completes)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let reward_achievement =
            super::super::reward_claim_mint::quest::prepare_achievement(reward_callback.as_ref())
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
        if (request.transition_key
            == super::super::world_event::changes::counters::benevola::HANDOVER)
            != benevola.is_some()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // The specialised owner evaluates these computed rules under the locked snapshot.
        // A generic NPC transition cannot bypass the action/cause validation.
        let grizzly_key = request
            .transition_key
            .starts_with("oteryn:quest-transition/grizzly/");
        if grizzly_key != grizzly_action.is_some()
            || matches!(
                grizzly_action,
                Some(oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction::EnterBoss(_))
            ) != grizzly_boss.is_some()
            || matches!(
                grizzly_action,
                Some(
                    oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction::KillCreature { .. }
                        | oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction::BossKill { .. }
                )
            ) != grizzly_party.is_some()
            || grizzly_action.as_ref().is_some_and(|action| {
                action.transition_key() != request.transition_key
                    || !grizzly::valid_cause(action, request.cause)
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // A paid-access state change must be composed with the existing fee
        // owner; the public generic Quest entry point cannot grant it for free.
        if (request.transition_key == oteryn_gameplay_quests::quest::rotten_blood::PURCHASE_ACCESS)
            != payment.is_some()
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let callback = if context.is_some() {
            Some(
                world_callbacks
                    .iter()
                    .find(|c| c.key == request.transition_key)
                    .cloned()
                    .ok_or(CharacterProgressionError::InvalidInput)?,
            )
        } else {
            None
        };
        let world_reward = callback
            .as_ref()
            .and_then(|c| c.effects.item_reward.as_ref());
        if (reward_callback.is_some() || world_reward.is_some()) != reward_admission.is_some()
            || (reward_callback.is_some() && !matches!(request.cause, QuestCause::Command(_)))
            || (world_reward.is_some() && !matches!(request.cause, QuestCause::Use(_)))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let lionheart = super::hero_of_bounac::prepare(&request)
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let wayfarer = super::wrath_sleeping_dragon::prepare(&request)
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let achievement = super::super::quest_monk_rewards::prepare(callback.as_ref())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let mount_achievement = super::super::quest_mount_rewards::prepare(callback.as_ref())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let dance_achievement = super::super::quest_unnatural_selection::prepare(callback.as_ref())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        // Native rules require a world occurrence even if a caller renames the
        // transition. Inspect the effects, not only the authored callback key.
        let native_world_rule =
            catalogue
                .transition(&request.transition_key)
                .is_some_and(|transition| {
                    transition.effects.iter().any(|effect| {
                        matches!(
                            effect.effect,
                            oteryn_gameplay_quests::quest::QuestEffectKind::Native(_)
                        )
                    })
                });
        // Value-bearing or context-dependent callbacks cannot use a weaker generic caller.
        if context.is_none()
            && (native_world_rule
                || world_callbacks.iter().any(|c| {
                    c.key == request.transition_key
                        && (!c.state_ops.is_empty()
                            || !c.offerings.is_empty()
                            || c.item_on_target.is_some()
                            || c.world_change.is_some()
                            || c.minimum_level.is_some()
                            || c.admission.is_some()
                            || c.experience.is_some()
                            || c.monk_reward.is_some()
                            || c.native_rule.is_some()
                            || c.effects.item_reward.is_some())
                }))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || !valid_quest_key(&request.transition_key)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // §5.3: a command cause belongs to the fenced GameSession; a still-pending one after a
        // same-GameSession reconnect commits with the current generation, and a replaced
        // session's command is refused. An obligation may come from an earlier session.
        if let QuestCause::Command(command) | QuestCause::Use(command) | QuestCause::Step(command) =
            request.cause
            && command.game_session_id() != fence.game_session_id
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let mut binding = payment.as_ref().map_or_else(
            || request_binding(fence.character_id, &request),
            |payment| payment.binding(fence.character_id),
        );
        if let Some((_, handover)) = &benevola {
            handover.bind(&mut binding)?;
        }
        if let Some(action) = &grizzly_action {
            grizzly::bind_action(&mut binding, action);
        }
        if let Some((_, context)) = &grizzly_boss {
            grizzly_boss::bind_context(&mut binding, context)?;
            if grizzly_action
                != Some(
                    oteryn_gameplay_quests::quest::grizzly_adams::GrizzlyAction::EnterBoss(
                        context.boss_id,
                    ),
                )
            {
                return Err(CharacterProgressionError::InvalidInput);
            }
        }
        if let Some(death) = &native {
            death.bind(&mut binding)?;
        }
        if let Some(source) = &grizzly_party {
            grizzly::bind_party(&mut binding, source)?;
        }
        if let Some((_, context, _)) = &context {
            let bytes = serde_json::to_vec(&(context, &callback))
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
            let digest = Sha256::new()
                .chain_update(binding)
                .chain_update(bytes)
                .finalize();
            binding[1..].copy_from_slice(&digest);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (
            context,
            grizzly_gate,
            grizzly_boss,
            grizzly_policy,
            reward_admission,
            benevola,
        );
        #[cfg(any(test, feature = "test-support"))]
        let test_clock = self.quest_test_clock_handle();
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, (context, grizzly_gate, grizzly_boss, grizzly_policy, reward_admission, benevola)| {
                Box::pin(async move {
                    let character = fence.character_id;
                    let mut reward_tuple=None;
                    let mut tx = if reward_admission.is_some() {
                        let typed=begin_type2_transaction(holder,deadline).await?;
                        reward_tuple=Some(typed.tuple());
                        typed.into_inner()
                    } else if let Some(payment)=&payment {
                        // The fee owner's activation fence is the first database
                        // read, before even recovery or admission statements.
                        let mut tx = begin_type2_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx,&recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        lock_receipt_key(&mut tx,character,&request).await?;
                        if let Some(row)=load_receipt(&mut tx,character,&request).await? {
                            if row.try_get::<Vec<u8>,_>("request_binding")? != binding {
                                return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                            }
                            let receipt=decode_receipt(&row)?;
                            tx.commit(deadline).await?;
                            return Ok(Ok(QuestTransitionOutcome::AlreadyCommitted(receipt)));
                        }
                        match assert_gameplay_fence(&mut tx,&fence,&node).await? {
                            Ok(_) => {}, Err(error) => return Ok(Err(error)),
                        }
                        if !super::rotten_blood_access::consume(&mut tx,fence,payment).await? {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                        tx.into_inner()
                    } else {
                        begin_semantic_transaction(holder, deadline).await?
                    };
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_receipt_key(&mut tx, character, &request).await?;
                    // The MINT cause precedes the Character root, matching the item owner.
                    if reward_callback.as_ref().is_some_and(|c|c.give.is_some())
                        && let QuestCause::Command(command)=request.cause {
                        super::super::reward_claim_mint::lock_cause(&mut tx,command).await?;
                    }
                    if let Some(row) = load_receipt(&mut tx, character, &request).await? {
                        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(QuestTransitionOutcome::AlreadyCommitted(committed)));
                    }
                    if grizzly_action.is_some() {
                        match grizzly::load_refusal(&mut tx, character, &request, &binding).await? {
                            Ok(Some(refusal)) => {
                                commit_semantic_transaction(tx, deadline).await?;
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            }
                            Err(error) => return Ok(Err(error)),
                            Ok(None) => {}
                        }
                    }
                    let Some(transition) = catalogue.transition(&request.transition_key) else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    // Premium's shared latch precedes the Character root lock. Historical
                    // receipts above replay without evaluating current entrance eligibility.
                    if callback.as_ref().and_then(|c| c.admission.as_ref()).is_some_and(|a| a.premium) {
                        let account: String = sqlx::query_scalar(
                            "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid"
                        ).bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                        if !grizzly_gate.task_permitted(&mut tx, uuid_text(&account)?).await? {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    if let Some(recipe) = timer_recipe
                        && !server_timer::admit(&mut tx,fence,&request,recipe).await? {
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    if reward_callback.as_ref().is_some_and(|c| c.requires_premium) {
                        let account: String = sqlx::query_scalar(
                            "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid"
                        ).bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                        if !grizzly_gate.task_permitted(&mut tx, uuid_text(&account)?).await? {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    let id = character.as_bytes().as_slice();
                    let state = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                                profile_revision, ruleset_revision, content_revision, \
                                simulation_revision, evidence_revision, declaration_revision, \
                                policy_revision, reward_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    // Reused from PC3 add7faf92: authoritative locked progression,
                    // after historical receipt lookup and before any fresh write.
                    if let Some(minimum)=reward_callback.as_ref().and_then(|c|c.minimum_level)
                        && state.try_get::<i64,_>("level")? < i64::from(minimum) {
                        tx.rollback().await?;
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let content_revision: String = state.try_get("content_revision")?;
                    if !state_matches_root(&state, &root)
                        || content_revision != catalogue.content_revision()
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    if request.transition_key==oteryn_gameplay_quests::quest::rotten_blood::RESTORE_DOOR
                        && state.try_get::<i64, _>("level")? < i64::from(oteryn_gameplay_quests::quest::rotten_blood::REQUIRED_LEVEL)
                    {
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    if let Some(change) = transition.world_change.as_deref()
                        && !super::super::world_event::change_active(&mut tx, fence.runtime_scope.world_id(), change).await?
                    {
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
                    )
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }
                    let obligation = match request.cause {
                        QuestCause::ClaimObligation(claim) => Some(claim),
                        _ => None,
                    };
                    if let Some(claim) = obligation {
                        let row = sqlx::query(
                            "SELECT character_id::text, transition_key, state \
                               FROM game_character_quest_obligations \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) FOR UPDATE",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .fetch_optional(&mut *tx)
                        .await?;
                        let Some(row) = row else {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        };
                        if uuid_text(row.try_get("character_id")?)? != *character.as_bytes()
                            || row.try_get::<String, _>("transition_key")?
                                != request.transition_key
                        {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        if row.try_get::<String, _>("state")? != "PENDING" {
                            return Ok(Ok(QuestTransitionOutcome::ObligationClosed));
                        }
                    }

                    let quest = transition.quest.as_str();
                    let current_hash = catalogue
                        .definition_hash(quest)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let stored_state = sqlx::query(
                        "SELECT pinned_content_revision, definition_hash, \
                                completed_character_revision IS NOT NULL AS completed \
                           FROM game_character_quest_states \
                          WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2 \
                          FOR UPDATE",
                    )
                    .bind(id)
                    .bind(quest)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (pin, hash, refusal) = match &stored_state {
                        Some(row) => {
                            let hash: [u8; 32] = row
                                .try_get::<Vec<u8>, _>("definition_hash")?
                                .try_into()
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            // §6: a completed quest keeps its pin and no longer blocks.
                            let blocked =
                                hash != current_hash && !row.try_get::<bool, _>("completed")?;
                            (
                                row.try_get::<String, _>("pinned_content_revision")?,
                                hash,
                                blocked.then_some(QuestRefusal::RevisionMismatch),
                            )
                        }
                        None => {
                            let full = !capacity::new_state_fits(&mut tx, character).await?;
                            (
                                content_revision.clone(),
                                current_hash,
                                full.then_some(QuestRefusal::CapacityExceeded),
                            )
                        }
                    };
                    let final_flags: Vec<String> = if request.transition_key == oteryn_gameplay_quests::quest::monk_shrines::FINISH {
                        oteryn_gameplay_quests::quest::monk_shrines::NAMES.iter().map(|name|
                            format!("{}{name}", oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX)).collect()
                    } else { Vec::new() };
                    let mut keys: Vec<&str> = transition
                        .effects
                        .iter()
                        .map(|effect| effect.track.as_str())
                        .collect();
                    keys.extend(catalogue.completion_tracks(quest));
                    let grizzly_keys = grizzly_action.as_ref().map(|_| {
                        oteryn_gameplay_quests::quest::grizzly_adams::storage_keys()
                    }).unwrap_or_default();
                    keys.extend(grizzly_keys.iter().map(String::as_str));
                    keys.extend(final_flags.iter().map(String::as_str));
                    if let Some(callback) = &reward_callback {
                        keys.extend(callback.requires.iter().map(|r| r.track.as_str()));
                        keys.extend(callback.participants.iter().flat_map(oteryn_gameplay_quests::quest::reward_callbacks::QuestParticipant::tracks));
                    }
                    keys.sort_unstable(); keys.dedup();
                    let rows = sqlx::query(
                        "SELECT track_key, quest_key, value FROM game_character_quest_tracks \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND track_key = ANY($2::text[]) \
                          ORDER BY track_key FOR UPDATE",
                    )
                    .bind(id)
                    .bind(&keys)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut stored = BTreeMap::new();
                    for row in &rows {
                        let key: String = row.try_get("track_key")?;
                        let read_only = reward_callback.as_ref().is_some_and(|c|
                            c.requires.iter().any(|r| r.track == key) || c.participants.iter().any(|p| p.tracks().any(|track| track == key)));
                        let expected_owner: &str = if read_only {
                            &catalogue.track(&key).ok_or(DurabilityError::InvalidStoredState)?.quest
                        } else { quest };
                        if row.try_get::<String, _>("quest_key")? != expected_owner {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        stored.insert(key, row.try_get("value")?);
                    }
                    let refusal = if refusal.is_none() {
                        if let Some(collection) = &collection {
                            (!super::collection::admit(&mut tx, fence, &request, collection).await?)
                                .then_some(QuestRefusal::StageMismatch)
                        } else { None }
                    } else { refusal };
                    // Parent guards use the same root lock and sorted track locks as
                    // owned writes. A conversation snapshot cannot authorize value.
                    let refusal = refusal.or_else(|| {
                        reward_callback.as_ref().is_some_and(|c|
                            c.requires.iter().any(|r| !r.holds(&catalogue, &stored)))
                            .then_some(QuestRefusal::StageMismatch)
                    });
                    let refusal = refusal.or_else(|| {
                        (!final_flags.is_empty() && oteryn_gameplay_quests::quest::monk_shrines::completed_count(&stored) != 10)
                            .then_some(QuestRefusal::StageMismatch)
                    });
                    let refusal=if refusal.is_none() {
                        if let Some(death)=&native {(!death.admit(&mut tx,fence,&catalogue).await?).then_some(QuestRefusal::StageMismatch)} else {None}

                    } else {refusal};
                    let mut grizzly_plan = None;
                    let refusal = if let (None, Some(action)) = (refusal, &grizzly_action) {
                        match grizzly::evaluate(&mut tx, character, &stored, &state, action,
                            *grizzly_gate, *grizzly_policy).await? {
                            Ok(plan) => { grizzly_plan = Some(plan); None }
                            Err(refusal) => Some(refusal),
                        }
                    } else { refusal };
                    let refusal = if refusal.is_none() {
                        if let Some(source) = grizzly_party.as_ref().or_else(||native.as_ref().and_then(|death|death.party())) {
                            match super::super::world_party::frozen_kill_membership(&mut tx, &fence, source).await {
                                Ok(true) => None,
                                Ok(false) => Some(QuestRefusal::StageMismatch),
                                Err(super::super::spell_item_transaction::SpellItemError::Durability(error)) => return Err(error),
                                Err(super::super::spell_item_transaction::SpellItemError::Database(error)) => return Err(error.into()),
                                Err(_) => Some(QuestRefusal::StageMismatch),
                            }
                        } else { None }
                    } else { refusal };
                    let refusal = if refusal.is_none() {
                        if let Some((permit, context)) = grizzly_boss.as_ref() {
                            (!grizzly_boss::admit(&mut tx, permit, fence, context).await?)
                                .then_some(QuestRefusal::StageMismatch)
                        } else { None }
                    } else { refusal };
                    // Reading all Grizzly inputs does not reserve capacity for untouched tracks.
                    let written_keys: Vec<&str> = grizzly_plan.as_ref().map_or_else(
                        || keys.iter().copied().filter(|key| reward_callback.as_ref().is_none_or(|c|
                            !c.requires.iter().any(|r| r.track == *key)))
                            .filter(|key| transition.effects.iter().any(|effect| effect.track.as_str() == *key)
                                || !catalogue.completion_tracks(quest).any(|read| read == *key)).collect(),
                        |plan| plan.changes.iter().map(|change| change.track.as_str()).collect(),
                    );
                    let new_tracks = written_keys.iter().filter(|key| !stored.contains_key(**key)).count();
                    let refusal = match refusal {
                        Some(refusal) => Some(refusal),
                        None if new_tracks > 0 => {
                            let existing = written_keys
                                .iter()
                                .filter(|key| stored.contains_key(**key))
                                .count();
                            (!capacity::tracks_fit(&mut tx, character, &written_keys, existing).await?)
                                .then_some(QuestRefusal::CapacityExceeded)
                        }
                        None => None,
                    };
                    // QUEST-GATE-0 §5.5: the already-held character_root lock serializes all
                    // inserts and consumption of this Character's XP obligations. Count them
                    // before any write; FOR UPDATE here would require the UPDATE privilege
                    // intentionally absent from the runtime role. At RL-10 refuse whole.
                    let reward_experience = grizzly_plan.as_ref()
                        .map_or(transition.experience, |plan| plan.experience);
                    let xp_full = match (refusal, reward_experience) {
                        (None, Some(_)) => {
                            let pending = sqlx::query(
                                "SELECT reward_occurrence_id::text \
                                   FROM game_character_quest_xp_obligations \
                                  WHERE character_id = encode($1,'hex')::uuid \
                                  ORDER BY reward_occurrence_id",
                            )
                            .bind(id)
                            .fetch_all(&mut *tx)
                            .await?;
                            pending.len() >= QUESTGATE0_RL_10
                        }
                        _ => false,
                    };
                    let now: i64 =
                        sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
                            .fetch_one(&mut *tx)
                            .await?;
                    #[cfg(any(test, feature = "test-support"))]
                    let now = test_clock.as_ref().map_or(now, |clock| {
                        let configured = clock.load(std::sync::atomic::Ordering::Acquire);
                        if configured >= 0 { configured } else { now }
                    });
                    let mut changes = match refusal
                        .map_or_else(|| grizzly_plan.as_ref().map_or_else(
                            || catalogue.evaluate(transition, &stored, now),
                            |plan| Ok(plan.changes.clone())), Err)
                        .and_then(|changes| {
                            if xp_full {
                                Err(QuestRefusal::OutOfRange)
                            } else {
                                Ok(changes)
                            }
                        }) {
                        Ok(changes) => changes,
                        Err(refusal) => {
                            if let Some(recipe) = timer_recipe
                                && refusal == QuestRefusal::StageMismatch
                                && stored.get(recipe.track).copied().unwrap_or(-1) > recipe.maximum_before {
                                server_timer::record(&mut tx,fence,&request,recipe,stored.get(recipe.track).copied().unwrap_or(-1),None).await?;
                                server_timer::audit(&mut tx,&request).await?;
                                commit_semantic_transaction(tx,deadline).await?;
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            }
                            if grizzly_action.is_some() {
                                grizzly::record_refusal(&mut tx, character, &request, &binding, refusal).await?;
                                commit_semantic_transaction(tx, deadline).await?;
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            }
                            let Some(claim) = obligation else {
                                // Dropping the transaction rolls back: nothing is written.
                                if context.is_some() {
                                    tx.rollback().await?;
                                }
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            };
                            let next = if refusal == QuestRefusal::RevisionMismatch {
                                "WAITING_MIGRATION"
                            } else {
                                "REFUSED"
                            };
                            let updated = sqlx::query(
                                "UPDATE game_character_quest_obligations \
                                    SET state = $3, result_code = $4, \
                                        updated_at = greatest(updated_at, \
                                          floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                                  WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                    AND claim_command_id = $2::text::numeric(20,0) \
                                    AND state = 'PENDING'",
                            )
                            .bind(claim.game_session_id().as_bytes().as_slice())
                            .bind(claim.command_id().get().to_string())
                            .bind(next)
                            .bind(refusal.code())
                            .execute(&mut *tx)
                            .await?;
                            if updated.rows_affected() != 1 {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                        }
                    };

                    // A native death reads its mission gate; it must not move that mission epoch.
                    if native.is_some() {changes.retain(|change|change.before!=change.after);}
                    if !super::rotten_blood_access::validate_access_changes(
                        &request.transition_key,&changes,state.try_get("level")?,
                    )? {
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    // Historical receipts return above without requiring a new grant
                    // participant. Fresh effects still require the admitted generation.
                    if let Some(reward) = callback.as_ref().and_then(|c| c.effects.outfit_reward.as_ref())
                        && context.as_ref().and_then(|(_, _, native)| *native).is_none_or(|native|
                            !super::super::reward_claim_mint::quest::outfit_compatible(native, reward))
                    {
                        tx.rollback().await?;
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    if let (Some(callback), Some((permit, context, _))) = (&callback, &context)
                        && !super::super::quest_world_callback::consume(&mut tx, permit, fence, &request, callback, context, (&recovery, &node)).await?
                    { return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    if let (Some(callback),Some(admission),Some(tuple))=(&reward_callback,*reward_admission,reward_tuple)
                        && !super::super::reward_claim_mint::quest::apply(&mut tx,tuple,&node,fence,&request,callback,admission,reward_achievement.as_ref(),&catalogue,stored_state.is_none()).await?
                    { tx.rollback().await?;return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    if let Some(world) = &callback
                        && let Some(reward) = &world.effects.item_reward
                    {
                        let (Some(admission), Some(tuple)) = (*reward_admission, reward_tuple) else { return Err(DurabilityError::InvalidStoredState); };
                        let QuestCause::Use(command) = request.cause else { return Err(DurabilityError::InvalidStoredState); };
                        // Fresh value must use the generation proved by the physical
                        // occurrence. Historical receipts returned before this check.
                        let revision = format!("sha256:{}", admission.native.source_digest().iter()
                            .map(|byte| format!("{byte:02x}")).collect::<String>());
                        let claim = if let Some(track) = world.effects.item_reward_cycle.as_deref() {
                            let initial = catalogue.track(track).ok_or(DurabilityError::InvalidStoredState)?.initial;
                            super::super::reward_claim_mint::quest::QuestMintClaim::Cycle {
                                track, value: stored.get(track).copied().unwrap_or(initial),
                            }
                        } else { super::super::reward_claim_mint::quest::QuestMintClaim::Once };
                        if context.as_ref().is_none_or(|(_, world, _)| world.content_revision != revision)
                            || admission.weights.generation_digest() != admission.native.source_digest()
                            || super::super::reward_claim_mint::quest::mint(&mut tx,tuple,&node,fence,command,&world.key,claim,reward,admission).await?.is_none()
                        { tx.rollback().await?; return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    }
                    if let Some(world) = &callback
                        && let Some(reward) = &world.effects.outfit_reward
                    {
                        let (cause, ordinal) = request.cause.key();
                        let occurrence = serde_json::to_vec(&(cause, ordinal, &world.key))
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        super::super::character_appearance::grant_content_outfit(
                            &mut tx, fence.character_id, &occurrence, reward,
                            context.as_ref().and_then(|(_, _, native)| *native).ok_or(DurabilityError::InvalidStoredState)?).await?;
                    }
                    if let Some((permit, handover)) = benevola.as_ref()
                        && !super::super::world_event::changes::counters::benevola::consume(
                            &mut tx, permit, &recovery, &node, fence, &request, handover).await?
                    { return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    if request.transition_key==oteryn_gameplay_quests::quest::hive_access::BUY {
                        let world=fence.runtime_scope.world_id();
                        let epoch:Option<String>=sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
                            .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                        let day=if let Some(epoch)=epoch {
                            super::super::world_event::changes::load(&mut tx,world,epoch.parse().map_err(|_|DurabilityError::InvalidStoredState)?).await?
                        }else{None};
                        if day.is_none_or(|day| !oteryn_gameplay_world_change::owner::is_active(&day,"hive_born")
                            || day.counter_stages[oteryn_gameplay_world_change::counter::CounterKind::HiveBorn.index()]<2) {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    if oteryn_gameplay_quests::quest::shared_world_progress::binding(&request.transition_key).is_some() || oteryn_gameplay_quests::quest::shared_world_progress::accepted_task(&request.transition_key).is_some() {
                        // Native object/NPC selection and the locked complete quest plan
                        // precede shared credit. Refusal drops all input and quest writes.
                        let world=fence.runtime_scope.world_id();
                        let epoch:String=sqlx::query_scalar("SELECT coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),0)::text")
                            .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                        let epoch=epoch.parse::<u64>().map_err(|_|DurabilityError::InvalidStoredState)?;
                        let Some(counter_authority)=super::super::world_event::changes::counters::authorize(
                            &mut tx,&recovery,&node,fence.runtime_scope,fence.scope_ownership_generation,epoch,
                        ).await? else {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        };
                        let credited=super::super::world_event::changes::counters::shared_progress::credit_completed(
                            &mut tx,&counter_authority,fence.runtime_scope,crate::foundation::CharacterId::decode(character.as_bytes()).map_err(|_|DurabilityError::InvalidStoredState)?,epoch,
                            &request.transition_key,request.cause,&changes,
                        ).await?;
                        if !matches!(credited,super::super::world_event::changes::counters::CreditOutcome::Applied {..}) {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    if request.transition_key==oteryn_gameplay_quests::quest::fire_feathered_native::COMPLETE {
                        let world=fence.runtime_scope.world_id();
                        let epoch:String=sqlx::query_scalar("SELECT coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),0)::text")
                            .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                        let epoch=epoch.parse::<u64>().map_err(|_|DurabilityError::InvalidStoredState)?;
                        let Some(counter_authority)=super::super::world_event::changes::counters::authorize(
                            &mut tx,&recovery,&node,fence.runtime_scope,fence.scope_ownership_generation,epoch,
                        ).await? else {return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)))};
                        let credited=super::super::world_event::changes::counters::fire_feathered::credit_completed(
                            &mut tx,&counter_authority,fence.runtime_scope,crate::foundation::CharacterId::decode(character.as_bytes()).map_err(|_|DurabilityError::InvalidStoredState)?,epoch,request.cause,&changes,
                        ).await?;
                        if !matches!(credited,super::super::world_event::changes::counters::CreditOutcome::Applied{..}) {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    if let Some(recipe) = timer_recipe {
                        server_timer::record(&mut tx,fence,&request,recipe,changes.first().ok_or(DurabilityError::InvalidStoredState)?.before,Some(committed_revision)).await?;
                    }
                    let (original, committed) =
                        (root.revision.to_string(), committed_revision.to_string());
                    for successor in [
                        "UPDATE game_character_roots \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    ] {
                        let updated = sqlx::query(successor)
                            .bind(id)
                            .bind(&committed)
                            .bind(&original)
                            .execute(&mut *tx)
                            .await?;
                        if updated.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let completes = catalogue.completes_after_claim_sets(transition, &stored, &changes);
                    let (cause_id, ordinal) = request.cause.key();
                    let track_keys: Vec<&str> =
                        changes.iter().map(|change| change.track.as_str()).collect();
                    let before: Vec<i64> = changes.iter().map(|change| change.before).collect();
                    let after: Vec<i64> = changes.iter().map(|change| change.after).collect();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_quest_receipts(\
                           character_id, cause_kind, cause_id, cause_ordinal, transition_key, \
                           quest_key, request_binding, pinned_content_revision, definition_hash, \
                           completes, track_keys, values_before, values_after, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           profile_revision, ruleset_revision, content_revision, \
                           simulation_revision, evidence_revision, declaration_revision, \
                           policy_revision, reward_revision, committed_at) \
                         SELECT s.character_id, $2, encode($3,'hex')::uuid, \
                           $4::text::numeric(20,0), $5, $6, $7, $8, $9, $10, $11::text[], \
                           $12::bigint[], $13::bigint[], $14::text::numeric(20,0), \
                           $15::text::numeric(20,0), s.level, s.level, s.total_experience, \
                           s.total_experience, s.profile_revision, s.ruleset_revision, \
                           s.content_revision, s.simulation_revision, s.evidence_revision, \
                           s.declaration_revision, s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($1,'hex')::uuid",
                    )
                    .bind(id)
                    .bind(request.cause.kind())
                    .bind(cause_id.as_slice())
                    .bind(ordinal.to_string())
                    .bind(&request.transition_key)
                    .bind(quest)
                    .bind(binding.as_slice())
                    .bind(&pin)
                    .bind(hash.as_slice())
                    .bind(completes)
                    .bind(&track_keys)
                    .bind(&before)
                    .bind(&after)
                    .bind(&original)
                    .bind(&committed)
                    .execute(&mut *tx)
                    .await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    // The same native receipt that owns shared Hive/Deeplings credit
                    // also owns personal progress in this World occurrence. Historical
                    // replay returned earlier, so it cannot attach to a later save.
                    if transition.world_change.is_some()
                        || oteryn_gameplay_quests::quest::shared_world_progress::binding(&request.transition_key).is_some()
                        || oteryn_gameplay_quests::quest::shared_world_progress::accepted_task(&request.transition_key).is_some()
                    {
                        sqlx::query("INSERT INTO game_character_event_quest_progress(character_id,world_id,reset_epoch,committed_character_revision,quest_key,transition_key,completes,track_keys,values_after) SELECT r.character_id,a.world_id,a.reset_epoch,r.committed_character_revision,r.quest_key,r.transition_key,r.completes,r.track_keys,r.values_after FROM game_character_quest_receipts r JOIN game_world_reset_active a ON a.world_id=encode($3,'hex')::uuid WHERE r.character_id=encode($1,'hex')::uuid AND r.committed_character_revision=$2::text::numeric")
                            .bind(id).bind(&committed).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                            .execute(&mut *tx).await?;
                    }
                    for change in &changes {
                        let written = sqlx::query(
                            "INSERT INTO game_character_quest_tracks(character_id, track_key, \
                               quest_key, value, committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0)) \
                             ON CONFLICT (character_id, track_key) DO UPDATE \
                               SET value = EXCLUDED.value, \
                                   committed_character_revision = \
                                     EXCLUDED.committed_character_revision",
                        )
                        .bind(id)
                        .bind(&change.track)
                        .bind(quest)
                        .bind(change.after)
                        .bind(&committed)
                        .execute(&mut *tx)
                        .await?;
                        if written.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let completed = completes.then_some(committed.as_str());
                    let written = if stored_state.is_some() {
                        sqlx::query(
                            "UPDATE game_character_quest_states \
                                SET committed_character_revision = $3::text::numeric(20,0), \
                                    completed_character_revision = coalesce(\
                                      completed_character_revision, $4::text::numeric(20,0)) \
                              WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    } else {
                        sqlx::query(
                            "INSERT INTO game_character_quest_states(character_id, quest_key, \
                               pinned_content_revision, definition_hash, \
                               started_character_revision, completed_character_revision, \
                               committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0), $6::text::numeric(20,0), \
                               $5::text::numeric(20,0))",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&pin)
                        .bind(hash.as_slice())
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    };
                    if written.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if let Some(world) = &callback
                        && !super::super::quest_world_callback::completion::apply(
                            &mut tx, fence, &request, world, &catalogue,
                        ).await?
                    {
                        tx.rollback().await?;
                        return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                    }
                    let experience = match reward_experience {
                        Some(amount) => Some(
                            insert_xp_obligation(
                                &mut tx,
                                character,
                                &request,
                                amount,
                                &pin,
                            )
                            .await?,
                        ),
                        None => None,
                    };
                    if let Some(claim) = obligation {
                        let consumed = sqlx::query(
                            "UPDATE game_character_quest_obligations \
                                SET state = 'CONSUMED', \
                                    updated_at = greatest(updated_at, \
                                      floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) \
                                AND state = 'PENDING'",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .execute(&mut *tx)
                        .await?;
                        if consumed.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    if let (Some(callback), Some((_, context, _))) = (&callback, &context) {
                        super::super::quest_monk_rewards::earn(&mut tx, fence, &request, callback,
                            &context.content_revision, achievement.as_ref()).await?;
                        super::super::quest_unnatural_selection::earn(&mut tx, fence, &request, callback,
                            &changes, dance_achievement.as_ref()).await?;
                    }
                    // Item custody participants finish before the account appearance lock.
                    if let Some(world) = &callback
                        && world.effects.mount_reward.is_some()
                    {
                        let Some(native) = context.as_ref().and_then(|(_, _, native)| *native) else {
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        };
                        if !super::super::quest_mount_rewards::earn(&mut tx, fence, &request, world, native, mount_achievement.as_ref()).await? {
                            tx.rollback().await?;
                            return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)));
                        }
                    }
                    super::hero_of_bounac::earn(&mut tx, fence, &request, &changes, lionheart.as_ref()).await?;
                    super::planegazer_finale::earn(&mut tx, fence, &request, &changes).await?;
                    super::wrath_sleeping_dragon::earn(&mut tx, fence, &request, &changes, wayfarer.as_ref()).await?;
                    if let Some(plan) = &grizzly_plan {
                        grizzly::earn(&mut tx, character, &request, plan, &content_revision).await?;
                    }
                    if let Some((_, context)) = grizzly_boss.as_ref() {
                        grizzly_boss::earn(&mut tx, fence, &request, context).await?;
                    }
                    if timer_recipe.is_some() { server_timer::audit(&mut tx,&request).await?; }
                    if let Some(callback) = &reward_callback {
                        map_marks::record(&mut tx, fence, callback, committed_revision).await?;
                    }
                    let committed = CommittedQuestTransition {
                        character_id: character,
                        cause: request.cause,
                        transition_key: request.transition_key,
                        quest_key: quest.to_owned(),
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        pinned_content_revision: pin,
                        definition_hash: hash,
                        completes,
                        changes,
                        experience,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(QuestTransitionOutcome::Committed(committed)))
                })
            })
            .await?
    }
}
