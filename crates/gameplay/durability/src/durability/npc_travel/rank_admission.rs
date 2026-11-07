//! Rathleton admission conditions join the existing fenced trade/travel owners.
//! No rank, vote or arrival is granted by these read-only predicates.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_content::content::npc_catalogue::travel::TravelRoute;
use oteryn_gameplay_quests::quest::rathleton;

async fn rank(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    minimum: i64,
) -> Result<bool, DurabilityError> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks \
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND value>=$3)",
    )
    .bind(character.as_bytes().as_slice())
    .bind(rathleton::RANK_TRACK)
    .bind(minimum)
    .fetch_one(&mut ***tx)
    .await
    .map_err(Into::into)
}

/// Call only after the trade owner's current Character/session fence.
pub(in crate::durability) async fn trade(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    npc: &str,
) -> Result<bool, DurabilityError> {
    let minimum = match npc {
        "oteryn:npc.alaistar" => 1,
        "oteryn:npc.flint" => 2,
        _ => return Ok(true),
    };
    rank(tx, character, minimum).await
}

pub(super) async fn epoch(
    tx: &mut Type2Transaction<'_>,
    world: WorldId,
) -> Result<Option<u64>, DurabilityError> {
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_active \
         WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut ***tx)
    .await?;
    epoch
        .map(|epoch| {
            epoch
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)
        })
        .transpose()
}

type MarvinAdmission = (i64, Option<&'static str>, (i32, i32, i16));

fn marvin_route(key: &str) -> Option<MarvinAdmission> {
    // The three dungeon targets are Crystal9f5 voting_oramond.lua:2-8.
    // SHA25694166b74d257fdb77043ae036d5f0272d6438fdc6fafbda7a9b63947d2586133.
    // Marvin passages are the owner's authored access fallback; their destination
    // and option are fixed before the live election/rank check, never supplied by a vote.
    Some(match key {
        "dungeon_minotaurs" => (1, Some("dungeon/minotaurs"), (31254, 32604, 9)),
        "dungeon_golems" => (1, Some("dungeon/golems"), (31061, 32605, 9)),
        "dungeon_wrathofevil" => (1, Some("dungeon/wrathofevil"), (33459, 31715, 9)),
        // First Strange Pipe: world-npc.xml:2949-2950; adjacent native ground.
        // SHA2563cc9a769cda450a0e5e0c60bcc65973031efe8b94e5ea6a4256473d3e45fd306.
        "hero_of_rathleton" => (4, None, (33533, 31951, 14)),
        _ => return None,
    })
}

/// Quote and commit both enter here after their current gameplay/admission fence.
pub(super) async fn travel(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    route: &TravelRoute,
) -> Result<bool, DurabilityError> {
    match (route.service.as_str(), route.route.key.as_str()) {
        ("oteryn:service.travel.captain_gulliver", "issavi")
        | ("oteryn:service.travel.captain_seahorse", "oramond") => {
            return rank(tx, fence.character_id, 3).await;
        }
        ("oteryn:service.travel.marvin", _) => {}
        _ => return Ok(true),
    }
    let Some((minimum, ballot, destination)) = marvin_route(&route.route.key) else {
        return Ok(false);
    };
    let actual = &route.route.destination;
    if (actual.x, actual.y, actual.floor) != destination
        || route.route.price != 0
        || !route.route.premium
        || !rank(tx, fence.character_id, minimum).await?
    {
        return Ok(false);
    }
    let Some(ballot) = ballot else {
        return Ok(true);
    };
    let world = fence.runtime_scope.world_id();
    let Some(epoch) = epoch(tx, world).await? else {
        return Ok(false);
    };
    Ok(
        crate::durability::world_event::changes::shared(tx, world, epoch)
            .await?
            .is_some_and(|state| {
                state.election_winners.get("dungeon").map(String::as_str) == Some(ballot)
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rathleton_admission_binds_each_ballot_to_its_source_destination() {
        assert_eq!(
            marvin_route("dungeon_minotaurs"),
            Some((1, Some("dungeon/minotaurs"), (31254, 32604, 9)))
        );
        assert_eq!(
            marvin_route("dungeon_golems"),
            Some((1, Some("dungeon/golems"), (31061, 32605, 9)))
        );
        assert_eq!(
            marvin_route("dungeon_wrathofevil"),
            Some((1, Some("dungeon/wrathofevil"), (33459, 31715, 9)))
        );
        assert_eq!(
            marvin_route("hero_of_rathleton"),
            Some((4, None, (33533, 31951, 14)))
        );
        assert!(marvin_route("dungeon/arbitrary").is_none());
    }
}
