//! Grizzly Adams uses the existing quest tracks, revision chain and XP obligations.
//! Rules run against the complete bounded snapshot read under the Character root lock;
//! callers supply an action, never a proposed storage delta. Item delivery remains with MINT.

use super::*;
use oteryn_gameplay_quests::quest::grizzly_adams::{
    self as rules, GrizzlyAction, GrizzlyActor, GrizzlyRefusal, GrizzlyTransition,
};

pub const HOLY_ICON_CLAIM: &str = "oteryn:reward-claim.grizzly-holy-icon";

/// Native Premium authority joined to this quest transaction. Grizzly's accepted
/// catalogue requirements require Current, including before Premium activation.
pub trait GrizzlyPremiumGate: Sync {
    fn task_permitted(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> impl std::future::Future<Output = StoredResult<bool>> + Send;
}

/// Kill/report/cancel paths need no Premium decision. Absence of the native Premium
/// owner may never grant a premium task, including before an activation was observed.
pub struct NoGrizzlyPremium;

impl GrizzlyPremiumGate for NoGrizzlyPremium {
    async fn task_permitted(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _account: [u8; 16],
    ) -> StoredResult<bool> {
        Ok(false)
    }
}

/// The one donor item reward, earned with the Demon task's quest receipt. A full
/// backpack leaves it pending; the next NPC interaction retries delivery, including
/// after a new admission has restored the Character's quest state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingGrizzlyReward {
    pub transition_key: String,
    pub item_source_id: u32,
    pub claim_key: String,
    pub content_revision: String,
}

impl DurabilityRoot {
    /// Reach only through the Character's revision slot. Exact replay uses the original
    /// quest receipt, or the recorded refusal, without evaluating the rules a second time.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_grizzly_transition<G: GrizzlyPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        gate: &G,
    ) -> Result<QuestTransitionOutcome> {
        let request = QuestTransitionRequest {
            transition_key: action.transition_key(),
            cause,
        };
        self.commit_quest_inner::<_, 0>(authority, node, fence, request, catalogue, None, Some(action), None, None, gate, None)
            .await
    }

    /// Task XP is still earned as an existing durable obligation. The pinned native
    /// policy lets the locked writer evaluate points at the level after that reward,
    /// including earlier pending rewards from this same source NPC batch.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_grizzly_transition_with_progression<G: GrizzlyPremiumGate, const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        gate: &G,
        policy: Option<&FiniteProgressionPolicy<String, N>>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner(authority, node, fence,
            QuestTransitionRequest { transition_key: action.transition_key(), cause },
            catalogue, None, Some(action), None, None, gate, policy).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_grizzly_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        action: GrizzlyAction,
        catalogue: Arc<QuestStateCatalogue>,
        source: crate::foundation::PartySnapshot,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(authority, node, fence,
            QuestTransitionRequest { transition_key: action.transition_key(), cause },
            catalogue, None, Some(action), None, Some(source), &NoGrizzlyPremium, None).await
    }

    pub async fn reconcile_grizzly_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        cause: QuestCause,
        action: GrizzlyAction,
        source: crate::foundation::PartySnapshot,
    ) -> Result<Option<QuestTransitionOutcome>> {
        if !matches!(action, GrizzlyAction::KillCreature { .. } | GrizzlyAction::BossKill { .. }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        self.reconcile_grizzly_inner(authority, character, cause, action, None, Some(source)).await
    }

    pub async fn pending_grizzly_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<Option<PendingGrizzlyReward>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    // The primary key is Character: the Demon task can earn this only once.
                    let row = sqlx::query(
                        "SELECT transition_key,item_source_id,claim_key,content_revision \
                         FROM game_character_grizzly_reward_entitlements \
                         WHERE character_id=encode($1,'hex')::uuid AND state='PENDING'",
                    )
                    .bind(character.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let reward = row
                        .map(|row| -> StoredResult<PendingGrizzlyReward> {
                            Ok(PendingGrizzlyReward {
                                transition_key: row.try_get("transition_key")?,
                                item_source_id: u32::try_from(row.try_get::<i32, _>("item_source_id")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                claim_key: row.try_get("claim_key")?,
                                content_revision: row.try_get("content_revision")?,
                            })
                        })
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(reward))
                })
            })
            .await?
    }

    /// Resolve an unknown commit using only historical evidence. This does not grant
    /// permission to act in the original session or turn a refusal into a new attempt.
    pub async fn reconcile_character_grizzly_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        cause: QuestCause,
        action: GrizzlyAction,
    ) -> Result<Option<QuestTransitionOutcome>> {
        if matches!(action, GrizzlyAction::EnterBoss(_) | GrizzlyAction::KillCreature { .. } | GrizzlyAction::BossKill { .. }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        self.reconcile_grizzly_inner(authority, character, cause, action, None, None).await
    }

    pub(super) async fn reconcile_grizzly_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        cause: QuestCause,
        action: GrizzlyAction,
        context: Option<super::grizzly_boss::GrizzlyBossEntryContext>,
        source: Option<crate::foundation::PartySnapshot>,
    ) -> Result<Option<QuestTransitionOutcome>> {
        if !valid_cause(&action, cause) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let request = QuestTransitionRequest {
            transition_key: action.transition_key(),
            cause,
        };
        let mut binding = request_binding(character, &request);
        bind_action(&mut binding, &action);
        if let Some(context) = &context {
            super::grizzly_boss::bind_context(&mut binding, context)?;
        }
        if let Some(source) = &source { bind_party(&mut binding, source)?; }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_receipt_key(&mut tx, character, &request).await?;
                    if let Some(row) = load_receipt(&mut tx, character, &request).await? {
                        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let receipt = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(Some(QuestTransitionOutcome::AlreadyCommitted(receipt))));
                    }
                    let refusal = match load_refusal(&mut tx, character, &request, &binding).await? {
                        Ok(refusal) => refusal,
                        Err(error) => return Ok(Err(error)),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(refusal.map(QuestTransitionOutcome::Refused)))
                })
            })
            .await?
    }
}

