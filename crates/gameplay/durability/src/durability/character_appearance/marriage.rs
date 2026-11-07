//! NPC-MARRIAGE-1 borrows the standing account appearance grant owner.
use crate::durability::{
    DurabilityError,
    npc_marriage::{GRANT_KEY, PROVENANCE},
};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, types::Uuid};

pub(crate) async fn grant_marriage_outfits(
    connection: &mut PgConnection,
    characters: &[Uuid],
    occurrence: [u8; 16],
) -> Result<(), DurabilityError> {
    let accounts:Vec<Uuid>=sqlx::query_scalar("SELECT DISTINCT account_id FROM game_character_roots WHERE character_id=ANY($1) ORDER BY account_id")
        .bind(characters).fetch_all(&mut *connection).await?;
    // Pair roots are already locked. Sort distinct accounts before borrowing their grant locks.
    for account in accounts {
        super::lock_account_grants(connection, account).await?;
        if sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE account_id=$1 AND unlock_key=$2)")
            .bind(account).bind(GRANT_KEY).fetch_one(&mut *connection).await? {continue;}
        let character:Uuid=sqlx::query_scalar("SELECT character_id FROM game_character_roots WHERE character_id=ANY($1) AND account_id=$2 ORDER BY character_id LIMIT 1")
            .bind(characters).bind(account).fetch_one(&mut *connection).await?;
        let event = [
            b"oteryn:npc-marriage-outfit:v1\0".as_slice(),
            occurrence.as_slice(),
            account.as_bytes(),
        ]
        .concat();
        let digest = Sha256::digest(&event);
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 15) | 0x80;
        id[8] = (id[8] & 63) | 0x80;
        let id = Uuid::from_bytes(id);
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES($1,$2,$3,$4,$5,'npc',$6)")
            .bind(id).bind(account).bind(GRANT_KEY).bind(PROVENANCE).bind(character).bind(event).execute(&mut *connection).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
            .bind(account).bind(GRANT_KEY).bind(id).execute(&mut *connection).await?;
    }
    Ok(())
}
