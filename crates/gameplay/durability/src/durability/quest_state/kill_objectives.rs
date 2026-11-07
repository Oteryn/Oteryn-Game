//! Native death descendants join the existing XP transaction and shared Quest persistence.
use super::*;
use oteryn_gameplay_quests::quest::kill_objectives::KillObjective;

/// Captured only from a committed native death and its coherent Character quest copy.
/// No wire command can construct the sealed death identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeObjectiveDeath {
    death: Vec<u8>,
    character: CharacterId,
    position: (i32, i32, i16),
    observed_revision: u64,
    phase: i64,
    spawn_origin: Option<(i32, i32, i16)>,
}

impl NativeObjectiveDeath {
    pub fn capture(
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        target: crate::foundation::ExactActorRef,
        character: CharacterId,
        observed_revision: u64,
        phase: i64,
    ) -> Option<Self> {
        if observed_revision == 0 {
            return None;
        }
        let (death, at) = runtime.borrow_combat_death().projected_death(target).ok()?;
        Some(Self {
            death: super::super::party_kill_xp::encode_death_key(death),
            character,
            position: (at.x, at.y, at.floor),
            observed_revision,
            phase,
            spawn_origin: None,
        })
    }

    /// Exact-origin goals additionally bind the immutable native spawn cell.
    pub fn capture_goal(
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        target: crate::foundation::ExactActorRef,
        character: CharacterId,
        observed_revision: u64,
    ) -> Option<Self> {
        let origin = runtime.creature_death_spawn_cell(target)?;
        let mut source = Self::capture(runtime, target, character, observed_revision, 0)?;
        source.spawn_origin = Some((origin.x, origin.y, origin.floor));
        Some(source)
    }

    fn occurrence(
        &self,
    ) -> Result<super::super::character_progression::ExperienceRewardOccurrence> {
        let hash = Sha256::new()
            .chain_update(b"oteryn:quest-native-death:v1")
            .chain_update(&self.death)
            .chain_update(self.character.as_bytes())
            .finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&hash[..16]);
        id[6] = (id[6] & 15) | 0x70;
        id[8] = (id[8] & 63) | 0x80;
        super::super::character_progression::ExperienceRewardOccurrence::from_bytes(id)
    }

    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        let hash = Sha256::new()
            .chain_update(*binding)
            .chain_update(&self.death)
            .chain_update(self.character.as_bytes())
            .chain_update(self.position.0.to_be_bytes())
            .chain_update(self.position.1.to_be_bytes())
            .chain_update(self.position.2.to_be_bytes())
            .chain_update(self.observed_revision.to_be_bytes())
            .chain_update(self.phase.to_be_bytes())
            .finalize();
        binding[1..].copy_from_slice(&hash);
        if let Some((x, y, floor)) = self.spawn_origin {
            let hash = Sha256::new()
                .chain_update(*binding)
                .chain_update(x.to_be_bytes())
                .chain_update(y.to_be_bytes())
                .chain_update(floor.to_be_bytes())
                .finalize();
            binding[1..].copy_from_slice(&hash);
        }
        Ok(())
    }
}

enum Binding {
    Objective(KillObjective),
    Goal(&'static oteryn_gameplay_quests::quest::native_kill_goals::Goal),
}

impl Binding {
    fn resolve(key: &str) -> Result<Self> {
        if let Some(rule) = oteryn_gameplay_quests::quest::kill_objectives::binding(key) {
            return Ok(Self::Objective(rule));
        }
        oteryn_gameplay_quests::quest::native_kill_goals::load()
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .iter()
            .find(|goal| goal.key == key)
            .map(Self::Goal)
            .ok_or(CharacterProgressionError::InvalidInput)
    }

    fn quest(&self) -> &str {
        match self {
            Self::Objective(rule) => rule.quest,
            Self::Goal(goal) => &goal.quest,
        }
    }

