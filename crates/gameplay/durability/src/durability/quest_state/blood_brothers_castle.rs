//! The four castle unlocks share one Character transaction.
use super::*;
use oteryn_gameplay_quests::quest::{blood_brothers_castle as rite, world_callbacks::Position};

const TRANSITION: &str = "oteryn:quest-transition/crystalserver/quest/u8_4/blood_brothers/castle_entrance_stg/movement_1";

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "four fenced participants in one source movement transaction"
    )]
    pub async fn commit_blood_brothers_castle(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &super::spell_owner_commit::SpellLanePermit,
        entered: Position,
        members: [(CurrentCharacterGameplayFence, CommandRef); 4],
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<bool> {
        let scope = members[0].0.runtime_scope;
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
            || members.iter().any(|(fence, command)| {
                fence.runtime_scope != scope || command.game_session_id() != fence.game_session_id
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit
            .check_scope(scope)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut order = [0, 1, 2, 3];
        order.sort_by_key(|&index| members[index].0.character_id);
        if order
            .windows(2)
            .any(|p| members[p[0]].0.character_id == members[p[1]].0.character_id)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let requests = members.map(|(_, command)| QuestTransitionRequest {
            transition_key: TRANSITION.into(),
            cause: QuestCause::Step(command),
        });
        let pass = self.try_issue_semantic_pass()?;
        pass.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let Some(transition) = catalogue.transition(TRANSITION) else {
                return Ok(Err(CharacterProgressionError::InvalidInput));
            };
            let Some(hash) = catalogue.definition_hash(&transition.quest) else {
                return Err(DurabilityError::InvalidStoredState);
            };
            for index in order {
                lock_receipt_key(&mut tx, members[index].0.character_id, &requests[index]).await?;
            }
            let mut players = [None; 4];
            let mut existing = [false; 4];
            for index in order {
                let (fence, _) = members[index];
                let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                    Ok(root) => root,
                    Err(error) => return Ok(Err(error)),
                };
                if root.content_revision != catalogue.content_revision() {
                    return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                }
                let id = fence.character_id.as_bytes().as_slice();
                let locked: Option<(i64, Option<i64>, bool)> = sqlx::query_as(
                    "SELECT coalesce((SELECT value FROM game_character_quest_tracks
                         WHERE character_id=p.character_id AND track_key=$2),0),
                         (SELECT value FROM game_character_quest_tracks
                         WHERE character_id=p.character_id AND track_key=$3),
                         EXISTS(SELECT 1 FROM game_quest_callback_carried(p.character_id,encode($4,'hex')::uuid) c
                           JOIN game_item_instances i ON i.item_instance_id=c.item
                           WHERE i.world_id=encode($4,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0
                           AND i.definition_family='Item' AND i.definition_production_key=$5)
                     FROM game_character_progression_state p WHERE p.character_id=encode($1,'hex')::uuid
                       AND p.character_revision=$6::text::numeric AND p.content_revision=$7
                       AND NOT EXISTS(SELECT 1 FROM game_character_pending_respawns r WHERE r.character_id=p.character_id)")
                    .bind(id).bind(rite::MISSION).bind(rite::ENTRANCE).bind(scope.world_id().as_bytes().as_slice())
                    .bind(rite::CHARGED_CRYSTAL).bind(root.revision.to_string()).bind(catalogue.content_revision())
                    .fetch_optional(&mut *tx).await?;
                let Some((mission, entrance, has_charged_crystal)) = locked else {
                    return Ok(Ok(false));
                };
                existing[index] = entrance.is_some();
                players[index] = Some(rite::Participant {
                    mission, entrance: entrance.unwrap_or(-1), has_charged_crystal,
                });
            }
            let Some(plan) = rite::plan(entered, players) else { return Ok(Ok(false)); };
            for index in order {
                if !plan[index].unlock {
                    continue;
                }
                let (fence, _) = members[index];
                let Some(player) = players[index] else { return Err(DurabilityError::InvalidStoredState); };
                let id = fence.character_id.as_bytes().as_slice();
                let stored: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT definition_hash FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2"
                ).bind(id).bind(&transition.quest).fetch_optional(&mut *tx).await?;
                if stored.as_deref().is_some_and(|stored| stored != hash)
                    || stored.is_none() && !capacity::new_state_fits(&mut tx, fence.character_id).await?
                    || !capacity::tracks_fit(&mut tx, fence.character_id, &[rite::ENTRANCE], usize::from(existing[index])).await?
                {
                    return Ok(Ok(false));
                }
                let request = &requests[index];
                let original = fence.expected_character_revision.get();
                let revision = original.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                for query in [
                    "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                    "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                ] {
                    if sqlx::query(query).bind(id).bind(revision.to_string()).bind(original.to_string())
                        .execute(&mut *tx).await?.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                }
                let (cause, ordinal) = request.cause.key();
                sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at)
                    SELECT character_id,$2,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9,false,ARRAY[$10]::text[],ARRAY[$11]::bigint[],ARRAY[1]::bigint[],$12::text::numeric,$13::text::numeric,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,floor(extract(epoch FROM statement_timestamp())*1000)::bigint
                    FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id).bind(request.cause.kind()).bind(cause.as_slice()).bind(ordinal.to_string())
                    .bind(TRANSITION).bind(&transition.quest).bind(request_binding(fence.character_id, request).as_slice())
                    .bind(catalogue.content_revision()).bind(hash.as_slice()).bind(rite::ENTRANCE).bind(player.entrance)
                    .bind(original.to_string()).bind(revision.to_string()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
                     VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5::text::numeric,$5::text::numeric)
                     ON CONFLICT(character_id,quest_key) DO UPDATE SET committed_character_revision=EXCLUDED.committed_character_revision")
                     .bind(id).bind(&transition.quest).bind(catalogue.content_revision()).bind(hash.as_slice())
                     .bind(revision.to_string()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
                     VALUES(encode($1,'hex')::uuid,$2,$3,1,$4::text::numeric)
                     ON CONFLICT(character_id,track_key) DO UPDATE SET value=1,committed_character_revision=EXCLUDED.committed_character_revision")
                     .bind(id).bind(rite::ENTRANCE).bind(&transition.quest).bind(revision.to_string())
                     .execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(true))
        })).await?
    }
}
