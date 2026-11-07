//! A display withdrawal is neither VOID nor zero standings. Authority survives it.
use super::*;
#[allow(clippy::too_many_arguments)] // Presentation changes retain explicit authority coordinates.
pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    c: &Control,
    family: &str,
    world: &str,
    subject: &str,
    requested: bool,
    tombstone: Option<&str>,
    binding: &[u8],
    old: Option<&Snapshot>,
    now: i64,
) -> Result<Option<Disposition>> {
    let row=sqlx::query("SELECT display_withdrawn,record,record_sha256,question_key FROM game_t94_subjects WHERE family=$1 AND world_id=$2::uuid AND subject_id=$3::uuid FOR UPDATE")
  .bind(family).bind(world).bind(subject).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(if requested {
            Some(Disposition::Pending)
        } else {
            None
        });
    };
    let private: Vec<u8> = row.try_get("record")?;
    if private.is_empty() {
        return Ok(Some(Disposition::Pending));
    }
    if row.try_get::<Vec<u8>, _>("record_sha256")? != Sha256::digest(&private).as_slice() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let privacy: bool = if family == "guild_season" {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_t94_privacy_watch w LEFT JOIN game_guilds g ON g.guild_id=w.guild_id AND g.world_id=w.world_id WHERE w.world_id=$1::uuid AND w.season_id=$2::uuid AND (g.guild_id IS NULL OR g.state=4) AND w.public_name IS DISTINCT FROM $3)")
  .bind(world).bind(subject).bind(tombstone).fetch_one(&mut **tx).await?
    } else {
        false
    };
    let current: bool = row.try_get("display_withdrawn")?;
    if privacy && !requested && tombstone.is_some() && family == "guild_season" {
        let stored: SeasonRecord = serde_json::from_slice(&private).map_err(invalid)?;
        if stored.state == "OPEN" {
            if current {
                let Some(mut virtual_old) = old.cloned() else {
                    return Ok(Some(Disposition::Pending));
                };
                if let Snapshot::Season(s) = &mut virtual_old {
                    s.season = vec![stored];
                }
                let rule: SeasonBinding = serde_json::from_slice(binding).map_err(invalid)?;
                return Ok(Some(
                    season::produce(tx, c, world, subject, &rule, Some(&virtual_old), now).await?,
                ));
            }
            return Ok(None);
        }
    }
    let withdrawn = requested || privacy;
    if withdrawn == current {
        return Ok(if withdrawn {
            Some(Disposition::Pending)
        } else {
            None
        });
    }
    let Some(old) = old else {
        return Ok(Some(Disposition::Pending));
    };
    let mut snapshot = old.clone();
    let next = revision(Some(old))?;
    match &mut snapshot {
        Snapshot::Event(s) => {
            s.projection_revision = next;
            s.source_observed_at = now.to_string();
            s.event = if withdrawn {
                Vec::new()
            } else {
                vec![serde_json::from_slice(&private).map_err(invalid)?]
            };
        }
        Snapshot::Season(s) => {
            s.projection_revision = next;
            s.source_observed_at = now.to_string();
            s.season = if withdrawn {
                Vec::new()
            } else {
                vec![serde_json::from_slice(&private).map_err(invalid)?]
            };
        }
    }
    let question = row
        .try_get::<Option<String>, _>("question_key")?
        .unwrap_or_default();
    Ok(Some(
        persist(tx, &snapshot, Some(old), &question, c, now, Some(withdrawn)).await?,
    ))
}