    fn creatures(&self, source: &NativeObjectiveDeath) -> Vec<&str> {
        match self {
            Self::Objective(rule)
                if source.phase == rule.phase_before
                    && rule.contains(source.position.0, source.position.1, source.position.2) =>
            {
                vec![rule.creature]
            }
            Self::Goal(goal) if source.phase == 0 => goal
                .origins
                .iter()
                .filter_map(|origin| {
                    (source.spawn_origin
                        == Some((
                            i32::from(origin.x),
                            i32::from(origin.y),
                            i16::from(origin.floor),
                        )))
                    .then_some(origin.creature.as_str())
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Called by the existing native XP transaction, with its Character slots and
/// scope/recovery authority held. A new write must observe that transaction's
/// own death audit; an exact receipt replay makes no fresh grant.
pub async fn apply_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    transition_key: &str,
    source: &NativeObjectiveDeath,
    catalogue: &QuestStateCatalogue,
) -> Result<Option<CommittedQuestTransition>> {
    let rule = Binding::resolve(transition_key)?;
    let request = QuestTransitionRequest {
        transition_key: transition_key.into(),
        cause: QuestCause::CreatureDeath(source.occurrence()?),
    };
    let mut binding = request_binding(fence.character_id, &request);
    source.bind(&mut binding)?;
    lock_receipt_key(tx, fence.character_id, &request).await?;
    if let Some(row) = load_receipt(tx, fence.character_id, &request).await? {
        if row
            .try_get::<Vec<u8>, _>("request_binding")
            .map_err(DurabilityError::from)?
            != binding
        {
            return Err(CharacterProgressionError::ConflictingOccurrence);
        }
        return Ok(Some(decode_receipt(&row)?));
    }
    let row = sqlx::query("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    let mut current = fence;
    current.expected_character_revision =
        CharacterRevision::new(numeric_u64(&row, "character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    let root = assert_gameplay_fence(tx, &current, node).await??;
    if !authorize(tx, &current, &request, &rule, source).await? {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let progression = sqlx::query("SELECT character_revision::text, level, total_experience, profile_revision, ruleset_revision, content_revision, simulation_revision, evidence_revision, declaration_revision, policy_revision, reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if numeric_u64(&progression, "character_revision")? != root.revision
        || !state_matches_root(&progression, &root)
        || progression
            .try_get::<String, _>("content_revision")
            .map_err(DurabilityError::from)?
            != catalogue.content_revision()
    {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let state = sqlx::query("SELECT pinned_content_revision, definition_hash, completed_character_revision IS NOT NULL AS completed FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rule.quest()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    let hash: [u8; 32] = state
        .try_get::<Vec<u8>, _>("definition_hash")
        .map_err(DurabilityError::from)?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if catalogue.definition_hash(rule.quest()) != Some(hash)
        && !state
            .try_get::<bool, _>("completed")
            .map_err(DurabilityError::from)?
    {
        return Err(CharacterProgressionError::ProgressionContextMismatch);
    }
    let pin: String = state
        .try_get("pinned_content_revision")
        .map_err(DurabilityError::from)?;
    let transition = catalogue
        .transition(transition_key)
        .ok_or(CharacterProgressionError::InvalidInput)?;
    if transition.quest != rule.quest() || transition.experience.is_some() || transition.completes {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let mut keys: Vec<&str> = transition
        .effects
        .iter()
        .map(|effect| effect.track.as_str())
        .collect();
    keys.sort_unstable();
    keys.dedup();
    let rows = sqlx::query("SELECT track_key, quest_key, value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    let mut stored = BTreeMap::new();
    for row in &rows {
        if row
            .try_get::<String, _>("quest_key")
            .map_err(DurabilityError::from)?
            != rule.quest()
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        stored.insert(
            row.try_get::<String, _>("track_key")
                .map_err(DurabilityError::from)?,
            row.try_get("value").map_err(DurabilityError::from)?,
        );
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
            .fetch_one(&mut **tx)
            .await
            .map_err(DurabilityError::from)?;
    let changes = match catalogue.evaluate(transition, &stored, now) {
        Ok(changes) => changes,
        Err(QuestRefusal::StageMismatch | QuestRefusal::OutOfRange) => return Ok(None),
        Err(_) => return Err(CharacterProgressionError::InvalidInput),
    };
    if !capacity::tracks_fit(tx, fence.character_id, &keys, rows.len()).await? {
        return Err(CharacterProgressionError::InvalidInput);
    }
    super::persist::persist(
        tx,
        super::persist::QuestWrite {
            character: fence.character_id,
            root_revision: root.revision,
            request: &request,
            transition,
            pin: &pin,
            hash: &hash,
            changes: &changes,
            binding: &binding,
            state_exists: true,
        },
    )
    .await?;
    let row = load_receipt(tx, fence.character_id, &request)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(decode_receipt(&row)?))
}

async fn authorize(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    rule: &Binding,
    source: &NativeObjectiveDeath,
) -> StoredResult<bool> {
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        || source.character != fence.character_id
        || rule.creatures(source).is_empty()
        || !super::super::party_kill_xp::death_matches_fence(&source.death, fence)
        || !matches!(request.cause, QuestCause::CreatureDeath(id)
            if source.occurrence().is_ok_and(|expected| id == expected))
    {
        return Ok(false);
    }
    // A shared-XP bystander is never the personal objective principal. Zero-stamina
    // deaths still have this native member audit and do not require an XP grant.
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_kill_xp_receipts d \
        JOIN game_party_kill_xp_members m USING(death_key) \
        JOIN game_character_quest_states q ON q.character_id=m.character_id \
        WHERE d.death_key=$1 AND d.created_xact_id=pg_current_xact_id() AND d.principal_id=encode($2,'hex')::uuid \
        AND m.character_id=d.principal_id AND d.creature_key=ANY($3::text[]) AND q.quest_key=$4 \
        AND q.started_character_revision <= $5::text::numeric)")
        .bind(&source.death).bind(fence.character_id.as_bytes().as_slice())
        .bind(rule.creatures(source)).bind(rule.quest()).bind(source.observed_revision.to_string())
        .fetch_one(&mut **tx).await.map_err(DurabilityError::from)
}
