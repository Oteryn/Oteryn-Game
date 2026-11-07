//! Isolated actual production CHECK, not a Quest transition or source death grant.
use super::TestResult;
use super::fixture::{Harness, id, run};

#[test]
fn srx3_rotten_tagged_instance_death_receipt_namespace_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-rotten-namespace", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "srx3_rotten_namespace", false, false)
                    .await?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("CREATE TEMP TABLE srx3_rotten_key_gate (LIKE game_character_rotten_blood_receipts INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP")
            .execute(&mut *tx).await?;
            let mut channel = id(42).to_vec();
            channel.extend_from_slice(&id(43));
            channel.extend_from_slice(&1u64.to_be_bytes());
            channel.extend_from_slice(&1u32.to_be_bytes());
            channel.extend_from_slice(&1u64.to_be_bytes());
            assert_eq!(channel.len(), 52);
            insert(&mut tx, &channel, 1).await?;
            let mut instance = vec![2];
            instance.extend_from_slice(&channel);
            assert!(
                crate::foundation::RuntimeScopeRefV1::instance(
                    crate::foundation::WorldId::decode(&channel[..16])?,
                    channel[16..32].try_into().map_err(|_| "scope DATA")?
                )
                .is_ok()
            );
            if let Err(error) = insert(&mut tx, &instance, 2).await {
                tx.rollback().await?;
                h.cleanup().await?;
                return Err(format!("production Rotten Blood CHECK rejects valid tag2+52-byte Instance DATA: {error}").into());
            }
            sqlx::query("SAVEPOINT wrong_tag").execute(&mut *tx).await?;
            instance[0] = 1;
            let refused = insert(&mut tx, &instance, 3).await;
            assert!(
                refused
                    .as_ref()
                    .err()
                    .and_then(sqlx::Error::as_database_error)
                    .is_some_and(|error| error.code().as_deref() == Some("23514")),
                "53-byte deaths require exact Instance tag2"
            );
            sqlx::query("ROLLBACK TO SAVEPOINT wrong_tag")
                .execute(&mut *tx)
                .await?;
            tx.rollback().await?;
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_ROTTEN_NAMESPACE_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn insert(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    death: &[u8],
    occurrence: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO srx3_rotten_key_gate(character_id,occurrence,request_binding,kind,source_death_key,source_party_minimum,source_dealt_damage,before_state,result,original_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES('00000000-0000-7000-8000-000000000001',$1,$2,1,$3,0,true,'{}','{}',1,1,1,0,0,'p','r','c','s','e','d','q','v')")
        .bind(vec![occurrence;32]).bind(vec![1u8;32]).bind(death).execute(&mut **tx).await?;
    Ok(())
}
