// A journey can lose its socket before logout, or be cancelled by its deadline.
// Fresh admission must observe the owning server's terminal release, including
// control-loss detection and grace; qualification never edits session state.
async fn wait_character_released(clients: &SeamClients<'_>, index: usize) -> TestResult {
    // Existing loss qualification allows 60s for detection and 100s for release.
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_secs(160);
    let mut connection =
        tokio::time::timeout_at(deadline, sqlx::PgConnection::connect(clients.url))
            .await
            .map_err(|_| "character release database connect deadline")??;
    let mut attempts = 0_u64;
    let released = wait_release(deadline, async || {
        attempts += 1;
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
            let wait = (started.elapsed(), attempts);
            report_unreleased_character(clients, index, &mut connection, wait).await;
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
// `wait` is how long the release was awaited and how many reads it took (LOG-17).
async fn report_unreleased_character(
    clients: &SeamClients<'_>,
    index: usize,
    connection: &mut sqlx::PgConnection,
    (waited, attempts): (Duration, u64),
) {
    let waited_ms = waited.as_millis();
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
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=snapshot \
             awaiting={} waited_ms={waited_ms} attempts={attempts} fields={fields}",
            release_awaiting(&fields)
        ),
        Ok(Ok(None)) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=absent \
             waited_ms={waited_ms} attempts={attempts}"
        ),
        Ok(Err(_)) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=unavailable \
             waited_ms={waited_ms} attempts={attempts}"
        ),
        Err(_) => eprintln!(
            "SEAM_EVIDENCE character_release_timeout fixture_index={index} diagnostic=timeout \
             waited_ms={waited_ms} attempts={attempts}"
        ),
    }
}

/// The closed name of what the unreleased session row still waits for (LOG-17): its transport
/// to detach, its grace to expire, or the terminal release after the grace.
fn release_awaiting(fields: &serde_json::Value) -> &'static str {
    match (
        fields.get("transport_present").and_then(serde_json::Value::as_bool),
        fields.get("grace_expired").and_then(serde_json::Value::as_bool),
    ) {
        (Some(true), _) => "transport_detach",
        (Some(false), Some(false)) => "grace_expiry",
        (Some(false), Some(true)) => "terminal_release",
        _ => "unknown",
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
    fn release_timeout_names_what_the_row_awaits() {
        let row = |transport: bool, expired: Option<bool>| {
            serde_json::json!({"state": 1, "transport_present": transport, "grace_expired": expired})
        };
        assert_eq!(release_awaiting(&row(true, Some(false))), "transport_detach");
        assert_eq!(release_awaiting(&row(false, Some(false))), "grace_expiry");
        assert_eq!(release_awaiting(&row(false, Some(true))), "terminal_release");
        assert_eq!(release_awaiting(&row(false, None)), "unknown");
        assert_eq!(release_awaiting(&serde_json::json!(null)), "unknown");
    }

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
