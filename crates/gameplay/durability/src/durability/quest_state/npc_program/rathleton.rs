//! Rathleton Quest ballots, in the NPC transaction (TibiaWiki Rathleton_Quest/Spoiler oldid 1141625).
//!
//! A confirmed Marvin cast spends one Oramond voting point (Crystal 201189f8
//! data-global/npc/marvin.lua:78-81 casts exactly one) and counts it once in the
//! World's fortnight tally. Replays return the stored NPC receipt before this
//! runs, and the ballot key is the command, so a vote is never counted twice.
use super::*;

/// Crystal 201189f8 storages.lua:2184 `OramondQuest.VotingPoints`, earned by the Oramond tasks.
const VOTING_POINTS: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const CAST: &str = "oteryn:quest-npc/authored-rathleton/marvin/";

/// Option code = topic category * 10 + option, as in the authored Marvin topics.
const OPTIONS: [(&str, i16); 12] = [
    ("boss/bullwark/cast", 31),
    ("boss/glooth-fairy/cast", 33),
    ("boss/lisa/cast", 32),
    ("dungeon/golem/cast", 21),
    ("dungeon/minotaur/cast", 22),
    ("dungeon/wrath/cast", 23),
    ("raid/mechanical-creature/cast", 42),
    ("raid/minotaurs/cast", 41),
    ("raid/wild-life/cast", 43),
    ("shortcut/catacomb/cast", 13),
    ("shortcut/fungi/cast", 12),
    ("shortcut/minotaur/cast", 11),
];

fn option(transition_key: &str) -> Option<i16> {
    let tail = transition_key.strip_prefix(CAST)?;
    OPTIONS
        .iter()
        .find(|(key, _)| *key == tail)
        .map(|(_, code)| *code)
}

/// `Ok(false)` when the Character holds no voting point; the caller rolls back.
pub(super) async fn cast(
    tx: &mut super::super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    request: &NpcProgramRequest,
    committed: &str,
) -> StoredResult<bool> {
    let Some(option) = option(&request.transition_key) else {
        return Ok(true);
    };
    let character = fence.character_id.as_bytes();
    let spent = sqlx::query("UPDATE game_character_quest_tracks SET value=value-1,committed_character_revision=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value>=1")
        .bind(character.as_slice()).bind(VOTING_POINTS).bind(committed).execute(&mut ***tx).await?.rows_affected();
    if spent != 1 {
        return Ok(false);
    }
    let world = fence.runtime_scope.world_id();
    // Fortnights start on Monday 2000-01-03 at the repository's Berlin 06:00 server-save day boundary.
    let period: i32 = sqlx::query_scalar("INSERT INTO game_rathleton_election_ballots(character_id,game_session_id,command_id,world_id,period,option_code) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,(((statement_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours')::date-date '2000-01-03')/14,$5) RETURNING period")
        .bind(character.as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(world.as_bytes().as_slice()).bind(option).fetch_one(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_rathleton_election_tallies(world_id,period,option_code,votes) VALUES(encode($1,'hex')::uuid,$2,$3,1) ON CONFLICT(world_id,period,option_code) DO UPDATE SET votes=game_rathleton_election_tallies.votes+1")
        .bind(world.as_bytes().as_slice()).bind(period).bind(option).execute(&mut ***tx).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_authored_cast_rule_has_one_option_code() -> Result<(), String> {
        let program: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../../content/dialogues/quest-rules/quest-authored-rathleton.json"
        ))
        .map_err(|e| e.to_string())?;
        let casts = program["rules"]
            .as_array()
            .ok_or("rules")?
            .iter()
            .filter_map(|rule| rule["key"].as_str())
            .filter(|key| key.ends_with("/cast"))
            .map(|key| super::option(key).ok_or(format!("unmapped cast {key}")))
            .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
        assert_eq!(casts.len(), super::OPTIONS.len());
        assert_eq!(super::option("oteryn:quest-npc/authored-rathleton/marvin/promotion/squire"), None);
        Ok(())
    }
}