pub(super) fn bind_action(binding: &mut [u8; 33], action: &GrizzlyAction) {
    let digest = Sha256::new()
        .chain_update(*binding)
        .chain_update(action.binding_bytes())
        .finalize();
    binding[1..].copy_from_slice(&digest);
}

/// Complete original source observation is request data, never current authority.
/// The World party owner independently verifies it in the successful transaction.
pub(super) fn bind_party(binding: &mut [u8; 33], source: &crate::foundation::PartySnapshot) -> Result<()> {
    if !source.is_well_formed() || source.observed_database_micros < 0 {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let mut digest = Sha256::new().chain_update(*binding).chain_update(b"grizzly-party-v1")
        .chain_update(source.world).chain_update(source.character);
    for id in [source.party_id, source.leader] {
        digest.update([u8::from(id.is_some())]);
        if let Some(id) = id { digest.update(id); }
    }
    digest.update(source.revision.unwrap_or(0).to_be_bytes());
    digest.update([u8::from(source.shared_xp_enabled)]);
    digest.update(source.observed_database_micros.to_be_bytes());
    digest.update((source.members.len() as u64).to_be_bytes());
    for member in &source.members {
        digest.update(member.character);
        digest.update(member.invitation_order.to_be_bytes());
        digest.update(member.presence_revision.to_be_bytes());
        digest.update([match member.presence {
            crate::foundation::PartyPresence::Channel => 1,
            crate::foundation::PartyPresence::Hidden => 2,
            crate::foundation::PartyPresence::Offline => 3,
        }]);
        digest.update([u8::from(member.channel.is_some())]);
        if let Some(channel) = member.channel { digest.update(channel); }
    }
    binding[1..].copy_from_slice(&digest.finalize());
    Ok(())
}

/// The source namespace is part of authority, not just an idempotency discriminator.
pub(super) fn valid_cause(action: &GrizzlyAction, cause: QuestCause) -> bool {
    match action {
        GrizzlyAction::KillCreature { .. } | GrizzlyAction::BossKill { .. } => {
            matches!(cause, QuestCause::CreatureDeath(_))
        }
        GrizzlyAction::EnterBoss(_) => matches!(cause, QuestCause::Step(_)),
        _ => matches!(cause, QuestCause::Command(_)),
    }
}

pub(super) async fn evaluate<G: GrizzlyPremiumGate, const N: usize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    state: &BTreeMap<String, i64>,
    progression: &sqlx::postgres::PgRow,
    action: &GrizzlyAction,
    gate: &G,
    policy: Option<&FiniteProgressionPolicy<String, N>>,
) -> StoredResult<std::result::Result<GrizzlyTransition, QuestRefusal>> {
    let level = u32::try_from(progression.try_get::<i64, _>("level")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if let GrizzlyAction::KillCreature { task_id, observed_revision, .. } = action {
        // The death owner froze an active task from this coherent revision. A kill
        // waiting in the queue cannot move a replacement run of that same task.
        let restarted: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks \
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 \
               AND committed_character_revision > $3::text::numeric)",
        )
        .bind(character.as_bytes().as_slice())
        .bind(rules::storage_track(rules::QUEST_STORAGE_BASE + u32::from(*task_id)))
        .bind(observed_revision.to_string())
        .fetch_one(&mut **tx)
        .await?;
        if *observed_revision == 0 || restarted {
            return Ok(Err(QuestRefusal::StageMismatch));
        }
    }
    if let GrizzlyAction::BossKill { boss_id, observed_revision } = action {
        let Some(boss) = rules::boss(*boss_id) else {
            return Ok(Err(QuestRefusal::NotSupported));
        };
        if *observed_revision == 0 {
            return Ok(Err(QuestRefusal::StageMismatch));
        }
        // Source death increments precede later NPC resets. A delayed death may
        // not resurrect a counter cleared by selection, task completion/report or
        // a special task. Only command receipts can reset it; genuine queued boss
        // increments remain independent and must not invalidate one another.
        // The 0648 partial index excludes the high-volume death receipt history.
        let reset: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_receipts \
             WHERE character_id=encode($1,'hex')::uuid \
               AND committed_character_revision > $3::text::numeric \
               AND cause_kind='command' \
               AND quest_key='oteryn:quest.killing_in_the_name_of_quest' \
               AND transition_key LIKE 'oteryn:quest-transition/grizzly/%' \
               AND values_after[array_position(track_keys,$2)] = 0)",
        )
        .bind(character.as_bytes().as_slice())
        .bind(rules::storage_track(boss.kill_storage))
        .bind(observed_revision.to_string())
        .fetch_one(&mut **tx).await?;
        if reset {
            return Ok(Err(QuestRefusal::StageMismatch));
        }
    }
    let premium = if matches!(action, GrizzlyAction::StartTask(_)) {
        let account: String = sqlx::query_scalar(
            "SELECT account_id::text FROM game_character_roots \
             WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(character.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        gate.task_permitted(tx, uuid_text(&account)?).await?
    } else {
        false
    };
    let level_after_reward = match action {
        GrizzlyAction::CompleteTask(id) => match rules::task(*id).and_then(|task| task.experience) {
            Some(amount) => match completion_level(tx, character, progression, level, amount, policy).await? {
                Ok(level) => Some(level),
                Err(refusal) => return Ok(Err(refusal)),
            },
            None => None,
        },
        _ => None,
    };
    Ok(rules::plan_action(state, GrizzlyActor { level, premium, level_after_reward }, action).map_err(|refusal| {
        match refusal {
            GrizzlyRefusal::OutOfRange => QuestRefusal::OutOfRange,
            GrizzlyRefusal::InvalidCatalogue | GrizzlyRefusal::InvalidTask
                | GrizzlyRefusal::InvalidBoss | GrizzlyRefusal::ProgressionUnavailable => QuestRefusal::NotSupported,
            _ => QuestRefusal::StageMismatch,
        }
    }))
}

