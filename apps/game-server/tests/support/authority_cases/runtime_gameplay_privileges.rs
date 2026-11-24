//! Runtime grant checks for guarded gameplay reads and backpack settlement.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::{
    spell_item_transaction::assert_spell_item_authority_in_transaction,
    spell_owner_commit::SpellLanePermit,
    spell_privacy::{ExivaDecision, read_exiva_decision_in_transaction},
};
use crate::foundation::{CommandId, CommandRef};

#[test]
fn runtime_gameplay_privileges_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "runtime_gameplay_privileges", true)
                        .await?;
                    let result = async {
                        exercise_exiva(&h).await?;
                        exercise_grants(&h).await?;
                        exercise_boss_and_bed_reads(&h).await?;
                        exercise_slot_delete(&h).await
                    }
                    .await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "gameplay privilege test thread failed")??;
    Ok(())
}

async fn exercise_exiva(h: &fixture::Harness) -> fixture::TestResult {
    let character = fixture::id(fixture::CHARACTER);
    sqlx::query(
        "INSERT INTO game_character_spell_access
         (character_id, staff_access, source_policy, source_policy_digest)
         VALUES (encode($1,'hex')::uuid, false, 'fixture', $2)",
    )
    .bind(character.as_slice())
    .bind([7_u8; 32].as_slice())
    .execute(&h.pool)
    .await?;

    let permit = SpellLanePermit::of_fresh_scope(fixture::scope()?).await?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let fence = fixture::item_fence()?;
    let command = CommandRef::new(
        fence.game_session_id,
        CommandId::new(903).map_err(fixture::debug)?,
    );
    let mut reader = h.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut reader,
        &permit,
        &h.root,
        &recovery,
        &h.node,
        &fence,
        command,
        [7; 32],
    )
    .await
    .map_err(fixture::debug)?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *reader)
        .await?;
    assert_eq!(
        read_exiva_decision_in_transaction(&mut reader, &authority, character)
            .await
            .map_err(fixture::debug)?,
        ExivaDecision::UnknownPreferences
    );

    let can_write: bool = sqlx::query_scalar(
        "SELECT has_any_column_privilege(current_user,'game_character_spell_access','INSERT')
             OR has_any_column_privilege(current_user,'game_character_spell_access','UPDATE')
             OR has_table_privilege(current_user,'game_character_spell_access','DELETE')",
    )
    .fetch_one(&mut *reader)
    .await?;
    assert!(!can_write);

    let mut writer = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *writer)
        .await?;
    let blocked = sqlx::query(
        "SELECT 1 FROM game_character_spell_access
         WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT",
    )
    .bind(character.as_slice())
    .fetch_one(&mut *writer)
    .await;
    assert_eq!(
        blocked
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("55P03")
    );
    writer.rollback().await?;
    reader.rollback().await?;
    Ok(())
}

