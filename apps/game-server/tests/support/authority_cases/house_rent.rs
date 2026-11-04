//! S05 PostgreSQL cases drive the production rent port and the existing disposition mover.
use crate::item_transfer_postgres_cases::{
    Harness, TestResult, WORLD, configured_admin, debug, id, runtime, scope,
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
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,ownership_generation,revision,acl_revision,acquisition_receipt_ref,paid_until) VALUES($1::uuid,$2,'private_house',3,100,decode(repeat('01',32),'hex'),$3::uuid,$4::uuid,1,1,1,game_character_uuid_v7(),statement_timestamp())")
            .bind(&world).bind(HOUSE).bind(&character).bind(&account).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT $1::uuid,$2,$3::uuid,1,acquisition_receipt_ref FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&world).bind(HOUSE).bind(&character).execute(&mut *tx).await?;
        // The production census updates sleepers even when this house has no beds.
        sqlx::query("INSERT INTO game_house_bed_revisions(world_id,house_key,revision) VALUES($1::uuid,$2,1)")
            .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        // game_house_property_proven (0143/0228) checks every UPDATE of this house at commit:
        // the canonical owner row carries the property's receipt and the account holds its slot.
        sqlx::query("INSERT INTO game_account_housing_slots(account_id,world_id,house_key) VALUES($1::uuid,$2::uuid,$3)")
            .bind(&account).bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_rent_state VALUES($1::uuid,$2,0,statement_timestamp()-interval '1 day')")
            .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;

        // paid_until is the donor's collection gate: a prepaid house is not charged early.
        sqlx::query("UPDATE game_house_properties SET paid_until=statement_timestamp()+interval '1 day',revision=revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&h.pool).await?;
        assert!(!h.root.advance_house_rent(&authority, &h.node, scope()?, 1).await?);
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,0);
        sqlx::query("UPDATE game_house_properties SET paid_until=statement_timestamp()-interval '1 day',revision=revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&h.pool).await?;

        // A stale execution generation cannot collect rent or issue a warning.
        assert!(h.root.advance_house_rent(&authority, &h.node, scope()?, 2).await.is_err());
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,0);

        // No balance: one durable Inbox letter, exactly as payHouses warns its owner.
        assert!(h.root.advance_house_rent(&authority, &h.node, scope()?, 1).await?);
        let warning: i16 = sqlx::query_scalar("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(warning, 1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters r JOIN game_item_character_inbox_locations i ON i.item_instance_id=r.letter_id AND i.world_id=r.world_id AND i.character_id=r.owner_character_id WHERE r.house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,1,"the warning letter is a real owner Inbox item");

        // Install a valid preceding rent ledger/balance, then prove collection debits it.
        let mut tx=h.pool.begin().await?;tx.execute("SET LOCAL session_replication_role=replica").await?;
        let entry:String=sqlx::query_scalar("WITH ids AS (SELECT game_character_uuid_v7() payment,game_character_uuid_v7() entry) INSERT INTO game_house_rent_payments SELECT payment,$1::uuid,$2,$3::uuid,$4::uuid,1,entry,statement_timestamp() FROM ids RETURNING bank_entry_id::text").bind(&world).bind(HOUSE).bind(&account).bind(&character).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,house_rent_payment_id,house_rent_kind,account_id,world_id,amount,balance_before,balance_after,acting_character_id) SELECT $1::uuid,payment_id,'HOUSE_RENT',$2::uuid,$3::uuid,1,101,100,$4::uuid FROM game_house_rent_payments WHERE bank_entry_id=$1::uuid").bind(&entry).bind(&account).bind(&world).bind(&character).execute(&mut *tx).await?;
        // The fixture balance is installed with triggers disabled; the production debit below runs every guard.
        sqlx::query("INSERT INTO game_account_bank_balances VALUES($1::uuid,$2::uuid,100,$3::uuid)").bind(&account).bind(&world).bind(entry).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '1 day' WHERE house_key=$1").bind(HOUSE).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT balance FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(&account).bind(&world).fetch_one(&h.pool).await?,0);

        // Seven failures send letters; the eighth creates EVICTION and its mover sends items to Inbox.
        let mut tx=h.pool.begin().await?;tx.execute("SET LOCAL session_replication_role=replica").await?;
        let item:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),$1::uuid,'Item','fixture:rent.item','r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text").bind(&world).fetch_one(&mut *tx).await?;
        let placement:String=sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES($1::uuid,$2::uuid,$3,'\\x01',1,$4::uuid)").bind(&item).bind(&world).bind(HOUSE).bind(&placement).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES($1::uuid,$2::uuid,$3,$4::uuid,$5::uuid,1)").bind(&item).bind(&world).bind(HOUSE).bind(&character).bind(&placement).execute(&mut *tx).await?;
        tx.commit().await?;
        for expected in 1..=7_i16 {
            sqlx::query("UPDATE game_house_properties SET paid_until=statement_timestamp()-interval '1 day',revision=revision+1 WHERE house_key=$1").bind(HOUSE).execute(&h.pool).await?;
            sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '1 day' WHERE house_key=$1").bind(HOUSE).execute(&h.pool).await?;
            assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?);
            assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,expected);
        }
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1 AND warning=1").bind(HOUSE).fetch_one(&h.pool).await?,2,"warning ordinals may repeat in distinct rent cycles");
        let letters_at_seven=sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?,0,"seven warning cycles do not evict");
        sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '2 days' WHERE house_key=$1").bind(HOUSE).execute(&h.pool).await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?);
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,7,"the terminal warning never violates the 0..7 check");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,letters_at_seven);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_disposition_audit_outbox a JOIN game_house_dispositions d USING(operation_id) WHERE d.house_key=$1 AND d.cause='EVICTION' AND a.written_xact_id=d.written_xact_id").bind(HOUSE).fetch_one(&h.pool).await?,1,"eviction creation carries its own physical audit event");
        sqlx::query("UPDATE game_house_rent_state SET next_rent_at=statement_timestamp()-interval '2 days' WHERE house_key=$1").bind(HOUSE).execute(&h.pool).await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?,1,"repeated terminal cycles never duplicate eviction");
        let disposition_grants: (bool,bool,bool) = sqlx::query_as("SELECT has_function_privilege('public','game_house_disposition_is_personal_move_out(uuid)','EXECUTE'),has_function_privilege('oteryn_game_runtime','game_house_disposition_is_personal_move_out(uuid)','EXECUTE'),has_function_privilege('oteryn_game_control','game_house_disposition_is_personal_move_out(uuid)','EXECUTE')").fetch_one(&h.pool).await?;
        assert_eq!(disposition_grants,(false,true,true),"the disposition classifier is private to the two game roles");
        let cause:String=sqlx::query_scalar("SELECT cause FROM game_house_dispositions WHERE house_key=$1 AND state=1").bind(HOUSE).fetch_one(&h.pool).await?;
        assert_eq!(cause,"EVICTION");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_acl_entries WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,0);

        let operation:String=sqlx::query_scalar("SELECT operation_id::text FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?;
        let permit=crate::durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(scope()?).await.map_err(debug)?;
        h.root.drain_house_dispositions(&permit,&authority,&h.node,scope()?,1).await.map_err(debug)?;
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT state FROM game_house_dispositions WHERE operation_id=$1::uuid").bind(&operation).fetch_one(&h.pool).await?,2,"the first pass freezes the census without delivery");
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_disposition_steps WHERE operation_id=$1::uuid").bind(&operation).fetch_one(&h.pool).await?,0);
        h.root.drain_house_dispositions(&permit,&authority,&h.node,scope()?,1).await.map_err(debug)?;
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT state FROM game_house_dispositions WHERE operation_id=$1::uuid").bind(&operation).fetch_one(&h.pool).await?,3);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_disposition_steps WHERE operation_id=$1::uuid AND step_number=1").bind(&operation).fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=$1::uuid AND character_id=$2::uuid").bind(&item).bind(&character).fetch_one(&h.pool).await?,1);

        // A genuine live-index collision on the terminal house is local: the next house still advances.
        let collision="oteryn:content.house.rent_test_collision";
        let other="oteryn:content.house.rent_test_z";
        let mut tx=h.pool.begin().await?;tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,ownership_generation,revision,acl_revision,acquisition_receipt_ref,paid_until) VALUES($1::uuid,$2,'private_house',3,100,decode(repeat('03',32),'hex'),$3::uuid,$4::uuid,1,1,1,game_character_uuid_v7(),statement_timestamp()-interval '1 day')")
            .bind(&world).bind(collision).bind(&character).bind(&account).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES($1::uuid,$2,$3::uuid,1,game_character_uuid_v7())")
            .bind(&world).bind(collision).bind(&character).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_rent_state VALUES($1::uuid,$2,6,statement_timestamp()-interval '2 days')")
            .bind(&world).bind(collision).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at,cause) VALUES($1::uuid,$2,$3::uuid,1,1,0,86400000,'MOVE_OUT')")
            .bind(&world).bind(collision).bind(&character).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,ownership_generation,revision,acl_revision,acquisition_receipt_ref,paid_until) VALUES($1::uuid,$2,'private_house',3,100,decode(repeat('02',32),'hex'),$3::uuid,$4::uuid,1,1,1,game_character_uuid_v7(),statement_timestamp()-interval '1 day')")
            .bind(&world).bind(other).bind(&character).bind(&account).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) VALUES($1::uuid,$2,$3::uuid,1,game_character_uuid_v7())")
            .bind(&world).bind(other).bind(&character).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_rent_state VALUES($1::uuid,$2,0,statement_timestamp()-interval '1 day')")
            .bind(&world).bind(other).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?,"the colliding disposition is isolated per house");
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(collision).fetch_one(&h.pool).await?,7);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_dispositions WHERE house_key=$1").bind(collision).fetch_one(&h.pool).await?,1,"the live MOVE_OUT really collided with the attempted eviction");
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?, 1).await?,"the world pass continues with the next due house");
        assert_eq!(sqlx::query_scalar::<_,i16>("SELECT warnings FROM game_house_rent_state WHERE house_key=$1").bind(other).fetch_one(&h.pool).await?,1);

        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn house_rent_inactivity_loss_after_30_days() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "house_rent_inactivity").await?;
        let world = hex::encode(id(WORLD));
        let character = hex::encode(id(41));
        let account: String = sqlx::query_scalar(
            "SELECT account_id::text FROM game_character_roots WHERE character_id=$1::uuid",
        )
        .bind(&character)
        .fetch_one(&h.pool)
        .await?;
        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,owner_character_id,owner_account_id,ownership_generation,revision,acl_revision,acquisition_receipt_ref,paid_until) VALUES($1::uuid,$2,'private_house',3,100,decode(repeat('04',32),'hex'),$3::uuid,$4::uuid,1,1,1,game_character_uuid_v7(),statement_timestamp()+interval '90 days')")
            .bind(&world).bind(HOUSE).bind(&character).bind(&account).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref) SELECT world_id,house_key,owner_character_id,1,acquisition_receipt_ref FROM game_house_properties WHERE world_id=$1::uuid AND house_key=$2")
            .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_housing_slots(account_id,world_id,house_key) VALUES($1::uuid,$2::uuid,$3)")
            .bind(&account).bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=floor(extract(epoch FROM statement_timestamp()-interval '29 days')*1000000)::bigint WHERE character_id=$1::uuid")
            .bind(&character).execute(&mut *tx).await?;
        tx.commit().await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;

        assert!(!h.root.advance_house_rent(&authority,&h.node,scope()?,1).await?);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?,0);

        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("UPDATE game_house_rent_config SET lose_after_inactivity_days=0").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET training_created_at_us=floor(extract(epoch FROM statement_timestamp()-interval '400 days')*1000000)::bigint WHERE character_id=$1::uuid")
            .bind(&character).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(!h.root.advance_house_rent(&authority,&h.node,scope()?,1).await?);

        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("UPDATE game_house_rent_config SET lose_after_inactivity_days=30").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid")
            .bind(&character).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(!h.root.advance_house_rent(&authority,&h.node,scope()?,1).await?);

        let mut tx = h.pool.begin().await?;
        tx.execute("SET LOCAL session_replication_role=replica").await?;
        sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state,training_created_at_us) VALUES(game_character_uuid_v7(),$1::uuid,$2::uuid,$3::uuid,1,$3::uuid,game_character_uuid_v7(),1,999999,1,1,1,1,decode(repeat('09',16),'hex'),3,floor(extract(epoch FROM statement_timestamp()-interval '31 days')*1000000)::bigint)")
            .bind(&account).bind(&character).bind(&world).execute(&mut *tx).await?;
        tx.commit().await?;
        assert!(h.root.advance_house_rent(&authority,&h.node,scope()?,1).await?);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_dispositions WHERE house_key=$1 AND cause='EVICTION'").bind(HOUSE).fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_house_rent_inbox_letters WHERE house_key=$1").bind(HOUSE).fetch_one(&h.pool).await?,0);

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