/// Crystal awards XP before checking the point cap (NPC:462-463,485-503).
/// The Character root lock serializes both pending-obligation insertion and XP
/// consumption. Projecting their bounded sum is therefore equivalent whether an
/// earlier task's XP is already in the progression row or still awaiting its owner.
async fn completion_level<const N: usize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    progression: &sqlx::postgres::PgRow,
    level: u32,
    reward: i64,
    policy: Option<&FiniteProgressionPolicy<String, N>>,
) -> StoredResult<std::result::Result<u32, QuestRefusal>> {
    use crate::domain::progression::{CurrentProgressionSnapshot, ProgressionOperation, calculate_progression};
    let Some(policy) = policy else { return Ok(Err(QuestRefusal::NotSupported)); };
    if !super::super::character_progression::stored_context_matches(
        progression, &policy.context, &policy.policy_revision, &policy.reward_revision,
    ) {
        return Ok(Err(QuestRefusal::RevisionMismatch));
    }
    let pending: Vec<i64> = sqlx::query_scalar(
        "SELECT amount FROM game_character_quest_xp_obligations \
         WHERE character_id=encode($1,'hex')::uuid \
         ORDER BY created_at,reward_occurrence_id LIMIT $2",
    )
    .bind(character.as_bytes().as_slice())
    .bind(i64::try_from(QUESTGATE0_RL_10 + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
    .fetch_all(&mut **tx).await?;
    if pending.len() >= QUESTGATE0_RL_10 {
        return Ok(Err(QuestRefusal::OutOfRange));
    }
    let Some(amount) = pending.into_iter().try_fold(reward, |sum, amount| {
        if amount > 0 { sum.checked_add(amount) } else { None }
    }) else { return Ok(Err(QuestRefusal::OutOfRange)); };
    let snapshot = CurrentProgressionSnapshot {
        level,
        total_experience: ExactI64::new(progression.try_get("total_experience")?),
        skill_progression: (), magic_progression: (),
    };
    let operation = ProgressionOperation::AwardExperience {
        source_occurrence: (), reward_revision: policy.reward_revision.clone(),
        amount: ExactI64::new(amount),
    };
    Ok(calculate_progression(&snapshot, &policy.context, &policy.policy_revision,
        &operation, policy).map(|result| result.level_after)
        .map_err(|_| QuestRefusal::OutOfRange))
}

pub(super) async fn load_refusal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
    binding: &[u8; 33],
) -> StoredResult<std::result::Result<Option<QuestRefusal>, CharacterProgressionError>> {
    let (id, ordinal) = request.cause.key();
    let row = sqlx::query(
        "SELECT request_binding,result_code FROM game_character_grizzly_refusals \
         WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid \
         AND cause_ordinal=$3::text::numeric AND transition_key=$4",
    )
    .bind(character.as_bytes().as_slice())
    .bind(id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else { return Ok(Ok(None)) };
    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
    }
    let refusal = match row.try_get::<&str, _>("result_code")? {
        "STAGE_MISMATCH" => QuestRefusal::StageMismatch,
        "REVISION_MISMATCH" => QuestRefusal::RevisionMismatch,
        "OUT_OF_RANGE" => QuestRefusal::OutOfRange,
        "NOT_SUPPORTED" => QuestRefusal::NotSupported,
        "CAPACITY_EXCEEDED" => QuestRefusal::CapacityExceeded,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(Ok(Some(refusal)))
}

pub(super) async fn record_refusal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
    binding: &[u8; 33],
    refusal: QuestRefusal,
) -> StoredResult<()> {
    let (id, ordinal) = request.cause.key();
    sqlx::query(
        "INSERT INTO game_character_grizzly_refusals(character_id,cause_kind,cause_id, \
         cause_ordinal,transition_key,request_binding,result_code) \
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7)",
    )
    .bind(character.as_bytes().as_slice())
    .bind(request.cause.kind())
    .bind(id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .bind(binding.as_slice())
    .bind(refusal.code())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Crystal names achievements in its NPC rewards. Resolve that display name against
/// the existing native catalogue; never manufacture an achievement key from a slug.
/// The native grant owner still decides whether the exact key is earnable or retired.
fn achievement_key(name: &str) -> StoredResult<&'static str> {
    use oteryn_gameplay_progression::achievement_catalogue::{Entry, SHARDS, parse_shards};
    static CATALOGUE: std::sync::LazyLock<Option<std::collections::BTreeMap<String, Entry>>> =
        std::sync::LazyLock::new(|| parse_shards(&SHARDS).ok());
    let catalogue = CATALOGUE.as_ref().ok_or(DurabilityError::InvalidStoredState)?;
    // Crystal's "Breaking The Ice" differs only in case from the admitted display
    // name. Ambiguous or missing names fail closed before any grant is committed.
    let mut matches = catalogue.iter().filter(|(_, entry)| entry.name.eq_ignore_ascii_case(name));
    let (key, _) = matches.next().ok_or(DurabilityError::InvalidStoredState)?;
    if matches.next().is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(key.as_str())
}

/// The successful quest receipt already exists in this same fenced transaction.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
    plan: &GrizzlyTransition,
    content_revision: &str,
) -> StoredResult<()> {
    if let Some(item) = &plan.item {
        if item.source_id != 9388 || item.count != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let (cause, ordinal) = request.cause.key();
        sqlx::query(
            "INSERT INTO game_character_grizzly_reward_entitlements(character_id,transition_key, \
             cause_id,cause_ordinal,item_source_id,claim_key,content_revision) \
             VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7)",
        )
        .bind(character.as_bytes().as_slice())
        .bind(&request.transition_key)
        .bind(cause.as_slice())
        .bind(ordinal.to_string())
        .bind(i32::try_from(item.source_id).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(HOLY_ICON_CLAIM)
        .bind(content_revision)
        .execute(&mut **tx)
        .await?;
    }
    if let Some(achievement) = &plan.achievement {
        let (cause, ordinal) = request.cause.key();
        let mut event = character.as_bytes().to_vec();
        event.extend_from_slice(&cause);
        event.extend_from_slice(&ordinal.to_be_bytes());
        event.extend_from_slice(request.transition_key.as_bytes());
        // Account achievements admit source events up to 64 bytes. Bind the whole
        // quest receipt identity without truncation to a fixed 32-byte event.
        let event: [u8; 32] = Sha256::new().chain_update(b"oteryn:grizzly-achievement:v1\0")
            .chain_update(event).finalize().into();
        super::super::account_achievement::grant_missing(
            tx,
            character,
            &[achievement_key(achievement)?],
            "oteryn:grizzly-adams",
            &event,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grizzly_boss_achievements_resolve_to_existing_canonical_keys() -> StoredResult<()> {
        let catalogue = rules::catalogue().map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut count = 0;
        for boss in &catalogue.bosses {
            if let Some(name) = &boss.achievement {
                assert!(achievement_key(name)?.starts_with("oteryn:achievement/"));
                count += 1;
            }
        }
        assert_eq!(count, 28);
        assert_eq!(achievement_key("Blood-Red Snapper")?, "oteryn:achievement/blood_red_snapper");
        assert_eq!(achievement_key("Breaking The Ice")?, "oteryn:achievement/breaking_the_ice");
        assert!(matches!(achievement_key("unknown Grizzly achievement"), Err(DurabilityError::InvalidStoredState)));
        Ok(())
    }
}
