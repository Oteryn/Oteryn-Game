//! A public projection of current, independently validated World policies, never inferred types.
use super::admission_authority_guards::{AdmissionGuardStore, Reader, read_scope};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use crate::foundation::{GameSessionId, GameSessionState, RuntimeScopeRefV1, WorldId};
use oteryn_protocol_oteryn::highscores::HighscoresQuery;
use oteryn_protocol_oteryn::highscores_catalogue::{self as wire, Entry, WorldType};
use sqlx::Row;
use std::collections::BTreeMap;
const SCOPES_SQL: &str = "SELECT scope_key FROM game_durability_admission_runtime_guards WHERE scope_key > $1 ORDER BY scope_key LIMIT 128";
const POLICY_SQL: &str = "SELECT world_type,source_pin,control_role,decision_identity FROM game_spell_field_world_policies WHERE world_id=encode($1,'hex')::uuid AND world_policy_revision=$2";

impl DurabilityRoot {
    pub async fn read_highscores_world_catalogue(&self) -> Result<Vec<Entry>, DurabilityError> {
        let root = self.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let store = AdmissionGuardStore::from_root(root.clone());
                    let mut worlds = BTreeMap::new();
                    let mut after: Vec<u8> = Vec::new();
                    loop {
                        let scopes = sqlx::query(SCOPES_SQL)
                            .bind(&after)
                            .fetch_all(&mut *tx)
                            .await?;
                        let Some(last) = scopes.last() else {
                            break;
                        };
                        after = last.try_get("scope_key")?;
                        for row in scopes {
                            let bytes: Vec<u8> = row.try_get("scope_key")?;
                            let mut reader = Reader::new(&bytes);
                            let scope = read_scope(&mut reader)?;
                            reader.finish()?;
                            let RuntimeScopeRefV1::Channel { world_id, .. } = scope else {
                                continue;
                            };
                            let guard = store
                                .load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
                                .await?
                                .ok_or(DurabilityError::InvalidStoredState)?;
                            let AdmissionAuthorityGuardStateV1::Runtime {
                                world_policy_revision,
                                ..
                            } = guard.state
                            else {
                                return Err(DurabilityError::InvalidStoredState);
                            };
                            // Catalogue metadata remains public while a World is offline.
                            // This type grants no live PvP authority.
                            let Some(policy) = sqlx::query(POLICY_SQL)
                                .bind(world_id.as_bytes().as_slice())
                                .bind(world_policy_revision)
                                .fetch_optional(&mut *tx)
                                .await?
                            else {
                                continue;
                            };
                            if policy.try_get::<String, _>("source_pin")?
                                != "99902524e052f37574194466c2949c576e4ab269"
                                || policy.try_get::<String, _>("control_role")?.is_empty()
                                || policy.try_get::<String, _>("decision_identity")?.is_empty()
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            let kind = match policy.try_get::<i16, _>("world_type")? {
                                1 => WorldType::Optional,
                                2 => WorldType::Open,
                                3 => WorldType::Hardcore,
                                _ => return Err(DurabilityError::InvalidStoredState),
                            };
                            if worlds
                                .insert(*world_id.as_bytes(), kind)
                                .is_some_and(|old| old != kind)
                                || worlds.len() > wire::MAX_WORLDS
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                        }
                    }
                    let entries = worlds
                        .into_iter()
                        .map(|(world, world_type)| {
                            Ok(Entry {
                                world: wire::WorldId::decode(&world)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                world_type,
                            })
                        })
                        .collect::<Result<Vec<_>, DurabilityError>>()?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(entries)
                })
            })
            .await
    }
}
pub async fn answer_session(
    root: &DurabilityRoot,
    served_world: WorldId,
    world: WorldId,
    session: GameSessionId,
    query: Option<(HighscoresQuery, Option<WorldType>)>,
) -> super::highscore_snapshot::reply::HighscoresReply {
    use super::highscore_snapshot::reply::{HighscoresReply, answer_highscores};
    if served_world != world {
        return HighscoresReply::Rejected;
    }
    let Ok((current, _)) = super::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session_at(session)
        .await
    else {
        return HighscoresReply::Rejected;
    };
    if current.session_state() == GameSessionState::Terminal || current.commit().world_id() != world
    {
        return HighscoresReply::Rejected;
    }
    let Some((query, kind)) = query else {
        return match root.read_highscores_world_catalogue().await {
            Ok(entries) => wire::encode_catalogue(&entries)
                .map_or(HighscoresReply::Rejected, HighscoresReply::Result),
            Err(_) => HighscoresReply::Rejected,
        };
    };
    if let Some(kind) = kind {
        let Ok(entries) = root.read_highscores_world_catalogue().await else {
            return HighscoresReply::Rejected;
        };
        let target = query.world.unwrap_or(world);
        if !entries
            .iter()
            .any(|entry| entry.world.as_bytes() == target.as_bytes() && entry.world_type == kind)
        {
            return HighscoresReply::Rejected;
        }
    }
    answer_highscores(root, world, current.commit().character_id(), query).await
}
