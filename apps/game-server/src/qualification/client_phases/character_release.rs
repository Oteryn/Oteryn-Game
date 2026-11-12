// A journey can lose its socket before logout, or be cancelled by its deadline.
// Fresh admission must observe the owning server's terminal release, including
// control-loss detection and grace; qualification never edits session state.
// The first timeout per fixture is the root; later waits on it fail at once.
async fn wait_character_released(clients: &SeamClients<'_>, index: usize) -> TestResult {
    let harness = verdict::Harness::global();
    harness.fixture_gate(index)?;
    wait_fixture_released(clients, index)
        .await
        .map_err(|error| harness.fixture_failed(index, error.as_ref()))
}

async fn wait_fixture_released(clients: &SeamClients<'_>, index: usize) -> TestResult {
    // Existing loss qualification allows 60s for detection and 100s for release.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(160);
    verdict::Harness::global().bounded_wait(deadline, "fixture-release");
    let mut connection =
        tokio::time::timeout_at(deadline, sqlx::PgConnection::connect(clients.url))
            .await
            .map_err(|_| "character release database connect deadline")??;
    let released = wait_release(deadline, async || {
        Ok(!sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
             WHERE world_id=encode($1,'hex')::uuid AND session_state<>3 \
             AND (account_id=$2::text::uuid OR character_id=encode($3,'hex')::uuid))",
        )
        .bind(clients.world.as_slice())
        .bind(&clients.accounts[index])
        .bind(clients.characters[index].as_slice())
        .fetch_one(&mut connection)
        .await?)
    })
    .await;
    if let Err(error) = released {
        if tokio::time::Instant::now() >= deadline {
            report_unreleased_character(clients, index, &mut connection).await;
        }
        return Err(error);
    }
    tokio::time::timeout_at(deadline, connection.close())
        .await
        .map_err(|_| "character release database close deadline")??;
    Ok(())
}

// This bounded read runs only after the release wait has already failed. Its
// outcome is diagnostic: even a now-absent session cannot make the wait pass.
async fn report_unreleased_character(
    clients: &SeamClients<'_>,
    index: usize,
    connection: &mut sqlx::PgConnection,
) {
    let snapshot = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT jsonb_build_object( \
             'state',session_state, \
             'transport_present',current_transport_ref IS NOT NULL, \
             'control_loss_epoch',control_loss_epoch, \
             'grace_deadline',original_grace_deadline, \
             'grace_expired',original_grace_deadline<=floor(extract(epoch FROM clock_timestamp()))::bigint, \
             'connection_generation',current_generation, \
             'lease_generation',character_lease_generation, \
             'scope_generation',scope_ownership_generation) \
             FROM game_durability_reconnect_sessions \
             WHERE world_id=encode($1,'hex')::uuid AND session_state<>3 \
             AND (account_id=$2::text::uuid OR character_id=encode($3,'hex')::uuid) \
             ORDER BY (character_id=encode($3,'hex')::uuid) DESC,game_session_id LIMIT 1",
        )
        .bind(clients.world.as_slice())
        .bind(&clients.accounts[index])
        .bind(clients.characters[index].as_slice())
        .fetch_optional(connection),
    )
    .await;
    match snapshot {
        Ok(Ok(Some(fields))) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=snapshot fields={fields}"
        ),
        Ok(Ok(None)) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=absent"
        ),
        Ok(Err(_)) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=unavailable"
        ),
        Err(_) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=timeout"
        ),
    }
}

async fn wait_release(
    deadline: tokio::time::Instant,
    mut released: impl AsyncFnMut() -> TestResult<bool>,
) -> TestResult {
    tokio::time::timeout_at(deadline, async {
        loop {
            if released().await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| "previous journey character was not terminally released after loss/grace")?
}

#[cfg(test)]
mod character_release_tests {
    use super::*;

    #[test]
    fn character_release_waits_through_active_loss_and_grace() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let started = tokio::time::Instant::now();
                let mut states = [2, 1, 1, 3].into_iter();
                wait_release(started + Duration::from_secs(5), async || {
                    Ok(states.next() == Some(3))
                })
                .await?;
                assert!(started.elapsed() >= Duration::from_millis(300));
                assert_eq!(states.next(), None);
                Ok(())
            })
    }

    #[test]
    fn character_release_never_qualifies_a_live_or_unreadable_session() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let deadline = tokio::time::Instant::now() + Duration::from_millis(10);
                assert!(wait_release(deadline, async || Ok(false)).await.is_err());
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                assert!(
                    wait_release(deadline, async || Err("session read failed".into()))
                        .await
                        .is_err()
                );
                Ok(())
            })
    }
}
