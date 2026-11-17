//! A competing earned source commits while a real USE waits on the account grant lock.
use super::*;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: AdmittedSession,
    payload: &[u8],
    source: NonZeroU32,
) -> TestResult {
    let mut competing = h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:appearance-account:' || encode($1,'hex')::uuid::text,0))")
        .bind(id(40).as_slice()).execute(&mut *competing).await?;
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *competing)
        .await?;
    let publish = async {
        // Wait for the real item writer to reach the shared lock. A timing-only sleep
        // could pass without exercising the cross-source read-after-lock guarantee.
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks l WHERE l.locktype='advisory' AND NOT l.granted AND $1=ANY(pg_blocking_pids(l.pid)) AND l.database=(SELECT oid FROM pg_database WHERE datname=current_database()))")
                    .bind(blocker).fetch_one(&h.pool).await?;
                if waiting { return Ok::<_, sqlx::Error>(()); }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await??;
        // The competitor is an explicit fixture for an independently sourced earned
        // fact, using the ordinary immutable appearance rows and normal DB guards.
        // This is not a new production grant path or a canonical content decision.
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts(grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,encode($5,'hex')::uuid,'npc',$6)")
            .bind(id(91).as_slice()).bind(id(40).as_slice()).bind(packet::COMPETING_GRANT)
            .bind(packet::COMPETING_PROVENANCE).bind(id(41).as_slice()).bind(id(92).as_slice())
            .execute(&mut *competing).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
            .bind(id(40).as_slice()).bind(packet::COMPETING_GRANT).bind(id(91).as_slice())
            .execute(&mut *competing).await?;
        competing.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    use std::future::Future;
    use std::task::Poll;
    let mut using = std::pin::pin!(super::super::super::rust_lottery::exchange(
        owner,
        admitted,
        4,
        payload,
        Some(source),
        None
    ));
    let mut publishing = std::pin::pin!(publish);
    let (mut used, mut published) = (None, None);
    std::future::poll_fn(|context| {
        if used.is_none()
            && let Poll::Ready(result) = using.as_mut().poll(context)
        {
            used = Some(result);
        }
        if published.is_none()
            && let Poll::Ready(result) = publishing.as_mut().poll(context)
        {
            published = Some(result);
        }
        if used.is_some() && published.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    published.ok_or("competing grant did not finish")??;
    let (end, result, _) = used.ok_or("item use did not finish")??;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        fiery_remaining(h).await?,
        2,
        "the newly owned target must preserve the fourth item"
    );
    assert_eq!(scalar(h, "SELECT contributions FROM game_character_item_cosmetic_progress WHERE component_key='fiery-horseshoe'").await?, 3);
    assert_eq!(grant_count(h).await?, 0);
    assert_eq!(
        receipt_count(h).await?,
        4,
        "the no-spend result remains replayable"
    );
    assert!(
        appearance(owner, &end)
            .await?
            .catalogue
            .mounts
            .iter()
            .any(|key| key == packet::MOUNT)
    );
    eprintln!("W19 POSTGRES_CASE_COMPLETED item_cosmetics competing_source_account_lock_no_spend");
    Ok(())
}
