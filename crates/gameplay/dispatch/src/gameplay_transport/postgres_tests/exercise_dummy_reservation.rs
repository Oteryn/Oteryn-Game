//! The bundle-backed exercise fixture cannot cover dispatch-level house dummies yet:
//! `NativeEntryMovementCells::from_bundle` retains house tiles under the entry scope, so
//! `house_at` refuses bundle-scoped cells. This case covers the durable release boundary.
use super::*;
use crate::durability::exercise_training::ExerciseDummyKey;
use crate::foundation::WorldId;

#[test]
fn exercise_dummy_reservation_postgres_one_trainer_release_and_lease_expiry() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "exercise dummy reservation PG panic")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "exercise_dummy_reservation", true, false)
                .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Fresh admission advances the character lease from generation 1 to 2.
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            character_lease_generation: 2,
            ..fence(1)?
        };
        let key = ExerciseDummyKey {
            world_id: WorldId::decode(&id(10)).map_err(debug)?,
            bundle_digest: [7; 32],
            placement_key: 23,
            reset_epoch: 5,
        };

        assert!(
            h.root
                .reserve_exercise_dummy(&authority, &h.node, fence, key)
                .await?
        );
        let first: i64 =
            sqlx::query_scalar("SELECT lease_until_ms FROM game_exercise_dummy_reservations")
                .fetch_one(&h.pool)
                .await?;
        assert!(
            h.root
                .reserve_exercise_dummy(&authority, &h.node, fence, key)
                .await?
        );
        let renewed: i64 =
            sqlx::query_scalar("SELECT lease_until_ms FROM game_exercise_dummy_reservations")
                .fetch_one(&h.pool)
                .await?;
        assert!(renewed >= first);

        sqlx::query(
            "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) \
             SELECT encode($1,'hex')::uuid,account_id,world_id,1,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Dummy Holder' \
             FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid",
        )
        .bind(id(61).as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query(
            "UPDATE game_exercise_dummy_reservations SET character_id=encode($1,'hex')::uuid,game_session_id=encode($2,'hex')::uuid,lease_until_ms=9223372036854775807",
        )
        .bind(id(61).as_slice())
        .bind(id(62).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(
            !h.root
                .reserve_exercise_dummy(&authority, &h.node, fence, key)
                .await?
        );
        let holder: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(character_id) FROM game_exercise_dummy_reservations",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(holder, id(61));

        sqlx::query("UPDATE game_exercise_dummy_reservations SET lease_until_ms=1")
            .execute(&h.pool)
            .await?;
        assert!(
            h.root
                .reserve_exercise_dummy(&authority, &h.node, fence, key)
                .await?
        );
        h.root
            .release_exercise_dummy(&authority, &h.node, fence, key)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_exercise_dummy_reservations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 0);

        sqlx::query(
            "INSERT INTO game_exercise_dummy_reservations(world_id,base_bundle_digest,placement_key,reset_epoch,character_id,game_session_id,lease_until_ms) \
             VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,9223372036854775807)",
        )
        .bind(key.world_id.as_bytes().as_slice())
        .bind(key.bundle_digest.as_slice())
        .bind(key.placement_key.to_string())
        .bind(key.reset_epoch.to_string())
        .bind(id(61).as_slice())
        .bind(id(62).as_slice())
        .execute(&h.pool)
        .await?;
        h.root
            .release_exercise_dummy(&authority, &h.node, fence, key)
            .await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_exercise_dummy_reservations")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(count, 1);

        let reset_key = ExerciseDummyKey {
            reset_epoch: 6,
            ..key
        };
        assert!(
            h.root
                .reserve_exercise_dummy(&authority, &h.node, fence, reset_key)
                .await?
        );
        let stored: (Vec<u8>, String) = sqlx::query_as(
            "SELECT uuid_send(character_id),reset_epoch::text FROM game_exercise_dummy_reservations",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            stored,
            (fence.character_id.as_bytes().to_vec(), "6".to_owned())
        );

        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