async fn exercise_grants(h: &fixture::Harness) -> fixture::TestResult {
    // SELECT, INSERT, table-wide UPDATE, DELETE.
    for (table, update, runtime, control) in [
        (
            "game_activity_handoff_participants",
            Some("handoff_id"),
            [true, true, false, false],
            [true, false, false, false],
        ),
        (
            "game_activity_runtime_entries",
            Some("handoff_id"),
            [true, true, false, false],
            [false, false, false, false],
        ),
        (
            "game_activity_runtime_placements",
            Some("handoff_id"),
            [true, true, false, false],
            [false, false, false, false],
        ),
        (
            "game_activity_transfer_recoveries",
            Some("recovery_grant_nonce"),
            [true, true, false, false],
            [false, false, false, false],
        ),
        (
            "game_boss_room_catalogue",
            None,
            [true, true, false, false],
            [false, false, false, false],
        ),
        (
            "game_boss_room_publications",
            None,
            [true, true, false, false],
            [true, false, false, false],
        ),
        (
            "game_character_blessings",
            Some("character_id"),
            [true, true, false, true],
            [true, false, false, false],
        ),
        (
            "game_character_death_receipts",
            Some("death_occurrence_id"),
            [true, true, false, false],
            [true, false, false, false],
        ),
        (
            "game_character_pending_respawns",
            Some("character_id"),
            [true, true, false, true],
            [true, false, false, false],
        ),
        (
            "game_character_spell_access",
            None,
            [true, false, false, false],
            [true, true, true, false],
        ),
        (
            "game_character_viola_book_cycles",
            Some("character_id"),
            [true, true, false, false],
            [true, false, false, false],
        ),
        (
            "game_house_bed_sleep_receipts",
            None,
            [false, false, false, false],
            [false, false, false, false],
        ),
        (
            "game_item_container_slots",
            Some("character_id"),
            [true, true, false, false],
            [true, false, false, false],
        ),
        (
            "game_rotten_bak_phase_origins",
            Some("handoff_id"),
            [true, true, false, false],
            [false, false, false, false],
        ),
    ] {
        for (role, expected) in [
            ("oteryn_game_runtime", runtime),
            ("oteryn_game_control", control),
        ] {
            let grants: Vec<bool> = sqlx::query_scalar(
                "SELECT has_table_privilege($1,$2,privilege)
                 FROM unnest(ARRAY['SELECT','INSERT','UPDATE','DELETE'])
                 WITH ORDINALITY AS p(privilege,ordinal) ORDER BY ordinal",
            )
            .bind(role)
            .bind(table)
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(grants, expected, "{table}: {role}");
        }
        let runtime_update: Vec<String> = sqlx::query_scalar(
            "SELECT attname::text FROM pg_attribute
             WHERE attrelid=$1::regclass AND attnum>0 AND NOT attisdropped
             AND has_column_privilege('oteryn_game_runtime',attrelid,attnum,'UPDATE')
             ORDER BY attnum",
        )
        .bind(table)
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            runtime_update.first().map(String::as_str),
            update,
            "{table}"
        );
        assert_eq!(
            runtime_update.len(),
            usize::from(update.is_some()),
            "{table}"
        );
    }
    for signature in [
        "game_spell_access_read_locked(uuid,uuid)",
        "game_boss_room_catalogue_read_locked(text,boolean)",
        "game_boss_room_publication_read_locked(uuid,text,text,bytea,bytea,bytea)",
        "game_house_bed_sleep_receipted(uuid,numeric)",
        "game_player_death_remove_backpack_slot(uuid,uuid)",
    ] {
        let executes: (bool, bool, bool) = sqlx::query_as(
            "SELECT has_function_privilege('oteryn_game_runtime',p.oid,'EXECUTE'),
                    has_function_privilege('oteryn_game_control',p.oid,'EXECUTE'),
                    EXISTS(SELECT 1 FROM aclexplode(p.proacl) a
                           WHERE a.grantee=0 AND a.privilege_type='EXECUTE')
             FROM pg_proc p WHERE p.oid=$1::regprocedure",
        )
        .bind(signature)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(executes, (true, false, false), "{signature}");
    }
    Ok(())
}

async fn exercise_boss_and_bed_reads(h: &fixture::Harness) -> fixture::TestResult {
    sqlx::query(
        "INSERT INTO game_boss_room_catalogue
         (boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,
          minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,
          room_template_key,reward_key,enabled)
         VALUES('oteryn:boss.acl-fixture','fixture','oteryn:creature.fixture',1,1,1,1,1,
                '[]','fixture','fixture',true)",
    )
    .execute(&h.pool)
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let revision: String = sqlx::query_scalar(
        "SELECT content_revision FROM game_boss_room_catalogue_read_locked('oteryn:boss.acl-fixture',true)",
    )
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(revision, "fixture");
    let published: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_boss_room_publication_read_locked(
           encode($1,'hex')::uuid,'oteryn:boss.acl-fixture','fixture',$2,$2,$2))",
    )
    .bind(fixture::id(fixture::WORLD).as_slice())
    .bind([1_u8; 32].as_slice())
    .fetch_one(&mut *tx)
    .await?;
    assert!(!published);
    let replayed: bool =
        sqlx::query_scalar("SELECT game_house_bed_sleep_receipted(encode($1,'hex')::uuid,1)")
            .bind(fixture::id(fixture::SESSION).as_slice())
            .fetch_one(&mut *tx)
            .await?;
    assert!(!replayed);
    tx.rollback().await?;
    Ok(())
}

async fn exercise_slot_delete(h: &fixture::Harness) -> fixture::TestResult {
    let backpack = fixture::id(71);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_instances
         (item_instance_id,world_id,definition_family,definition_production_key,
          definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i2854',
                'rev-1',1,1,encode($3,'hex')::uuid)",
    )
    .bind(backpack.as_slice())
    .bind(fixture::id(fixture::WORLD).as_slice())
    .bind(fixture::id(72).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)",
    )
    .bind(fixture::id(fixture::CHARACTER).as_slice())
    .bind(backpack.as_slice())
    .bind(fixture::id(fixture::WORLD).as_slice())
    .bind(fixture::id(73).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let refused = sqlx::query(
        "SELECT game_player_death_remove_backpack_slot(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
    )
    .bind(backpack.as_slice())
    .bind(fixture::id(74).as_slice())
    .execute(&mut *tx)
    .await;
    assert_eq!(
        refused
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("23514"),
        "unproven death-slot deletion must reach the installed proof guard: {refused:?}"
    );
    tx.rollback().await?;
    Ok(())
}
