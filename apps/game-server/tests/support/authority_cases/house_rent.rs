//! S05 PostgreSQL cases drive the production rent port and the existing disposition mover.
use crate::item_transfer_postgres_cases::{
    Harness, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use sqlx::Executor;

const HOUSE: &str = "oteryn:content.house.rent_test";

#[test]
fn rent_payment_warning_and_seventh_warning_eviction_moves_items_to_inbox() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "house_rent").await?;
        let world = hex::encode(id(WORLD));
        let character = hex::encode(id(41));
        let account: String = sqlx::query_scalar(
            "SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,ownership_generation,revision,acl_revision,acquisition_receipt_ref,paid_until) VALUES(encode($1,'hex')::uuid,$2,'private_house',3,100,decode(repeat('01',32),'hex'),encode($3,'hex')::uuid,$4::uuid,1,1,1,game_character_uuid_v7(),statement_timestamp())")
            .bind(&world).bind(HOUSE).bind(&character).bind(&account).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,game_character_uuid_v7())")
            .bind(&world).bind(HOUSE).bind(&character).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_rent_state VALUES(encode($1,'hex')::uuid,$2,0,statement_timestamp()-interval '1 day')")
            .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;

        // No balance: one durable Inbox letter, exactly as payHouses warns its owner.
        assert!(h.root.advance_house_rent(&authority, &h.node, crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?).await?);
        let warning: i16 = sqlx::query_scalar("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(warning, 1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,1);

        // Install a valid preceding rent ledger/balance, then prove collection debits it.
        let mut tx=h.pool.begin().await?;tx.execute("SET LOCAL session_replication_role=replica").await?;
        let entry:String=sqlx::query_scalar("WITH ids AS (SELECT game_character_uuid_v7() payment,game_character_uuid_v7() entry) INSERT INTO game_house_rent_payments SELECT payment,encode($1,'hex')::uuid,$2,$3::uuid,encode($4,'hex')::uuid,1,entry,statement_timestamp() FROM ids RETURNING bank_entry_id::text").bind(&world).bind(HOUSE).bind(&account).bind(&character).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,house_rent_payment_id,house_rent_kind,account_id,world_id,amount,balance_before,balance_after,acting_character_id) SELECT $1::uuid,payment_id,'HOUSE_RENT',$2::uuid,encode($3,'hex')::uuid,1,101,100,encode($4,'hex')::uuid FROM game_house_rent_payments WHERE bank_entry_id=$1::uuid").bind(&entry).bind(&account).bind(&world).bind(&character).execute(&mut *tx).await?;
        // The fixture balance is installed with triggers disabled; the production debit below runs every guard.
        sqlx::query("INSERT INTO game_account_bank_balances VALUES($1::uuid,encode($2,'hex')::uuid,100,$3::uuid)").bind(&account).bind(&world).bind(entry).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '1 day' WHERE house_key=$1").bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?).await?);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid").bind(&account).bind(&world).fetch_one(&h.pool).await?,0);

        // Seven failures create EVICTION; its established production mover sends house items to Inbox.
        let mut tx=h.pool.begin().await?;tx.execute("SET LOCAL session_replication_role=replica").await?;
        let item:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','fixture:rent.item','r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text").bind(&world).fetch_one(&mut *tx).await?;
        let placement:String=sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES($1::uuid,encode($2,'hex')::uuid,$3,'\\x01',1,$4::uuid)").bind(&item).bind(&world).bind(HOUSE).bind(&placement).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES($1::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5::uuid,1)").bind(&item).bind(&world).bind(HOUSE).bind(&character).bind(&placement).execute(&mut *tx).await?;
        tx.commit().await?;
        for expected in 1..=7_i16 {
            sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '1 day' WHERE house_key=$1").bind(HOUSE).execute(&h.pool).await?;
            assert!(h.root.advance_house_rent(&authority,&h.node,crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?).await?);
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,expected);
        }
        let cause:String=sqlx::query_scalar("SELECT cause FROM game_house_dispositions WHERE house_key=$1 AND state=1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(cause,"EVICTION");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_acl_entries WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,0);
        let operation:String=sqlx::query_scalar("SELECT operation_id::text FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(sqlx::query_scalar::<_,i32>("SELECT game_house_disposition_advance($1::uuid,2000,100,0)").bind(&operation).fetch_one(&h.pool).await?,-1);
        assert_eq!(sqlx::query_scalar::<_,i32>("SELECT game_house_disposition_advance($1::uuid,2000,100,1)").bind(&operation).fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=$1::uuid").bind(&item).fetch_one(&h.pool).await?,1);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

mod hex {
    pub fn encode(bytes: [u8; 16]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
