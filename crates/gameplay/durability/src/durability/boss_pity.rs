//! WU26 T188/T197: durable Moonsilver first-drop pity (capability 492), one row per
//! character and boss. The rule is `oteryn_gameplay_encounter_rules::boss::pity`.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_runtime_scope_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::boss::pity::PityState;
use sqlx::{Postgres, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

/// The largest reward census a pity read accepts (the reward contributor bound).
const PITY_PARTICIPANTS_MAX: usize = 50;

impl DurabilityRoot {
    /// The pity bonus (permille of base) each participant rolls with, read once under the
    /// current scope owner before the reward roll is frozen. Participants without an active
    /// tracker read 0; the settlement transaction then applies the victory to the stored row.
    pub async fn boss_pity_bonuses(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        pity_key: String,
        participants: Vec<[u8; 16]>,
    ) -> Result<Vec<([u8; 16], u16)>> {
        if participants.len() > PITY_PARTICIPANTS_MAX {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut out = Vec::with_capacity(participants.len());
                    for character in participants {
                        let state = read_state(&mut tx, character, &pity_key, false).await?;
                        out.push((character, state.bonus_permille()));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(out)
                })
            })
            .await
    }
}

/// One character's stored tracker; no row is a character without a recorded victory.
pub(crate) async fn read_state(
    tx: &mut Transaction<'_, Postgres>,
    character: [u8; 16],
    pity_key: &str,
    lock: bool,
) -> Result<PityState> {
    let sql = if lock {
        "SELECT kills_without_drop,first_drop_received FROM game_character_boss_pity WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE"
    } else {
        "SELECT kills_without_drop,first_drop_received FROM game_character_boss_pity WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2"
    };
    let row: Option<(i16, bool)> = sqlx::query_as(sql)
        .bind(character.as_slice())
        .bind(pity_key)
        .fetch_optional(&mut **tx)
        .await?;
    let Some((kills, first_drop_received)) = row else {
        return Ok(PityState::default());
    };
    Ok(PityState {
        kills_without_drop: u16::try_from(kills)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        first_drop_received,
    })
}

/// Applies one boss victory per recipient inside the reward settlement transaction, which
/// has already proved every recipient's captured session and Character lease. `outcomes`
/// is ascending by character, so sibling settlements lock pity rows in one order.
pub(crate) async fn record_victories_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    pity_key: &str,
    outcomes: &[([u8; 16], bool)],
) -> Result<()> {
    for (character, dropped) in outcomes {
        let next = read_state(tx, *character, pity_key, true)
            .await?
            .after_victory(*dropped);
        sqlx::query("INSERT INTO game_character_boss_pity(character_id,boss_key,kills_without_drop,first_drop_received) VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT(character_id,boss_key) DO UPDATE SET kills_without_drop=excluded.kills_without_drop,first_drop_received=excluded.first_drop_received")
            .bind(character.as_slice())
            .bind(pity_key)
            .bind(i16::try_from(next.kills_without_drop).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(next.first_drop_received)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// The capability 492 tracker of one boss window, or `None` for a boss outside the pity set.
pub(crate) async fn window_tracker(
    tx: &mut Transaction<'_, Postgres>,
    character: [u8; 16],
    boss_key: &str,
    boss_creature_key: &str,
) -> Result<Option<oteryn_protocol_oteryn::boss_difficulty::PityTracker>> {
    use oteryn_gameplay_encounter_rules::boss::pity::pity_boss_key;
    let Some(pity_key) = pity_boss_key(boss_key).or_else(|| pity_boss_key(boss_creature_key))
    else {
        return Ok(None);
    };
    let state = read_state(tx, character, &pity_key, false).await?;
    Ok(Some(oteryn_protocol_oteryn::boss_difficulty::PityTracker {
        kills_without_drop: state.kills_without_drop,
        bonus_permille: state.bonus_permille(),
        tracker_active: state.tracker_active(),
    }))
}
