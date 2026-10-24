//! OFFLINE0 fields carried by the existing build/death receipt chain (0180).
use super::{DurabilityError, character_build::BuildCause};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfflineTrainingState {
    pub stamina_minutes: u16,
    pub pool_seconds: u32,
    /// Native build family: 0 magic, 1 fist, 2 club, 3 sword, 4 axe, 5 distance.
    pub skill: Option<usize>,
}
impl Default for OfflineTrainingState {
    fn default() -> Self {
        Self {
            stamina_minutes: 2520,
            pool_seconds: 43200,
            skill: None,
        }
    }
}
impl OfflineTrainingState {
    pub(super) fn decode(row: &sqlx::postgres::PgRow) -> Result<Self, DurabilityError> {
        let skill: i16 = row.try_get("offline_skill")?;
        Ok(Self {
            stamina_minutes: u16::try_from(row.try_get::<i32, _>("stamina_minutes")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            pool_seconds: u32::try_from(row.try_get::<i32, _>("offline_pool_s")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            skill: if skill == -1 {
                None
            } else {
                Some(usize::try_from(skill).map_err(|_| DurabilityError::InvalidStoredState)?)
            },
        })
    }
    pub(super) fn skill_code(self) -> Result<i16, DurabilityError> {
        self.skill.map_or(Ok(-1), |skill| {
            i16::try_from(skill).map_err(|_| DurabilityError::InvalidStoredState)
        })
    }
    pub(super) fn valid(self) -> bool {
        self.stamina_minutes <= 2520
            && self.pool_seconds <= 43200
            && self.skill.is_none_or(|skill| skill <= 5)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct OfflineChange {
    pub before: OfflineTrainingState,
    pub after: OfflineTrainingState,
}

pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
) -> Result<OfflineTrainingState, DurabilityError> {
    sqlx::query("SELECT stamina_minutes, offline_pool_s, offline_skill FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).fetch_optional(&mut **tx).await?
        .map(|row| OfflineTrainingState::decode(&row)).transpose().map(|state| state.unwrap_or_default())
}

pub(super) fn admits(cause: BuildCause, change: OfflineChange) -> bool {
    if !change.before.valid() || !change.after.valid() {
        return false;
    }
    match cause {
        BuildCause::Logout => {
            change.before.stamina_minutes == change.after.stamina_minutes
                && change.after.pool_seconds >= change.before.pool_seconds
        }
        BuildCause::OfflineSettlement => {
            change.after.skill.is_none()
                && change.after.stamina_minutes >= change.before.stamina_minutes
        }
        _ => change.before == change.after,
    }
}

/// Recovery verifies the same offline chain as each deferred write guard, in bounded pages.
pub(super) async fn verify(tx: &mut Transaction<'_, Postgres>) -> Result<(), DurabilityError> {
    let mut after = String::from("00000000-0000-0000-0000-000000000000");
    loop {
        let ids:Vec<String>=sqlx::query_scalar("SELECT character_id::text FROM game_character_build_state WHERE character_id>$1::uuid ORDER BY character_id LIMIT 256")
            .bind(&after).fetch_all(&mut **tx).await?;
        for id in &ids {
            sqlx::query("SELECT game_verify_training_integrity($1::uuid)")
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
        let Some(last) = ids.last() else {
            return Ok(());
        };
        after = last.clone();
        if ids.len() < 256 {
            return Ok(());
        }
    }
}
