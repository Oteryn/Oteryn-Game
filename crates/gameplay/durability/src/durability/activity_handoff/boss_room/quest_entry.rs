//! Recheck each source boss's Quest predicates under the admission transaction's
//! Character root locks. The room catalogue cannot lower these prerequisites.
use super::*;
use crate::durability::quest_state::{rotten_blood, soul_war};
use oteryn_gameplay_quests::quest::{rotten_blood as rotten, soul_war as soul};

pub(super) async fn validate(
    tx: &mut Transaction<'_, Postgres>,
    creature: &str,
    character: crate::domain::CharacterId,
    level: i64,
    participants: usize,
    now_ms: i64,
) -> Result<Option<ActivityRefusal>> {
    let soul_final = creature == "oteryn:creature.goshnar_s_megalomania_purple"
        || soul::SoulWarBoss::from_creature(creature) == Some(soul::SoulWarBoss::Megalomania);
    let soul_boss = soul_final || soul::SoulWarBoss::from_creature(creature).is_some();
    let rotten_boss = rotten::RottenBloodBoss::from_creature_key(creature);
    let primal_boss = creature == "oteryn:creature.the_primal_menace";
    if !soul_boss && rotten_boss.is_none() && !primal_boss {
        return Ok(None);
    }
    if participants > 5 {
        return Ok(Some(ActivityRefusal::PartyCount));
    }
    // Crystal 201189f actions/bosses_levers/the_primal_manace.lua:6-12.
    // An alias in the room catalogue cannot bypass the source's level 500 gate.
    let minimum_level = if primal_boss {
        500
    } else if soul_boss {
        soul::MINIMUM_LEVEL
    } else {
        rotten::REQUIRED_LEVEL
    };
    if level < i64::from(minimum_level) {
        return Ok(Some(ActivityRefusal::Level));
    }
    if primal_boss {
        return Ok(None);
    }
    if soul_boss {
        // The existing read projection expires taints without manufacturing a
        // progression write. Use the same database clock here, before admission.
        let state = soul_war::storage::load(tx, character)
            .await?
            .expired(now_ms / 1_000);
        if !state.access() || (soul_final && !state.chain_ready()) {
            return Ok(Some(ActivityRefusal::AuthorityRejected));
        }
    } else {
        let state = rotten_blood::read_state(tx, character).await?;
        if state
            .validate_entry(
                u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
                rotten_boss,
            )
            .is_err()
        {
            return Ok(Some(ActivityRefusal::AuthorityRejected));
        }
    }
    Ok(None)
}
