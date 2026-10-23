use super::*;
use oteryn_gameplay_housing::house_auction::Bid;
use oteryn_gameplay_housing::house_auction::eligibility::Eligibility;

pub(in crate::durability) struct Candidate {
    pub bid: Bid,
    pub character: [u8; 16],
    pub facts: Eligibility,
}

pub(super) async fn read(
    tx: &mut Tx<'_>,
    character: &[u8; 16],
    world: &[u8; 16],
    bid: Bid,
    premium_allowed: bool,
) -> Result<Candidate> {
    let account = bid.account;
    let row=sqlx::query("SELECT r.lifecycle=1 AS active,r.account_id=encode($2,'hex')::uuid AS account_matches,r.world_id=encode($3,'hex')::uuid AS world_matches,coalesce(s.level,0) AS level,EXISTS(SELECT 1 FROM game_account_housing_slots WHERE account_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid) AS slot_occupied,EXISTS(SELECT 1 FROM game_account_house_bans WHERE account_id=encode($2,'hex')::uuid AND world_id=encode($3,'hex')::uuid AND banned_until>clock_timestamp()) AS banned FROM game_character_roots r LEFT JOIN game_character_progression_state s USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(character.as_slice()).bind(account.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
    Ok(Candidate {
        bid,
        character: *character,
        facts: Eligibility {
            active: row.try_get("active")?,
            account_matches: row.try_get("account_matches")?,
            world_matches: row.try_get("world_matches")?,
            level: u32::try_from(row.try_get::<i64, _>("level")?)
                .map_err(|_| HouseOwnershipError::InvalidInput)?,
            junior: super::super::bank::fee_payer_is_junior(
                crate::domain::CharacterId::from_bytes(*character)
                    .map_err(|_| HouseOwnershipError::InvalidInput)?,
            ),
            slot_occupied: row.try_get("slot_occupied")?,
            banned: row.try_get("banned")?,
            premium_allowed,
        },
    })
}
