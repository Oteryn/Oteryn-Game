//! Settle the two personal palace milestones in either completion order.
use super::{CharacterId, DurabilityError};

pub(super) const REPORT: &str = "oteryn:quest-npc/lost-brother/tarun/report";
pub(super) const ORCHID: &str = "oteryn:quest-transition/native-kills/engenc-asura-orchid";

/// Both callers have written their track under the character root lock.
pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<(), DurabilityError> {
    let event: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(account_id) || uuid_send(character_id) FROM game_character_roots
          WHERE character_id = encode($1,'hex')::uuid
            AND 2 = (SELECT count(*) FROM game_character_quest_tracks
          WHERE character_id = encode($1,'hex')::uuid
            AND ((track_key = 'oteryn:quest-progress/engenc/asura/orchid/count' AND value >= 1)
              OR (track_key = 'oteryn:quest-progress/quest/u10_80/the_lost_brother_quest' AND value >= 3)))",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(event) = event {
        super::super::account_achievement::grant_missing(
            tx,
            character,
            &["oteryn:achievement/lost_palace_raider"],
            "oteryn:lost-palace-raider",
            &event,
        )
        .await?;
    }
    Ok(())
}
