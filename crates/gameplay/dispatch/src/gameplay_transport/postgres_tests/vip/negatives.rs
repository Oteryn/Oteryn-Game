use super::*;
use crate::foundation::{ConnectionGeneration, ScopeOwnershipGeneration};
use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) async fn fences(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_command_receipts")
        .fetch_one(pool)
        .await?;
    for mutation in 0..8 {
        let mut candidate = admitted;
        let fence = candidate.item_fence.as_mut().ok_or("fence")?;
        match mutation {
            0 => {
                fence.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
                candidate.continuity.connection_generation = 2;
            }
            1 => fence.character_lease_generation += 1,
            2 => {
                fence.scope_ownership_generation =
                    ScopeOwnershipGeneration::new(2).map_err(debug)?
            }
            3 => candidate.world_id = crate::foundation::WorldId::decode(&id(99)).map_err(debug)?,
            4 => {
                candidate.channel_id =
                    crate::foundation::ChannelId::decode(&id(99)).map_err(debug)?
            }
            5 => fence.character_id = CharacterId::from_bytes(id(99)).map_err(debug)?,
            6 => {
                candidate.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?
            }
            _ => candidate.runtime_actor = None,
        }
        let result = owner
            .vip_command(candidate, 600 + mutation, 48, &named(1, "Alice"))
            .await
            .ok_or("typed fence refusal")?;
        assert_eq!(code(&result)?, 10, "fence mutation {mutation}");
    }
    // Change one durable recovery identity field, leaving the rest valid. Even a retained
    // successful receipt cannot be replayed by a stale external recovery capability.
    let issuer:String=sqlx::query_scalar("SELECT issuer_identity FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1").fetch_one(pool).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_recovery_admissions SET issuer_identity='different-current-issuer'",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    assert!(
        owner
            .vip_command(admitted, 1, 48, &named(1, "Alice"))
            .await
            .is_none()
    );
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_recovery_admissions SET issuer_identity=$1")
        .bind(issuer)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_command_receipts")
        .fetch_one(pool)
        .await?;
    assert_eq!(before, after);
    // The installed runtime privilege boundary admits the narrow operation functions, not DML.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    for table in [
        "game_vip_lists",
        "game_vip_entries",
        "game_vip_groups",
        "game_vip_group_members",
        "game_vip_consents",
        "game_vip_handles",
        "game_vip_command_receipts",
    ] {
        let allowed: bool = sqlx::query_scalar(
            "SELECT has_table_privilege(current_user,$1,'INSERT,UPDATE,DELETE,TRUNCATE')",
        )
        .bind(table)
        .fetch_one(&mut *tx)
        .await?;
        assert!(!allowed, "runtime DML {table}");
    }
    for column in ["vip_status", "vip_consent_revision"] {
        let allowed: bool = sqlx::query_scalar("SELECT has_column_privilege(current_user,'game_character_social_settings',$1,'INSERT,UPDATE')")
            .bind(column).fetch_one(&mut *tx).await?;
        assert!(!allowed, "direct VIP setting write {column}");
    }
    sqlx::query("UPDATE game_character_social_settings SET party_invites=party_invites WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    let allowed:bool=sqlx::query_scalar("SELECT has_function_privilege(current_user,'game_vip_add(jsonb,jsonb,numeric,jsonb,integer,integer)','EXECUTE')").fetch_one(&mut *tx).await?;
    assert!(allowed);
    assert!(
        sqlx::query("UPDATE game_vip_entries SET icon=1")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.rollback().await?;
    Ok(())
}
